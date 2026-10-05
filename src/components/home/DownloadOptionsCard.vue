<script setup lang="ts">
import {
  formatFileSize,
  formatResolutionLabel,
  normalizeAudioFormat,
  normalizeVideoFormat,
} from "@/utils/normalizer";
import { getCodecKey, getCodecLabel, resolveDownloadMode } from "@/utils/formats";
import { useI18n } from "vue-i18n";
import { useThemeVars } from "naive-ui";
import type { VideoFormat, VideoInfo } from "@/types";

/** 三条流轨道：含音频的视频、纯视频、纯音频 */
type TrackKey = "muxed" | "video" | "audio";
/** 筛选维度：视频按编码与分辨率筛，音频只按编码筛 */
type FilterKey = "codec" | "resolution";

interface FormatCard {
  id: string;
  /** 首行：编码 · 容器 */
  head: string;
  /** 中间信息行：分辨率、帧率、动态范围等 */
  lines: string[];
  tags: string[];
  /** 末行：码率 · 体积 */
  tail: string;
  /** 悬浮提示用的完整描述 */
  label: string;
}

interface FilterView {
  key: FilterKey;
  label: string;
  options: { label: string; value: string }[];
}

interface TrackView {
  key: TrackKey;
  /** 轨道标题 */
  title: string;
  filters: FilterView[];
  cards: FormatCard[];
  visibleCards: FormatCard[];
  hiddenCount: number;
  selected: string;
  /** 卡片不可点选（该轨道已被含音频的视频格式取代） */
  cardsDisabled: boolean;
  note: string;
}

const { t } = useI18n();

const props = defineProps<{
  /** 纯视频流（不含音轨） */
  videoFormats: VideoFormat[];
  /** 已封装音轨的视频流 */
  muxedFormats: VideoFormat[];
  /** 纯音频流 */
  audioFormats: VideoFormat[];
  videoInfo: VideoInfo;
  /** 额外选项中的“不合并”：开启后双轨道只保留视频，合并说明不再展示 */
  noMerge?: boolean;
}>();

const selectedVideoFormat = defineModel<string>("selectedVideoFormat", { required: true });
const selectedAudioFormat = defineModel<string>("selectedAudioFormat", { required: true });

/** 主题主色用于选中卡片描边，避免硬编码主题色值 */
const themeVars = useThemeVars();
const accentStyle = computed(() => ({ "--format-accent": themeVars.value.primaryColor }));

const TRACK_KEYS: TrackKey[] = ["muxed", "video", "audio"];
const FILTER_KEYS: Record<TrackKey, FilterKey[]> = {
  muxed: ["codec", "resolution"],
  video: ["codec", "resolution"],
  audio: ["codec"],
};
const ALL = "all";
/** 折叠时每条轨道展示的格式卡片数量 */
const PREVIEW_COUNT = 6;

const filters = reactive<Record<TrackKey, Record<FilterKey, string>>>({
  muxed: { codec: ALL, resolution: ALL },
  video: { codec: ALL, resolution: ALL },
  audio: { codec: ALL, resolution: ALL },
});
const expanded = reactive<Record<TrackKey, boolean>>({ muxed: false, video: false, audio: false });
/** 含音频的视频被选中时暂存的音轨选择，切回纯视频轨时自动恢复 */
const suspendedAudioFormat = ref("");

/** 是否为正在直播 */
const isLive = computed(
  () => props.videoInfo.is_live === true || props.videoInfo.live_status === "is_live",
);

const formatsIncomplete = computed(() => {
  if (props.audioFormats.length > 0) return false;
  const heights = [...props.videoFormats, ...props.muxedFormats].map((format) => format.height || 0);
  if (!heights.length) return false;
  return Math.max(...heights) <= 360;
});

const formatsByTrack = computed<Record<TrackKey, VideoFormat[]>>(() => ({
  muxed: props.muxedFormats,
  video: props.videoFormats,
  audio: props.audioFormats,
}));

