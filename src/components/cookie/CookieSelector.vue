<script setup lang="ts">
import { useCookieStore } from "@/stores/cookie";
import { useI18n } from "vue-i18n";
import type { CookieMatchResult, CookieSelection } from "@/types";

interface Props {
  /** 当前待解析/待下载的链接，用于展示自动匹配结果 */
  url: string;
  /** 用户选择意图（v-model） */
  selection: CookieSelection;
  /** 解析时实际使用的 cookie 快照，用于说明「这次用的是谁」 */
  usageName?: string | null;
  disabled?: boolean;
}

const props = withDefaults(defineProps<Props>(), {
  usageName: null,
  disabled: false,
});

const emit = defineEmits<{
  (event: "update:selection", value: CookieSelection): void;
}>();

const { t } = useI18n();
const cookieStore = useCookieStore();

const preview = ref<CookieMatchResult | null>(null);
const previewing = ref(false);

const selectOptions = computed(() => [
  { label: t("cookie.auto"), value: "auto" },
  { label: t("cookie.none"), value: "none" },
  ...cookieStore.selectableProfiles.map((profile) => ({
    label: profile.name,
    value: `profile:${profile.id}`,
  })),
]);

const selectedValue = computed(
  () =>
    props.selection.mode === "profile"
      ? `profile:${props.selection.profileId ?? ""}`
      : props.selection.mode,
);

/** 选择项变化时回写用户意图 */
const handleSelectChange = (value: string): void => {
  if (value === "auto" || value === "none") {
    emit("update:selection", { mode: value, profileId: null });
    return;
  }
  emit("update:selection", { mode: "profile", profileId: value.replace(/^profile:/, "") });
};

/** 当前选择的说明文案；自动匹配时展示命中的 profile 与规则 */
const previewText = computed(() => {
  if (props.selection.mode === "none") return t("cookie.notUsed");
  if (props.selection.mode === "profile") {
    const name = cookieStore.profiles.find((item) => item.id === props.selection.profileId)?.name;
    return name ? t("cookie.resolved", { name }) : t("cookie.notUsed");
  }
  if (!props.url.trim()) return t("cookie.autoHint");
  if (previewing.value) return t("cookie.matching");
  if (preview.value) {
    return t("cookie.resolvedAuto", {
      name: preview.value.profileName,
      domain: preview.value.matchedDomain,
    });
  }
  return t("cookie.noMatch");
});

/** 重新计算自动匹配的预览结果 */
const refreshPreview = async (): Promise<void> => {
  if (props.selection.mode !== "auto" || !props.url.trim()) {
    preview.value = null;
    return;
  }
  previewing.value = true;
  try {
    preview.value = await cookieStore.matchForUrl(props.url.trim());
  } catch {
    preview.value = null;
  } finally {
    previewing.value = false;
  }
};

onMounted(async () => {
  await cookieStore.load();
  await refreshPreview();
});

watch(
  () => [props.url, props.selection.mode, props.selection.profileId],
  () => void refreshPreview(),
);

defineExpose({ refreshPreview });
</script>

<template>
  <n-flex align="center" :size="8" :wrap="false" class="cookie-selector">
    <n-icon size="16" :depth="3"><icon-mdi-cookie-outline /></n-icon>
    <n-select
      :value="selectedValue"
      :options="selectOptions"
      size="small"
      :disabled="disabled"
      style="width: 168px"
      @update:value="handleSelectChange"
    />
    <n-text depth="3" class="cookie-hint">
      <n-ellipsis :line-clamp="1" :tooltip="false">
        {{ usageName ?? previewText }}
      </n-ellipsis>
    </n-text>
  </n-flex>
</template>

<style scoped lang="scss">
.cookie-selector {
  width: 100%;

  .cookie-hint {
    flex: 1;
    min-width: 0;
    font-size: 12px;
  }
}
</style>
