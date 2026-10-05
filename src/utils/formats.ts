import type { DownloadMode, PendingItem, VideoFormat } from "@/types";

export const getCodecKey = (codec: string): string => {
  const normalized = codec.toLowerCase();
  if (/^(avc1|avc3|h264)/.test(normalized)) return "h264";
  if (/^(hev1|hvc1|h265|hevc)/.test(normalized)) return "hevc";
  if (/^(av01|av1)/.test(normalized)) return "av1";
  if (/^(vp09|vp9)/.test(normalized)) return "vp9";
  if (/^vp8/.test(normalized)) return "vp8";
  if (/^(mp4a|aac)/.test(normalized)) return "aac";
  if (/^(opus)/.test(normalized)) return "opus";
  if (/^(vorbis)/.test(normalized)) return "vorbis";
  if (/^(mp3)/.test(normalized)) return "mp3";
  if (/^(ec-3|eac3)/.test(normalized)) return "eac3";
  if (/^(ac-3|ac3)/.test(normalized)) return "ac3";
  return normalized.split(".")[0] || "unknown";
};

const CODEC_LABELS: Record<string, string> = {
  h264: "H.264",
  hevc: "H.265",
  av1: "AV1",
  vp9: "VP9",
  vp8: "VP8",
  aac: "AAC",
  opus: "Opus",
  vorbis: "Vorbis",
  mp3: "MP3",
  eac3: "E-AC-3",
  ac3: "AC-3",
  unknown: "Unknown",
};

export const getCodecLabel = (codec: string): string => {
  const key = getCodecKey(codec);
  return CODEC_LABELS[key] || key.toUpperCase();
};

/**
 * 由所选轨道推导下载模式，避免模式与格式选择两份状态互相打架。
 *
 * 两条轨道都选中 → 合并下载；只选中一条 → 仅下载该条轨道；
 * 两条都没选（站点未提供可选格式）时回落 default，交由 yt-dlp 自行挑选。
 */
export const resolveDownloadMode = (videoFormat: string, audioFormat: string): DownloadMode => {
  if (videoFormat && !audioFormat) return "video";
  if (!videoFormat && audioFormat) return "audio";
  return "default";
};

/** 该格式是否自带音轨（仅视频流时 acodec 为 none） */
export const hasAudioStream = (format: VideoFormat): boolean =>
  Boolean(format.acodec && format.acodec !== "none");

/**
 * 查找所选视频格式：所选格式可能来自纯视频轨或含音频轨，两份列表都要检索
 */
export const findSelectedVideoFormat = (
  item: Pick<PendingItem, "videoFormats" | "muxedFormats" | "selectedVideoFormat">,
): VideoFormat | undefined =>
  item.videoFormats.find((format) => format.format_id === item.selectedVideoFormat) ??
  item.muxedFormats.find((format) => format.format_id === item.selectedVideoFormat);

const audioRoleRank = (format: VideoFormat): number => {
  const description = `${format.format_note || ""} ${format.format || ""}`.toLowerCase();
  if (description.includes("original")) return 3;
  if (description.includes("audio description") || description.includes("descriptive")) return -2;
  if (description.includes("dubbed") || description.includes("translated")) return -1;
  if (description.includes("default")) return 2;
  return 0;
};

/**
 * 保留 yt-dlp 的语言偏好语义，优先原声，再按是否 DRC、码率排序。
 * 旧逻辑只比较码率，会把高码率配音轨误设为默认值。
 */
export const compareAudioFormats = (a: VideoFormat, b: VideoFormat): number => {
  if (a.language_preference != null && b.language_preference != null) {
    const preferenceDifference = b.language_preference - a.language_preference;
    if (preferenceDifference !== 0) return preferenceDifference;
  }

  const roleDifference = audioRoleRank(b) - audioRoleRank(a);
  if (roleDifference !== 0) return roleDifference;

  const aDrc = /\bdrc\b/i.test(a.format_note || "") ? 1 : 0;
  const bDrc = /\bdrc\b/i.test(b.format_note || "") ? 1 : 0;
  if (aDrc !== bDrc) return aDrc - bDrc;

  return (b.abr || b.tbr || 0) - (a.abr || a.tbr || 0);
};
