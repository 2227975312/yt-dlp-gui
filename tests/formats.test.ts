import { describe, it, expect } from "vitest";
import type { VideoFormat } from "@/types";
import {
  compareAudioFormats,
  findSelectedVideoFormat,
  getCodecKey,
  getCodecLabel,
  hasAudioStream,
  resolveDownloadMode,
} from "@/utils/formats";

const audioFormat = (overrides: Partial<VideoFormat>): VideoFormat => ({
  format_id: "251",
  ext: "webm",
  resolution: "audio only",
  height: null,
  width: null,
  fps: null,
  vcodec: "none",
  acodec: "opus",
  filesize: null,
  filesize_approx: null,
  format_note: "",
  tbr: null,
  abr: 128,
  ...overrides,
});

const videoFormat = (overrides: Partial<VideoFormat>): VideoFormat => ({
  format_id: "137",
  ext: "mp4",
  resolution: "1080p",
  height: 1080,
  width: 1920,
  fps: 30,
  vcodec: "avc1.640028",
  acodec: "none",
  filesize: null,
  filesize_approx: null,
  format_note: "",
  tbr: null,
  abr: null,
  ...overrides,
});

describe("formats utils", () => {
  it("prefers yt-dlp's original-language priority over bitrate", () => {
    const formats = [
      audioFormat({
        format_id: "251-1",
        language: "en",
        language_preference: -2,
        format_note: "English - dubbed-auto",
        abr: 160,
      }),
      audioFormat({
        format_id: "251-0",
        language: "es",
        language_preference: -1,
        format_note: "Spanish - original (default)",
        abr: 128,
      }),
    ].sort(compareAudioFormats);

    expect(formats[0].format_id).toBe("251-0");
  });

  it("uses original marker and non-DRC audio as stable fallbacks", () => {
    const formats = [
      audioFormat({ format_id: "dub", format_note: "English - dubbed", abr: 160 }),
      audioFormat({ format_id: "drc", format_note: "Spanish - original, DRC", abr: 140 }),
      audioFormat({ format_id: "original", format_note: "Spanish - original", abr: 128 }),
    ].sort(compareAudioFormats);

    expect(formats.map((format) => format.format_id)).toEqual(["original", "drc", "dub"]);
  });

  it("normalizes common yt-dlp codec identifiers", () => {
    expect(getCodecKey("avc1.640028")).toBe("h264");
    expect(getCodecLabel("av01.0.08M.08")).toBe("AV1");
    expect(getCodecLabel("mp4a.40.2")).toBe("AAC");
  });
});

describe("resolveDownloadMode", () => {
  it("merges when both tracks are selected", () => {
    expect(resolveDownloadMode("137", "251")).toBe("default");
  });

  it("keeps only the selected track", () => {
    expect(resolveDownloadMode("137", "")).toBe("video");
    expect(resolveDownloadMode("", "251")).toBe("audio");
  });

  it("falls back to yt-dlp default when nothing is selectable", () => {
    expect(resolveDownloadMode("", "")).toBe("default");
  });
});

describe("hasAudioStream", () => {
  it("separates muxed formats from video-only streams", () => {
    expect(hasAudioStream(videoFormat({ format_id: "18", acodec: "mp4a.40.2" }))).toBe(true);
    expect(hasAudioStream(videoFormat({ acodec: "none" }))).toBe(false);
    expect(hasAudioStream(videoFormat({ acodec: "" }))).toBe(false);
  });
});

describe("findSelectedVideoFormat", () => {
  const videoOnly = videoFormat({ format_id: "137" });
  const muxed = videoFormat({ format_id: "18", acodec: "mp4a.40.2", height: 360 });
  const item = { videoFormats: [videoOnly], muxedFormats: [muxed] };

  it("looks up both video-only and muxed lists", () => {
    expect(findSelectedVideoFormat({ ...item, selectedVideoFormat: "137" })?.format_id).toBe("137");
    expect(findSelectedVideoFormat({ ...item, selectedVideoFormat: "18" })?.format_id).toBe("18");
  });

  it("returns undefined for unknown or empty selections", () => {
    expect(findSelectedVideoFormat({ ...item, selectedVideoFormat: "999" })).toBeUndefined();
    expect(findSelectedVideoFormat({ ...item, selectedVideoFormat: "" })).toBeUndefined();
  });
});
