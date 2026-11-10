<script setup lang="ts">
import { readText } from "@tauri-apps/plugin-clipboard-manager";
import { useCookieStore } from "@/stores/cookie";
import { useI18n } from "vue-i18n";
import type { CookieProfileSummary } from "@/types";

interface Props {
  show: boolean;
  /** 编辑目标；为空表示新建 */
  profile: CookieProfileSummary | null;
}

const props = defineProps<Props>();

const emit = defineEmits<{
  (event: "update:show", value: boolean): void;
  (event: "saved"): void;
}>();

const { t } = useI18n();
const cookieStore = useCookieStore();

const name = ref("");
const content = ref("");
const domains = ref<string[]>([]);
const domainInput = ref("");
const enabled = ref(true);
const saving = ref(false);
const detecting = ref(false);

/**
 * 把后端错误码翻译成可理解的提示
 *
 * 未识别的错误原样展示，避免把「命令缺失」这类问题误报成格式错误。
 */
const showCookieError = (error: unknown): void => {
  const message = String(error);
  const lineMatched = /err_invalid_cookie_line:(\d+)/.exec(message);
  if (lineMatched) {
    window.$message.error(t("cookie.invalidFormatLine", { line: lineMatched[1] }));
  } else if (/err_too_many_cookies|err_cookie_too_large/.test(message)) {
    window.$message.error(t("cookie.tooManyCookies"));
  } else if (/err_invalid_cookie_header|err_empty_cookie_file/.test(message)) {
    window.$message.error(t("cookie.invalidFormat"));
  } else if (/err_empty_cookie_profile_name/.test(message)) {
    window.$message.warning(t("cookie.nameRequired"));
  } else if (/err_empty_cookie_domains/.test(message)) {
    window.$message.warning(t("cookie.domainsRequired"));
  } else if (/err_invalid_cookie_profile_id/.test(message)) {
    window.$message.error(t("cookie.profileInvalidId"));
  } else if (/err_cookie_profile_missing/.test(message)) {
    window.$message.error(t("cookie.profileMissing"));
  } else {
    window.$message.error(t("common.saveFailed", { e: message }));
  }
};

const showModal = computed({
  get: () => props.show,
  set: (value: boolean) => emit("update:show", value),
});

const isEditing = computed(() => Boolean(props.profile));

/**
 * 
 * 载入编辑目标
 *
 * cookie 正文保存后不再回显（避免在界面上暴露敏感内容），编辑时总是留空：
 * 用户不重新粘贴则保留库中已有正文，粘贴新内容则整体覆盖。
 */
const loadForm = (): void => {
  name.value = props.profile?.name ?? "";
  domains.value = [...(props.profile?.domains ?? [])];
  enabled.value = props.profile?.enabled ?? true;
  content.value = "";
  domainInput.value = "";
};

watch(
  () => [props.show, props.profile?.id],
  () => {
    if (props.show) loadForm();
  },
);

/** 是否粘贴了新的 cookie 正文（为空即表示沿用已有内容） */
const hasNewContent = computed(() => content.value.trim().length > 0);

/** 添加匹配域名（前端做一次轻量归一化，后端保存时还会再校验一次） */
const addDomain = (): void => {
  const normalized = domainInput.value
    .trim()
    .replace(/^\*\./, "")
    .replace(/^\.+/, "")
    .replace(/\/+$/, "")
    .toLowerCase();
  if (!normalized) return;
  if (domains.value.includes(normalized)) {
    window.$message.warning(t("cookie.domainDuplicated", { domain: normalized }));
    return;
  }
  domains.value = [...domains.value, normalized].sort();
  domainInput.value = "";
};

const removeDomain = (domain: string): void => {
  domains.value = domains.value.filter((item) => item !== domain);
};

/** 按 cookie 正文自动识别匹配域名 */
const detectDomains = async (force = false): Promise<void> => {
  if (!content.value.trim()) {
    window.$message.warning(t("cookie.emptyContent"));
    return;
  }
  if (!force && domains.value.length > 0) return;
  detecting.value = true;
  try {
    const detected = await cookieStore.previewDomains(content.value);
    if (detected.length === 0) {
      window.$message.warning(t("cookie.noDomainDetected"));
      return;
    }
    domains.value = detected;
  } catch (error: unknown) {
    showCookieError(error);
  } finally {
    detecting.value = false;
  }
};

/** 从剪贴板填入 cookie 文本 */
const handlePaste = async (): Promise<void> => {
  try {
    const text = (await readText()).trim();
    if (!text) {
      window.$message.warning(t("clipboard.empty"));
      return;
    }
    content.value = text;
    await detectDomains(true);
    window.$message.success(t("clipboard.pasteSuccess"));
  } catch {
    window.$message.warning(t("clipboard.readFailed"));
  }
};

