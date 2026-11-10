import { defineStore } from "pinia";
import { invoke } from "@tauri-apps/api/core";
import type {
  CookieMatchResult,
  CookieProfileInput,
  CookieProfileSummary,
  CookieSelection,
  CookieUsage,
} from "@/types";

/** 未使用任何 cookie 的空快照 */
const EMPTY_USAGE: CookieUsage = {
  mode: "none",
  profileId: null,
  profileName: null,
  matchedDomain: null,
  cookieFile: null,
};

/**
 * Cookie 配置 Store
 *
 * SQLite 是配置的唯一事实源，这里只做会话内缓存：
 * - `matchCache` 缓存「链接 → 命中 profile」，避免同一次解析流程重复请求后端
 * - `cliOverrideFile` 保存命令行 `--cookies` 传入的一次性文件路径（不落盘、不持久化）
 */
export const useCookieStore = defineStore("cookie", () => {
  const profiles = ref<CookieProfileSummary[]>([]);
  const loaded = ref(false);
  const matchCache = new Map<string, CookieMatchResult | null>();
  const cliOverrideFile = ref<string | null>(null);

  /** 可用于选择的 profile（按用户排序，仅启用项） */
  const selectableProfiles = computed(() => profiles.value.filter((profile) => profile.enabled));

  /** 拉取 profile 列表 */
  const load = async (force = false): Promise<CookieProfileSummary[]> => {
    if (loaded.value && !force) return profiles.value;
    profiles.value = await invoke<CookieProfileSummary[]>("cookie_profile_list");
    loaded.value = true;
    return profiles.value;
  };

  /** 配置变更后清空匹配缓存并重新拉取列表 */
  const reload = async (): Promise<void> => {
    matchCache.clear();
    await load(true);
  };

  /** 按链接匹配 profile（同一链接在本次会话内只请求一次） */
  const matchForUrl = async (url: string): Promise<CookieMatchResult | null> => {
    if (matchCache.has(url)) return matchCache.get(url) ?? null;
    const result = await invoke<CookieMatchResult | null>("cookie_profile_match", { url });
    matchCache.set(url, result);
    return result;
  };

  /** 把用户的选择意图解析为可交给 yt-dlp 的 cookie 快照 */
  const resolve = async (url: string, selection: CookieSelection): Promise<CookieUsage> => {
    if (selection.mode === "none") return { ...EMPTY_USAGE };
    await load();

    if (selection.mode === "profile") {
      const profileId = selection.profileId;
      if (!profileId) return { ...EMPTY_USAGE };
      const profile = profiles.value.find((item) => item.id === profileId);
      if (!profile?.enabled) return { ...EMPTY_USAGE };
      const cookieFile = await invoke<string>("cookie_profile_file", { id: profileId });
      return {
        mode: "profile",
        profileId,
        profileName: profile.name,
        matchedDomain: null,
        cookieFile,
      };
    }

    const matched = await matchForUrl(url);
    if (!matched) return { ...EMPTY_USAGE };
    return {
      mode: "auto",
      profileId: matched.profileId,
      profileName: matched.profileName,
      matchedDomain: matched.matchedDomain,
      cookieFile: matched.cookieFile,
    };
  };

  /** 只需要文件路径的场景（工具箱、频道归档等）：按链接自动匹配 */
  const resolveAutoFile = async (url: string): Promise<string | null> => {
    const usage = await resolve(url, { mode: "auto", profileId: null });
    return usage.cookieFile;
  };

  /** 使用命令行一次性指定文件（优先级最高，不参与匹配） */
  const resolveWithFile = (path: string | null): CookieUsage => {
    if (!path) return { ...EMPTY_USAGE };
    return {
      mode: "file",
      profileId: null,
      profileName: null,
      matchedDomain: null,
      cookieFile: path,
    };
  };

  /** 仅对本次 CLI 导入生效的 cookie 文件 */
  const setCliOverride = (path: string | null): void => {
    cliOverrideFile.value = path;
  };

  /** 取出并清空 CLI 一次性 cookie 文件（真正的一次性语义） */
  const takeCliOverride = (): string | null => {
    const path = cliOverrideFile.value;
    cliOverrideFile.value = null;
    return path;
  };

  const listProfiles = async (): Promise<CookieProfileSummary[]> => load();

  /** 保存 profile（不传 content 表示保留库中已有的 cookie 正文） */
  const saveProfile = async (input: CookieProfileInput): Promise<void> => {
    await invoke("cookie_profile_save", { input });
    await reload();
  };

  const deleteProfile = async (id: string): Promise<void> => {
    await invoke("cookie_profile_delete", { id });
    await reload();
  };

  const reorderProfiles = async (ids: string[]): Promise<void> => {
    await invoke("cookie_profile_reorder", { ids });
    await reload();
  };

  /** 校验 cookie 文本并返回可用的匹配域名 */
  const previewDomains = async (text: string): Promise<string[]> =>
    invoke<string[]>("cookie_preview_domains", { text });

  /** 管理员在设置页的匹配测试 */
  const testMatch = async (url: string): Promise<CookieMatchResult | null> => {
    matchCache.delete(url);
    return matchForUrl(url);
  };

  return {
    profiles,
    loaded,
    selectableProfiles,
    cliOverrideFile,
    load,
    reload,
    listProfiles,
    saveProfile,
    deleteProfile,
    reorderProfiles,
    previewDomains,
    matchForUrl,
    testMatch,
    resolve,
    resolveAutoFile,
    resolveWithFile,
    setCliOverride,
    takeCliOverride,
  };
});