const selectedVideoGroup = computed<TrackKey | "">(() => {
  const id = selectedVideoFormat.value;
  if (!id) return "";
  if (props.muxedFormats.some((format) => format.format_id === id)) return "muxed";
  if (props.videoFormats.some((format) => format.format_id === id)) return "video";
  return "";
});

const videoOn = computed(() => Boolean(selectedVideoFormat.value));
const audioOn = computed(() => Boolean(selectedAudioFormat.value));
const muxedSelected = computed(() => selectedVideoGroup.value === "muxed");

/** 下载模式由轨道选择推导：两条都选中即合并，只选一条即仅该条 */
const downloadMode = computed(() =>
  resolveDownloadMode(selectedVideoFormat.value, selectedAudioFormat.value),
);

const modeLabel = computed(() => {
  if (muxedSelected.value) return t("detail.videoWithAudio");
  if (videoOn.value && audioOn.value) return `${t("detail.video")} + ${t("detail.audio")}`;
  if (videoOn.value) return t("detail.videoOnly");
  if (audioOn.value) return t("detail.audioOnly");
  return "";
});

const modeTagType = computed(() =>
  downloadMode.value === "default" && videoOn.value && audioOn.value ? "primary" : "info",
);

/** 只有“纯视频 + 音频”才需要 ffmpeg 合并 */
const showMergeHint = computed(
  () => selectedVideoGroup.value === "video" && audioOn.value && !props.noMerge,
);

/** 该格式在当前轨道下参与筛选的编码键 */
const trackCodecKey = (key: TrackKey, format: VideoFormat) =>
  getCodecKey(key === "audio" ? format.acodec : format.vcodec);

const matchesFilter = (
  key: TrackKey,
  format: VideoFormat,
  filter: FilterKey,
  value: string,
): boolean => {
  if (value === ALL) return true;
  if (filter === "codec") return trackCodecKey(key, format) === value;
  return String(format.height ?? 0) === value;
};

const buildFilterOptions = (key: TrackKey, filter: FilterKey, source: VideoFormat[]) => {
  if (filter === "codec") {
    const codecs = new Map<string, string>();
    for (const format of source) {
      codecs.set(
        trackCodecKey(key, format),
        getCodecLabel(key === "audio" ? format.acodec : format.vcodec),
      );
    }
    return [
      { label: t("detail.allCodecs"), value: ALL },
      ...Array.from(codecs, ([value, label]) => ({ value, label })),
    ];
  }
  const heights = [...new Set(source.map((format) => format.height ?? 0))].sort((a, b) => b - a);
  return [
    { label: t("detail.allResolutions"), value: ALL },
    ...heights.map((height) => ({ label: formatResolutionLabel(height), value: String(height) })),
  ];
};

/**
 * 筛选选项受同轨另一个筛选的结果约束，任一组合都保证有格式可选；
 * 只有一种取值时该筛选没有意义，不展示。
 */
const filtersByTrack = computed<Record<TrackKey, FilterView[]>>(() => {
  const result = {} as Record<TrackKey, FilterView[]>;
  for (const key of TRACK_KEYS) {
    result[key] = FILTER_KEYS[key]
      .map((filter) => {
        const other = filter === "codec" ? "resolution" : "codec";
        const scoped = formatsByTrack.value[key].filter(
          (format) =>
            !FILTER_KEYS[key].includes(other) ||
            matchesFilter(key, format, other, filters[key][other]),
        );
        return {
          key: filter,
          label: filter === "codec" ? t("detail.codec") : t("detail.tplResolution"),
          options: buildFilterOptions(key, filter, scoped),
        };
      })
      .filter((view) => view.options.length > 2 || filters[key][view.key] !== ALL);
  }
  return result;
});

const filteredByTrack = computed<Record<TrackKey, VideoFormat[]>>(() => {
  const result = {} as Record<TrackKey, VideoFormat[]>;
  for (const key of TRACK_KEYS) {
    result[key] = formatsByTrack.value[key].filter((format) =>
      FILTER_KEYS[key].every((filter) =>
        matchesFilter(key, format, filter, filters[key][filter]),
      ),
    );
  }
  return result;
});

