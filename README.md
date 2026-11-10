<div align="center">

<img src="./public/app-icon.svg" width="80" height="80" alt="yt-dlp GUI">

# yt-dlp GUI

A modern, beautiful desktop GUI for [yt-dlp](https://github.com/yt-dlp/yt-dlp).

Download videos from YouTube, Bilibili, Twitter/X and [1000+ websites](https://github.com/yt-dlp/yt-dlp/blob/master/supportedsites.md) with ease.

[![License](https://img.shields.io/github/license/imsyy/yt-dlp-gui?color=f0f0f0&labelColor=555555)](LICENSE)
[![Release](https://img.shields.io/github/v/release/imsyy/yt-dlp-gui?color=f0f0f0&labelColor=555555)](https://github.com/imsyy/yt-dlp-gui/releases)
[![Stars](https://img.shields.io/github/stars/imsyy/yt-dlp-gui?style=flat&color=f0f0f0&labelColor=555555)](https://github.com/imsyy/yt-dlp-gui)
[![Downloads](https://img.shields.io/github/downloads/imsyy/yt-dlp-gui/total?color=f0f0f0&labelColor=555555)](https://github.com/imsyy/yt-dlp-gui/releases)

**English** | [简体中文](./README.zh-CN.md)

</div>

---

## Why yt-dlp GUI?

yt-dlp is powerful, but its command-line interface can be intimidating. **yt-dlp GUI** wraps it in a clean, native desktop app — no terminal needed.

- **Zero config to start** — paste a link, pick a quality, click download
- **Native & lightweight** — built with Tauri 2 + Rust, ~10 MB installer, low memory usage
- **Cross-platform** — Windows, macOS, and Linux
- **Multilingual** — 7 languages with auto-detection

## Features

### Core

- Paste a video URL and instantly preview title, thumbnail, duration, and available formats
- Choose video quality, audio-only, or video-only downloads
- Download queue with pause / resume / cancel controls
- Real-time progress with speed and ETA display
- Playlist support — download all or selected items
- Configurable concurrent downloads and fragment threading

### Toolbox

- **Thumbnail Downloader** — browse and save all available cover images in any resolution
- **Subtitle Extractor** — download subtitles in SRT / VTT / ASS / LRC, with bilingual merge support
- **Live Chat Archiver** — extract YouTube live chat replay, filter with regex, export as JSON / CSV
- **Plugin Manager** — install yt-dlp plugins (e.g. ChromeCookieUnlock) with one click

### Advanced

- Custom filename templates with rich variables (title, author, date, resolution, etc.)
- Time-based clip trimming — download only a segment of the video
- Re-encode to MP4 / MKV / WebM / MP3 / AAC / FLAC and more
- Embed subtitles, thumbnails, metadata, and chapters into the output file
- SponsorBlock integration — automatically skip sponsored segments
- Multiple cookie profiles with manual Netscape-format input and automatic domain matching
- Proxy support (HTTP / SOCKS)
- Download speed limiter
- Light / Dark / Auto theme
- Download completion notifications (in-app and/or system)

## Screenshots

| Home (Dark) | Home (Light) |
|:-:|:-:|
| ![Home](screenshot/home.png) | ![Home Light](screenshot/home-light.png) |

| Download Options | Extra Options |
|:-:|:-:|
| ![Download](screenshot/download.png) | ![Download Other](screenshot/download-other.png) |

| Downloading | Tools |
|:-:|:-:|
| ![Downloading](screenshot/downloading.png) | ![Tools](screenshot/tools.png) |

## Getting Started

### Download

Grab the latest release for your platform from [**Releases**](https://github.com/imsyy/yt-dlp-gui/releases):

| Platform | File |
|----------|------|
| Windows  | `.exe` installer |
| macOS    | `.dmg` |
| Linux    | `.AppImage` / `.deb` / `.rpm` |

#### Arch Linux

Use the x86_64 AppImage from the release page. AppImage uses FUSE 2 on Arch, so install the compatibility package once, make the download executable, and run it:

```bash
sudo pacman -S fuse2
chmod +x YDL.GUI_*.AppImage
./YDL.GUI_*.AppImage
```

If FUSE cannot be enabled in your environment, run the same file with `--appimage-extract-and-run` instead. The AppImage is portable and does not require a system-wide package installation.

### First Launch

1. Open the app and go to **Settings**
2. Click **Download** next to yt-dlp — the binary is fetched automatically
3. *(Optional)* Install **Deno** runtime for full YouTube format support
4. Set your **download directory**
5. Go back to the home page, paste a URL, and start downloading

> [!TIP]
> If you encounter login-required videos, add a cookie profile in **Settings → Cookie Settings**: paste Netscape-format cookie text, and each link is matched to a profile by its domain automatically.

### Command-line automation

An installed YDL GUI instance can be opened from scripts with session-specific inputs:

```bash
yt-dlp-gui --url "https://example.com/video" \
  --cookies "/path/to/cookies.txt" \
  --dir "/path/to/downloads" \
  --yt-dlp-path "/path/to/yt-dlp" \
  --deno-path "/path/to/deno"
```

`--flag=value` is also accepted, and an HTTP(S) URL may be passed without `--url`. Tool path overrides apply only to the current app session and are never updated or replaced by the app. `--cookies` is a one-shot override for that invocation only — it never modifies your saved cookie profiles. Re-running the command forwards the request to an already-open instance.

## Tech Stack

| Layer | Technology |
|-------|-----------|
| Backend | [Tauri 2](https://tauri.app/) + [Rust](https://www.rust-lang.org/) |
| Frontend | [Vue 3](https://vuejs.org/) + [TypeScript](https://www.typescriptlang.org/) |
| UI | [Naive UI](https://www.naiveui.com/) |
| State | [Pinia](https://pinia.vuejs.org/) with persistence |
| Build | [Vite](https://vitejs.dev/) |
| i18n | [Vue I18n](https://vue-i18n.intlify.dev/) — zh-CN, zh-TW, en-US, ja-JP, ko-KR, es-ES, ru-RU |

## Development

### Prerequisites

- [Node.js](https://nodejs.org/) >= 22
- [pnpm](https://pnpm.io/) >= 11
- [Rust](https://www.rust-lang.org/tools/install)

### Setup

```bash
# Clone the repository
git clone https://github.com/imsyy/yt-dlp-gui.git
cd yt-dlp-gui

# Install dependencies
pnpm install

# Run in development mode (Vite + Tauri)
pnpm tauri:dev

# Build for production
pnpm tauri:build
```

## Contributing

Contributions are welcome! Feel free to open an [issue](https://github.com/imsyy/yt-dlp-gui/issues) or submit a pull request.

## License

[MIT](LICENSE) &copy; [imsyy](https://github.com/imsyy)
