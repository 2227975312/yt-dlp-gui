<script setup lang="ts">
import { formatFileSize } from "@/utils/format";
import { estimateFileSize } from "@/utils/normalizer";
import { findSelectedVideoFormat, resolveDownloadMode } from "@/utils/formats";
import { useVideoStore } from "@/stores/video";
import { useSettingStore } from "@/stores/setting";
import { usePendingStore } from "@/stores/pending";
import { useDownloadLauncher } from "@/composables/useDownloadLauncher";
import { useI18n } from "vue-i18n";
import { useThemeVars } from "naive-ui";
import type { VideoInfo } from "@/types";

const { t } = useI18n();
const router = useRouter();
const videoStore = useVideoStore();
const settingStore = useSettingStore();
const pendingStore = usePendingStore();
const { launchDownload } = useDownloadLauncher();

/** 标签条配色跟随主题，避免硬编码色值 */
const themeVars = useThemeVars();
const tabStripStyle = computed(() => ({
  "--tab-strip-slot": themeVars.value.actionColor,
  "--tab-strip-border": themeVars.value.dividerColor,
  "--tab-strip-accent": themeVars.value.primaryColor,
  "--tab-strip-accent-hover": themeVars.value.primaryColorHover,
}));

const activeItem = computed(() => pendingStore.activeItem);

/**
 * 待下载页偏好型选项持久化
 */
const persistedOptionSnapshot = computed(() => {
  const item = activeItem.value;
  if (!item) return null;
  return {
    embedSubs: item.embedSubs,
    embedThumbnail: item.embedThumbnail,
    writeThumbnail: item.writeThumbnail,
    writeDescription: item.writeDescription,
    embedMetadata: item.embedMetadata,
    embedChapters: item.embedChapters,
    sponsorblockRemove: item.sponsorblockRemove,
    extractAudio: item.extractAudio,
    audioConvertFormat: item.audioConvertFormat,
    noMerge: item.noMerge,
    recodeFormat: item.recodeFormat,
    remuxFormat: item.remuxFormat,
    limitRate: item.limitRate,
    customArgs: item.customArgs,
  };
});

watch(persistedOptionSnapshot, (snapshot) => {
  if (!snapshot) return;
  settingStore.defaultEmbedSubs = snapshot.embedSubs;
  settingStore.defaultEmbedThumbnail = snapshot.embedThumbnail;
  settingStore.defaultWriteThumbnail = snapshot.writeThumbnail;
  settingStore.defaultWriteDescription = snapshot.writeDescription;
  settingStore.defaultEmbedMetadata = snapshot.embedMetadata;
  settingStore.defaultEmbedChapters = snapshot.embedChapters;
  settingStore.defaultSponsorblockRemove = snapshot.sponsorblockRemove;
  settingStore.defaultExtractAudio = snapshot.extractAudio;
  settingStore.defaultAudioConvertFormat = snapshot.audioConvertFormat;
  settingStore.defaultNoMerge = snapshot.noMerge;
  settingStore.defaultRecodeFormat = snapshot.recodeFormat;
  settingStore.defaultRemuxFormat = snapshot.remuxFormat;
  settingStore.defaultLimitRate = snapshot.limitRate;
  settingStore.defaultCustomArgs = snapshot.customArgs;
});

const estimatedSize = computed(() => {
  const item = activeItem.value;
  if (!item) return 0;
  const downloadMode = resolveDownloadMode(item.selectedVideoFormat, item.selectedAudioFormat);
  let total = 0;
  const duration = item.videoInfo?.duration || 0;
  if (downloadMode !== "audio") {
    const vf = findSelectedVideoFormat(item);
    if (vf) {
      const { size } = estimateFileSize(vf, duration);
      total += size;
    }
  }
  if (downloadMode !== "video") {
    const af = item.audioFormats.find((f) => f.format_id === item.selectedAudioFormat);
    if (af) {
      const { size } = estimateFileSize(af, duration);
      total += size;
    }
  }
  return total;
});