/** 体积统一按实际数值展示，不带预估的“~”前缀 */
const sizeText = (filesize: number) => (filesize > 0 ? formatFileSize(filesize) : "—");

const buildVideoCard = (format: VideoFormat): FormatCard => {
  const n = normalizeVideoFormat(format, props.videoInfo.duration || 0);
  const resolution = [n.resolutionLabel, n.fpsLabel].filter(Boolean).join(" ");
  const lines = [resolution, n.dynamicRange].filter(Boolean);
  const head = [n.codec, n.container].filter(Boolean).join(" · ");
  return {
    id: n.formatId,
    head,
    lines,
    tags: [],
    tail: [n.bitrateLabel, sizeText(n.filesize)].filter(Boolean).join(" · "),
    label: [head, ...lines, n.filesizeLabel, `#${n.formatId}`].filter(Boolean).join(" · "),
  };
};

const buildAudioCard = (format: VideoFormat): FormatCard => {
  const n = normalizeAudioFormat(format, props.videoInfo.duration || 0);
  const head = [n.codec, n.container].filter(Boolean).join(" · ");
  const tags = [n.languageLabel || n.language, n.roleLabel].filter(Boolean);
  const lines = [n.channelsLabel].filter(Boolean);
  return {
    id: n.formatId,
    head,
    lines,
    tags,
    tail: [n.bitrateLabel, sizeText(n.filesize)].filter(Boolean).join(" · "),
    label: [head, tags.length ? `[${tags.join(" · ")}]` : "", ...lines, n.filesizeLabel, `#${n.formatId}`]
      .filter(Boolean)
      .join(" · "),
  };
};

const cardsByTrack = computed<Record<TrackKey, FormatCard[]>>(() => ({
  muxed: filteredByTrack.value.muxed.map(buildVideoCard),
  video: filteredByTrack.value.video.map(buildVideoCard),
  audio: filteredByTrack.value.audio.map(buildAudioCard),
}));

const buildTrackState = (
  key: TrackKey,
): Pick<TrackView, "selected" | "cardsDisabled" | "note"> => {
  if (key === "audio") {
    // 含音频的视频格式已经自带音轨，此时音轨不可再选
    if (muxedSelected.value) {
      return { selected: "", cardsDisabled: true, note: t("detail.audioLockedByMuxed") };
    }
    return { selected: selectedAudioFormat.value, cardsDisabled: false, note: "" };
  }
  const active = selectedVideoGroup.value === key;
  return { selected: active ? selectedVideoFormat.value : "", cardsDisabled: false, note: "" };
};

/** 只渲染有内容的轨道，没有对应格式的轨道直接隐藏 */
const tracks = computed<TrackView[]>(() => {
  const views: TrackView[] = [];
  const titles: Record<TrackKey, string> = {
    muxed: t("detail.videoWithAudio"),
    video: t("detail.videoNoAudio"),
    audio: t("detail.audio"),
  };
  for (const key of TRACK_KEYS) {
    if (!formatsByTrack.value[key].length) continue;
    const cards = cardsByTrack.value[key];
    views.push({
      key,
      title: titles[key],
      filters: filtersByTrack.value[key],
      cards,
      visibleCards: expanded[key] ? cards : cards.slice(0, PREVIEW_COUNT),
      hiddenCount: Math.max(cards.length - PREVIEW_COUNT, 0),
      ...buildTrackState(key),
    });
  }
  return views;
});

const clearSelection = (key: TrackKey) => {
  if (key === "audio") selectedAudioFormat.value = "";
  else selectedVideoFormat.value = "";
};

/** 点击卡片：未选中则选中，已选中则取消选中 */
const selectFormat = (key: TrackKey, formatId: string) => {
  if (key === "audio") {
    selectedAudioFormat.value = selectedAudioFormat.value === formatId ? "" : formatId;
    return;
  }
  selectedVideoFormat.value = selectedVideoFormat.value === formatId ? "" : formatId;
};