const handleSave = async (): Promise<void> => {
  if (!name.value.trim()) {
    window.$message.warning(t("cookie.nameRequired"));
    return;
  }
  // 新建时必须提供正文；编辑时留空表示沿用已有正文
  if (!hasNewContent.value && !isEditing.value) {
    window.$message.warning(t("cookie.emptyContent"));
    return;
  }
  if (domains.value.length === 0) {
    window.$message.warning(t("cookie.domainsRequired"));
    return;
  }
  saving.value = true;
  try {
    await cookieStore.saveProfile({
      id: props.profile?.id ?? null,
      name: name.value.trim(),
      content: hasNewContent.value ? content.value : null,
      domains: domains.value,
      enabled: enabled.value,
    });
    window.$message.success(t("cookie.saved"));
    emit("saved");
    showModal.value = false;
  } catch (error: unknown) {
    showCookieError(error);
  } finally {
    saving.value = false;
  }
};
</script>

<template>
  <n-modal
    v-model:show="showModal"
    preset="card"
    :title="isEditing ? $t('cookie.editProfile') : $t('cookie.addProfile')"
    style="width: min(720px, 92vw)"
    :mask-closable="false"
  >
    <n-flex vertical :size="14">
      <n-flex vertical :size="6">
        <n-text depth="3" style="font-size: 13px">{{ $t("cookie.nameLabel") }}</n-text>
        <n-input v-model:value="name" size="medium" :placeholder="$t('cookie.namePlaceholder')" />
      </n-flex>

      <n-flex vertical :size="6">
        <n-flex align="center" justify="space-between">
          <n-text depth="3" style="font-size: 13px">{{ $t("cookie.contentLabel") }}</n-text>
          <n-flex :size="8">
            <n-button size="small" secondary @click="handlePaste">
              <template #icon>
                <n-icon><icon-mdi-content-paste /></n-icon>
              </template>
              {{ $t("common.paste") }}
            </n-button>
            <n-button size="small" secondary :loading="detecting" @click="detectDomains(true)">
              <template #icon>
                <n-icon><icon-mdi-auto-fix /></n-icon>
              </template>
              {{ $t("cookie.detectDomains") }}
            </n-button>
          </n-flex>
        </n-flex>
        <n-alert v-if="isEditing" type="warning" :bordered="false" style="font-size: 12px">
          {{ $t("cookie.contentKeepHint") }}
        </n-alert>
        <n-input
          v-model:value="content"
          type="textarea"
          :placeholder="isEditing ? $t('cookie.textPlaceholderKeep') : $t('cookie.textPlaceholder')"
          :autosize="{ minRows: 6, maxRows: 14 }"
          size="medium"
          @blur="detectDomains()"
        />
      </n-flex>

      <n-flex vertical :size="8">
        <n-alert type="info" :bordered="false" style="font-size: 12px">
          {{ $t("cookie.domainsHint") }}
        </n-alert>
        <n-flex :size="8" :wrap="false" align="center">
          <n-input
            v-model:value="domainInput"
            size="medium"
            :placeholder="$t('cookie.domainPlaceholder')"
            style="flex: 1"
            @keydown.enter.prevent="addDomain"
          />
          <n-button size="medium" secondary @click="addDomain">
            <template #icon>
              <n-icon><icon-mdi-plus /></n-icon>
            </template>
            {{ $t("common.add") }}
          </n-button>
        </n-flex>
        <n-flex v-if="domains.length > 0" :size="6" :wrap="true">
          <n-tag
            v-for="domain in domains"
            :key="domain"
            size="medium"
            closable
            :bordered="false"
            type="info"
            @close="removeDomain(domain)"
          >
            {{ domain }}
          </n-tag>
        </n-flex>
        <n-text v-else depth="3" style="font-size: 12px">
          {{ $t("cookie.noDomains") }}
        </n-text>
      </n-flex>

      <n-flex align="center" :size="8">
        <n-switch v-model:value="enabled" size="small" />
        <n-text depth="3" style="font-size: 13px">{{ $t("cookie.enabledHint") }}</n-text>
      </n-flex>
    </n-flex>

    <template #footer>
      <n-flex justify="end" :size="8">
        <n-button size="medium" @click="showModal = false">{{ $t("common.cancel") }}</n-button>
        <n-button size="medium" type="primary" :loading="saving" @click="handleSave">
          {{ $t("common.save") }}
        </n-button>
      </n-flex>
    </template>
  </n-modal>
</template>
