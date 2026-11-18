<p align="center">
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset="src-tauri/icons/source/icono-dark-D1.svg" />
    <img src="src-tauri/icons/source/icono-light-L1.svg" alt="Esperanto icon" width="128" height="128" />
  </picture>
</p>

# Esperanto

**Live transcription and translation of any audio on your computer, powered by AI models that run on your own machine.**

[![License: GPL-3.0](https://img.shields.io/badge/license-GPL--3.0-blue.svg)](LICENSE)
![Status: early development](https://img.shields.io/badge/status-early%20development-orange.svg)
![Platform: macOS Apple Silicon](<https://img.shields.io/badge/platform-macOS%20(Apple%20Silicon)-lightgrey.svg>)

Esperanto listens to your microphone or to the audio your computer is playing (a meeting, a video, a podcast…) and shows a live transcript with the lowest latency possible. Pick a target language and it translates at the same time.

> [!NOTE]
> Esperanto is in **early development**. It already transcribes your microphone live and translates between English and Spanish with a local model (milestone M2). There is no packaged release yet: build it from source (see [Development](#development)).

## What works today

- 🎙️ **Live transcript of your microphone or of what your Mac is playing** (meetings, videos): partial text while someone speaks, final text a moment after each pause (under a second on an M2 Max).
- 🗣️ **Speaker labels (optional)**: each phrase is labeled with who said it, in a different color, and you can rename speakers during the session. A small voice model (30 MB) tells voices apart on your Mac.
- 🌍 **Live translation** English ↔ Spanish, switchable without stopping.
- 🔒 **Fully local**: Voxtral Mini 3B runs on your Mac through a bundled `llama-server`; no audio leaves it.
- 🗂️ **Model manager**: download (resumable, checksum-verified), pause and delete models from the app.
- 🖧 **Remote engine (optional)**: use a model behind your own API gateway (any OpenAI-compatible server) with an API token kept in the macOS Keychain. The side panel always shows whether audio stays on your Mac or goes to a remote host.
- 🕘 **History**: every session is saved on your Mac (never uploaded) so you can read it again later; it can be turned off or deleted at any time.
- 📄 **Export** a session as plain text, Markdown with times, or subtitles (SRT, WebVTT), with the original, the translation or both.
- 🔖 **Mark moments** during a session and jump back to them from the side panel.
- 🌐 **English and Spanish interface**, dark and light themes: a three-column window with the speakers, the transcript as a table (time, who, original, translation) and a side panel with the audio source, the languages and the session details.
- 🪟 **Desktop integration**: remembers the window size and position (even across monitor changes), runs as a single instance, and keeps running in the menu bar when you close its window.

## Planned features

- 🎚️ **Per-app capture** and mixing microphone + system audio.
- 💬 **Caption overlay**, history, and export to TXT / Markdown / SRT / VTT.
- 🌐 **More languages** and automatic language detection.

## How it works

```
Audio (mic / system) ─▶ Resample ─▶ Voice activity detection ─▶ AI model ─▶ Live transcript + translation
                                                                  ▲
                                         local sidecar (llama.cpp) or your remote server
```

- **Core**: Rust, [Tauri v2](https://v2.tauri.app/).
- **UI**: React, TypeScript, Vite, [shadcn/ui](https://ui.shadcn.com/), Tailwind CSS, react-i18next.
- **Inference**: an audio-capable language model that transcribes and translates in one pass (under evaluation: Voxtral, Qwen2.5-Omni, Canary…), served locally by `llama-server` or by any OpenAI-compatible server you run.

## Platforms

| Platform                                   | Status        |
| ------------------------------------------ | ------------- |
| macOS 14.4+ on Apple Silicon (M1 or newer) | First target  |
| macOS on Intel                             | Not supported |

Early builds are **not signed or notarized**: macOS will ask you to allow the app in _System Settings → Privacy & Security_ the first time you open it.
| Windows | Planned |
| Linux | Planned |

## Development

### Prerequisites

- A Mac with Apple Silicon running macOS 14.4 or later
- [Rust](https://www.rust-lang.org/tools/install) (stable)
- [Node.js](https://nodejs.org/) LTS and [pnpm](https://pnpm.io/)
- [Task](https://taskfile.dev/) (`brew install go-task`)
- Tauri's [system prerequisites](https://v2.tauri.app/start/prerequisites/) (on macOS: Xcode Command Line Tools; full Xcode 26+ is needed for release bundles to include the dark-mode app icon)
- [llama.cpp](https://github.com/ggml-org/llama.cpp) for `task dev` (`brew install llama.cpp`). Packaged builds (`task build`) bundle their own `llama-server`, downloaded by `task sidecar`.

On first launch, download the model from the welcome screen (3.2 GB). Logs are in `~/Library/Logs/com.mdiazn80.esperanto/`.

### Common tasks

Day-to-day operations live in [Taskfile.yml](Taskfile.yml). Run `task` to list them all.

| Command                      | What it does                                                                    |
| ---------------------------- | ------------------------------------------------------------------------------- |
| `task dev`                   | Run the desktop app with hot reload                                             |
| `task check`                 | Run every check (format, lint, types, tests)                                    |
| `task format`                | Format the frontend and Rust code                                               |
| `task test`                  | Run frontend and Rust tests                                                     |
| `task bindings`              | Regenerate `src/bindings.ts` from Rust commands                                 |
| `task ui:add -- <component>` | Add shadcn/ui components                                                        |
| `task build`                 | Build an unsigned release `.app` for Apple Silicon, with `llama-server` bundled |
| `task build:dmg`             | Build the `.app` and a `.dmg`                                                   |
| `task clean`                 | Remove build outputs                                                            |

Without Task, the equivalent commands are `pnpm install` then `pnpm tauri dev`; see the Taskfile for the rest.

### Recommended editor setup

[VS Code](https://code.visualstudio.com/) with the [Tauri](https://marketplace.visualstudio.com/items?itemName=tauri-apps.tauri-vscode) and [rust-analyzer](https://marketplace.visualstudio.com/items?itemName=rust-lang.rust-analyzer) extensions.

## Roadmap

1. **M0, foundations** ✅: project layout, UI shell, settings, i18n.
2. **M1, model evaluation** ✅: Voxtral Mini 3B chosen as the default model.
3. **M2, local pipeline** ✅: microphone to live transcript and translation.
4. **M3, remote provider** ✅: models behind an API gateway, with an API token.
5. **M4, system audio** ✅: capture what your Mac is playing.
6. **M5, first release**: onboarding, caption overlay, export.

## Contributing

Contributions are welcome: code, design feedback, translations, testing. Please read [CONTRIBUTING.md](CONTRIBUTING.md) and our [Code of Conduct](CODE_OF_CONDUCT.md).

To report a security vulnerability, follow [SECURITY.md](SECURITY.md). Don't open a public issue for it.

## License

Esperanto is free software, licensed under the [GNU General Public License v3.0 or later](LICENSE).

AI models are downloaded separately and keep their own licenses. The app shows each model's license before you download it.