const changeFilter = (key: TrackKey, filter: FilterKey, value: string) => {
  filters[key][filter] = value;
  expanded[key] = false;
  // 联动后另一个筛选可能失去取值，回到“全部”避免出现空结果
  for (const other of FILTER_KEYS[key]) {
    if (other === filter) continue;
    const options = filtersByTrack.value[key].find((view) => view.key === other)?.options ?? [];
    if (!options.some((option) => option.value === filters[key][other])) filters[key][other] = ALL;
  }
  // 被筛掉的选中项不再参与下载，直接取消选中
  if (key !== "audio" && selectedVideoGroup.value !== key) return;
  const current = key === "audio" ? selectedAudioFormat.value : selectedVideoFormat.value;
  if (current && !filteredByTrack.value[key].some((format) => format.format_id === current)) {
    clearSelection(key);
  }
};

// 选中含音频的视频格式时音轨被取代，切回纯视频轨时恢复原来的音轨选择
watch(muxedSelected, (isMuxed) => {
  if (isMuxed) {
    if (selectedAudioFormat.value) {
      suspendedAudioFormat.value = selectedAudioFormat.value;
      selectedAudioFormat.value = "";
    }
    return;
  }
  if (suspendedAudioFormat.value) {
    if (props.audioFormats.some((format) => format.format_id === suspendedAudioFormat.value)) {
      selectedAudioFormat.value = suspendedAudioFormat.value;
    }
    suspendedAudioFormat.value = "";
  }
});

// 重新解析后清理已失效的筛选与展开状态
watch(
  () => [props.muxedFormats, props.videoFormats, props.audioFormats],
  () => {
    for (const key of TRACK_KEYS) {
      for (const filter of FILTER_KEYS[key]) {
        const options = filtersByTrack.value[key].find((view) => view.key === filter)?.options ?? [];
        if (!options.some((option) => option.value === filters[key][filter])) {
          filters[key][filter] = ALL;
        }
      }
      expanded[key] = false;
    }
  },
);
</script>

<template>
  <n-card :title="$t('detail.downloadMethod')" size="small" :style="accentStyle">
    <template #header-extra>
      <n-tag v-if="videoOn || audioOn" size="small" round :bordered="false" :type="modeTagType">
        {{ modeLabel }}
      </n-tag>
    </template>

    <n-flex vertical :size="12">
      <n-alert v-if="formatsIncomplete" type="warning" :bordered="false">
        {{ $t("detail.incompleteFormatsHint") }}
      </n-alert>

      <n-alert v-if="isLive" type="info" :bordered="false">
        {{ $t("detail.liveFormatHint") }}
      </n-alert>

      <section
        v-for="track in tracks"
        :key="track.key"
        class="track-block"
        :class="{ 'is-locked': track.cardsDisabled }"
        :aria-label="track.title"
      >
        <header class="track-head">
          <n-icon size="16">
            <icon-mdi-movie-open-outline v-if="track.key === 'muxed'" />
            <icon-mdi-music-note-outline v-else-if="track.key === 'audio'" />
            <icon-mdi-video-outline v-else />
          </n-icon>
          <span class="track-name">{{ track.title }}</span>
          <n-text v-if="track.note" depth="3" class="track-note">{{ track.note }}</n-text>
        </header>

        <div v-for="filter in track.filters" :key="filter.key" class="filter-row">
          <n-text depth="3" class="filter-label">{{ filter.label }}</n-text>
          <n-flex :size="6" wrap>
            <n-button
              v-for="option in filter.options"
              :key="option.value"
              size="tiny"
              round
              :type="filters[track.key][filter.key] === option.value ? 'primary' : 'default'"
              :secondary="filters[track.key][filter.key] === option.value"
              :quaternary="filters[track.key][filter.key] !== option.value"
              @click="changeFilter(track.key, filter.key, option.value)"
            >
              {{ option.label }}
            </n-button>
          </n-flex>
        </div>

        <div class="format-grid" :class="{ 'is-scroll': expanded[track.key] }" role="group">
          <button
            v-for="card in track.visibleCards"
            :key="card.id"
            type="button"
            class="format-card"
            :class="{ 'is-active': card.id === track.selected }"
            :aria-pressed="card.id === track.selected"
            :disabled="track.cardsDisabled"
            :title="card.label"
            @click="selectFormat(track.key, card.id)"
          >
            <span class="format-card__head">{{ card.head }}</span>
            <span v-if="card.tags.length" class="format-card__tags">
              <span v-for="tag in card.tags" :key="tag" class="format-card__tag">{{ tag }}</span>
            </span>
            <span v-for="line in card.lines" :key="line" class="format-card__line">{{ line }}</span>
            <span class="format-card__tail">{{ card.tail }}</span>
          </button>
        </div>

        <n-button
          v-if="track.hiddenCount > 0"
          text
          size="tiny"
          type="primary"
          class="expand-action"
          @click="expanded[track.key] = !expanded[track.key]"
        >
          {{
            expanded[track.key]
              ? $t("detail.collapseFormats")
              : $t("detail.expandFormats", { count: track.cards.length })
          }}
        </n-button>
      </section>

      <n-text v-if="showMergeHint" depth="3" class="merge-hint">
        <n-icon size="14"><icon-mdi-information-outline /></n-icon>
        {{ $t("detail.mergeTracksHint") }}
      </n-text>
    </n-flex>
  </n-card>