const estimatedSizeText = computed(() => {
  if (!estimatedSize.value) return t("common.unknown");
  return formatFileSize(estimatedSize.value);
});

const dirCardRef = ref<HTMLElement | null>(null);

/** 音视频轨道一个都没选时不允许下载 */
const hasFormatSelection = computed(
  () => Boolean(activeItem.value?.selectedVideoFormat || activeItem.value?.selectedAudioFormat),
);

/**
 * 标签文字：按半角宽度估算截断（汉字计 2），保证末尾一定能看到省略号；
 * 先截断再交给 #tab 插槽，比依赖 text-overflow 在各平台更可靠。
 * 完整标题通过 tab-props 的 title 属性悬浮查看。
 */
const tabLabel = (title: string): string => {
  if (!title) return t("detail.unknownVideo");
  const LIMIT = 24;
  let width = 0;
  let label = "";
  for (const char of title) {
    const charWidth = /[\u2e80-\u9fff\uff00-\uffef]/.test(char) ? 2 : 1;
    if (width + charWidth > LIMIT) return `${label}…`;
    width += charWidth;
    label += char;
  }
  return label;
};

const handleTabClose = (name: string | number) => {
  pendingStore.remove(String(name));
};

const handleTabAdd = () => {
  router.push({ name: "home" });
};

const handleBackToHome = () => {
  router.push({ name: "home" });
};

/** 重新获取当前项视频信息 */
const handleRefresh = async () => {
  const item = activeItem.value;
  if (!item) return;
  const data = await videoStore.fetchVideoInfo(item.url);
  if (data) {
    pendingStore.refresh(item.id, data);
    window.$message.success(t("detail.refreshSuccess"));
  }
};

/** 开始下载当前项 */
const handleDownload = async () => {
  const item = activeItem.value;
  if (!item) return;
  const result = await launchDownload(item);
  if (result === "missing-directory") {
    dirCardRef.value?.scrollIntoView({ behavior: "smooth", block: "center" });
  } else if (result === "started" || result === "queued") {
    pendingStore.remove(item.id);
    router.push({ name: "downloads" });
  }
};
</script>