</template>

<style scoped lang="scss">
.track-block {
  padding: 10px 12px 12px;
  border: 1px solid var(--n-border-color);
  border-radius: 10px;
  transition: opacity 160ms var(--n-bezier, ease);

  &.is-locked {
    .format-grid,
    .expand-action {
      opacity: 0.45;
    }
  }
}

.track-head {
  display: flex;
  align-items: center;
  gap: 8px;

  .track-name {
    font-size: 13px;
    font-weight: 600;
  }

  .track-note {
    margin-inline-start: auto;
    font-size: 12px;
  }
}

.filter-row {
  display: flex;
  align-items: center;
  gap: 8px;
  margin-top: 8px;

  .filter-label {
    flex-shrink: 0;
    font-size: 12px;
  }
}

.format-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(168px, 1fr));
  gap: 8px;
  margin-top: 8px;

  &.is-scroll {
    max-height: 244px;
    overflow-y: auto;
    padding-inline-end: 4px;
  }
}

.format-card {
  position: relative;
  display: flex;
  flex-direction: column;
  gap: 2px;
  padding: 8px 10px;
  border: 1px solid var(--n-border-color);
  border-radius: 8px;
  background: transparent;
  color: inherit;
  font: inherit;
  text-align: start;
  cursor: pointer;
  transition:
    border-color 140ms var(--n-bezier, ease),
    box-shadow 140ms var(--n-bezier, ease);

  &:hover:not(:disabled) {
    border-color: var(--format-accent);
  }

  &:disabled {
    cursor: not-allowed;
  }

  &.is-active {
    border-color: var(--format-accent);
    box-shadow: inset 0 0 0 1px var(--format-accent);

    .format-card__head,
    .format-card__line {
      color: var(--format-accent);
    }
  }
}

.format-card__head {
  font-size: 13px;
  font-weight: 600;
  overflow: hidden;
  white-space: nowrap;
  text-overflow: ellipsis;
}

.format-card__tags {
  display: flex;
  flex-wrap: wrap;
  gap: 4px;
}

.format-card__tag {
  padding: 0 5px;
  border-radius: 4px;
  background: rgba(128, 128, 128, 0.18);
  font-size: 11px;
  line-height: 17px;
}

.format-card__line,
.format-card__tail {
  font-size: 12px;
  opacity: 0.62;
  font-variant-numeric: tabular-nums;
  overflow: hidden;
  white-space: nowrap;
  text-overflow: ellipsis;
}

.format-card__tail {
  margin-top: 2px;
}

.expand-action {
  margin-top: 8px;
}

.merge-hint {
  display: flex;
  align-items: center;
  gap: 4px;
  font-size: 12px;
}
</style>