<template>
  <div class="pending-page">
    <template v-if="pendingStore.items.length > 0">
      <n-tabs
        v-model:value="pendingStore.activeId"
        type="card"
        size="small"
        closable
        class="tabs-bar"
        :style="tabStripStyle"
        @close="handleTabClose"
      >
        <template #prefix>
          <n-tooltip>
            <template #trigger>
              <n-button
                size="small"
                strong
                secondary
                circle
                :aria-label="$t('common.back')"
                @click="handleBackToHome"
              >
                <template #icon>
                  <n-icon><icon-mdi-arrow-left /></n-icon>
                </template>
              </n-button>
            </template>
            {{ $t("common.back") }}
          </n-tooltip>
        </template>

        <template #suffix>
          <n-tooltip>
            <template #trigger>
              <n-button
                size="small"
                strong
                secondary
                circle
                :aria-label="$t('pending.goParse')"
                @click="handleTabAdd"
              >
                <template #icon>
                  <n-icon><icon-mdi-plus /></n-icon>
                </template>
              </n-button>
            </template>
            {{ $t("pending.goParse") }}
          </n-tooltip>
        </template>

        <n-tab-pane
          v-for="item in pendingStore.items"
          :key="item.id"
          :name="item.id"
          :tab-props="{ title: tabLabel(item.videoInfo.title) }"
          display-directive="show"
        >
          <!-- 标签文字必须走 #tab 插槽：无子节点的 pane 会让 naive-ui 用空默认插槽覆盖 tab 属性 -->
          <template #tab>{{ tabLabel(item.videoInfo.title) }}</template>
        </n-tab-pane>
      </n-tabs>

      <div v-if="activeItem" :key="activeItem.id" class="pending-content">
        <n-flex :size="8" align="center" :wrap="false" style="margin-bottom: 16px">
          <n-input
            :value="activeItem.url"
            :placeholder="$t('detail.videoLink')"
            size="small"
            round
            readonly
            style="flex: 1; min-width: 0"
          />
          <n-button
            size="small"
            strong
            secondary
            round
            :loading="videoStore.fetching"
            @click="handleRefresh"
          >
            <template #icon>
              <n-icon><icon-mdi-refresh /></n-icon>
            </template>
          </n-button>
        </n-flex>

        <VideoInfoCard
          :video-info="activeItem.videoInfo as VideoInfo"
          :is-playlist="activeItem.isPlaylist"
          :playlist-count="activeItem.playlistEntries.length"
          class="section-card"
        />

        <n-card
          v-if="activeItem.isPlaylist && activeItem.playlistEntries.length > 0"
          size="small"
          class="section-card"
        >
          <template #header>
            <n-flex align="center" :size="8">
              <n-icon size="16"><icon-mdi-playlist-play /></n-icon>
              <span>{{ $t("detail.playlist") }}</span>
              <n-tag size="small" round :bordered="false" type="info">
                {{ activeItem.selectedPlaylistItems.length }} /
                {{ activeItem.playlistEntries.length }}
              </n-tag>
            </n-flex>
          </template>
          <template #header-extra>
            <n-flex :size="8">
              <n-button
                size="tiny"
                secondary
                @click="
                  activeItem.selectedPlaylistItems = activeItem.playlistEntries.map((_, i) => i + 1)
                "
              >
                {{ $t("common.selectAll") }}
              </n-button>
              <n-button size="tiny" secondary @click="activeItem.selectedPlaylistItems = []">
                {{ $t("common.deselectAll") }}
              </n-button>
            </n-flex>
          </template>
          <n-checkbox-group v-model:value="activeItem.selectedPlaylistItems">
            <n-flex vertical :size="6">
              <n-checkbox
                v-for="(entry, index) in activeItem.playlistEntries"
                :key="entry.id"
                :value="index + 1"
                :label="`P${index + 1} ${entry.title}`"
              />
            </n-flex>
          </n-checkbox-group>
        </n-card>

        <DownloadOptionsCard
          v-model:selected-video-format="activeItem.selectedVideoFormat"
          v-model:selected-audio-format="activeItem.selectedAudioFormat"
          :video-formats="activeItem.videoFormats"
          :muxed-formats="activeItem.muxedFormats"
          :audio-formats="activeItem.audioFormats"
          :video-info="activeItem.videoInfo as VideoInfo"
          :no-merge="activeItem.noMerge"
          class="section-card"
        />

        <SubtitleCard
          v-model:selected-subtitles="activeItem.selectedSubtitles"
          :video-info="activeItem.videoInfo as VideoInfo"
          class="section-card"
        />

        <ExtraOptionsCard
          v-model:start-time="activeItem.startTime"
          v-model:end-time="activeItem.endTime"
          v-model:embed-subs="activeItem.embedSubs"
          v-model:embed-thumbnail="activeItem.embedThumbnail"
          v-model:write-thumbnail="activeItem.writeThumbnail"
          v-model:write-description="activeItem.writeDescription"
          v-model:embed-metadata="activeItem.embedMetadata"
          v-model:embed-chapters="activeItem.embedChapters"
          v-model:sponsorblock-remove="activeItem.sponsorblockRemove"
          v-model:extract-audio="activeItem.extractAudio"
          v-model:audio-convert-format="activeItem.audioConvertFormat"
          v-model:no-merge="activeItem.noMerge"
          v-model:recode-format="activeItem.recodeFormat"
          v-model:remux-format="activeItem.remuxFormat"
          v-model:limit-rate="activeItem.limitRate"
          v-model:ffmpeg-args="activeItem.ffmpegArgs"
          v-model:custom-args="activeItem.customArgs"
          :video-info="activeItem.videoInfo as VideoInfo"
          class="section-card"
        />

        <div ref="dirCardRef" class="section-card">
          <DownloadDirCard />
        </div>

        <DownloadBar
          :estimated-size-text="estimatedSizeText"
          :disabled="!hasFormatSelection"
          @download="handleDownload"
        />
      </div>
    </template>

    <n-empty v-else :description="$t('pending.empty')" class="empty-state">
      <template #extra>
        <n-button type="primary" round @click="handleBackToHome">
          <template #icon>
            <n-icon><icon-mdi-magnify /></n-icon>
          </template>
          {{ $t("pending.goParse") }}
        </n-button>
      </template>
    </n-empty>
  </div>
</template>

<style scoped lang="scss">
.pending-page {
  position: relative;
  height: 100%;
  overflow-y: auto;
  padding: 16px;

  .tabs-bar {
    margin-bottom: 12px;
    // card 型标签条是为“标签与内容面板连成一体”设计的，本页内容不在 pane 里，
    // 这里把主题的分隔线色置空，一次性去掉吸附在标签条上的所有连接线。
    // naive-ui 把这些变量写成内联样式，所以必须 !important。
    --n-tab-border-color: transparent !important;

    :deep(.n-tabs-nav) {
      align-items: center;
    }

    // 标签底槽：挂在 inline-flex 的 tabs-wrapper 上，紧贴标签本身，
    // 避免通栏底槽跟下方的 URL 输入框长得一样
    :deep(.n-tabs-wrapper) {
      align-items: center;
      padding: 3px;
      border-radius: 999px;
      background-color: var(--tab-strip-slot);
    }

    // 标签间距：naive 用内联变量控制间隔元素的宽度，这里直接改元素本身
    :deep(.n-tabs-tab-pad) {
      width: 6px !important;
    }

    // naive-ui 的 card 型选择器深度到 5 层（.n-tabs.n-tabs--top.n-tabs--card-type .n-tabs-tab--active），
    // 普通 :deep() 覆盖会出现“只有一半生效”（例如激活标签底边仍是透明的），
    // 所以胶囊的形状与配色声明统一用 !important 锁住。
    :deep(.n-tabs-tab) {
      height: 26px !important;
      padding: 0 12px !important;
      align-items: center;
      border: 1px solid var(--tab-strip-border) !important;
      border-radius: 999px !important;
      background-color: transparent !important;
      transition:
        border-color 0.16s var(--n-bezier, ease),
        background-color 0.16s var(--n-bezier, ease);
    }

    :deep(.n-tabs-tab:not(.n-tabs-tab--active):hover) {
      border-color: var(--tab-strip-accent-hover) !important;
    }

    :deep(.n-tabs-tab--closable) {
      padding-inline-end: 6px !important;
    }

    :deep(.n-tabs-tab--active) {
      // 压掉 card 型“激活标签底边透明”的规则，否则胶囊底部会开一个口
      border-color: var(--tab-strip-accent) !important;
      // 旧版 WebKit 不认 color-mix，上一条会保留透明底
      background-color: transparent !important;
      background-color: color-mix(in srgb, var(--tab-strip-accent) 20%, transparent) !important;
      box-shadow: 0 1px 2px rgba(0, 0, 0, 0.06);
    }

    :deep(.n-tabs-tab__label) {
      max-width: 200px;
      overflow: hidden;
      white-space: nowrap;
      text-overflow: ellipsis;
    }

    // 关闭按钮平时隐藏，悬浮或激活时才出现；隐藏时不拦截点击
    :deep(.n-tabs-tab__close) {
      opacity: 0;
      pointer-events: none;
      transition: opacity 0.16s var(--n-bezier, ease);
    }

    :deep(.n-tabs-tab:hover .n-tabs-tab__close),
    :deep(.n-tabs-tab--active .n-tabs-tab__close) {
      opacity: 1;
      pointer-events: auto;
    }

    :deep(.n-tabs-nav__prefix) {
      padding-right: 8px;
    }

    :deep(.n-tabs-nav__suffix) {
      padding-left: 8px;
    }
  }

  .section-card {
    margin-bottom: 16px;
  }

  .pending-content {
    padding-bottom: 96px;
  }

  .empty-state {
    margin-top: 120px;
  }
}
</style>
