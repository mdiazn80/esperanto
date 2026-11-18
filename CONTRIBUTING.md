# Contributing to Esperanto

Thanks for your interest in Esperanto! This guide explains how to propose changes and what we expect from contributions.

By participating you agree to follow our [Code of Conduct](CODE_OF_CONDUCT.md).

## Ways to contribute

- **Design feedback**: the project is in its design phase. Share it through issues or discussions.
- **Bug reports and feature requests**: open an issue. Search existing ones first.
- **Code**: pick an issue labeled `good first issue` or `help wanted`, or propose your own change.
- **Translations**: help localize the UI (see [Internationalization](#internationalization)).
- **Model evaluation**: run the benchmarks on your hardware and share the results (see the `bench:*` tasks in [Taskfile.yml](Taskfile.yml)).

Security vulnerabilities must **not** be reported in public issues. See [SECURITY.md](SECURITY.md).

## Development setup

Prerequisites:

- A Mac with Apple Silicon running macOS 14.4 or later (the only supported platform for now)
- [Rust](https://www.rust-lang.org/tools/install) (stable)
- [Node.js](https://nodejs.org/) LTS and [pnpm](https://pnpm.io/)
- Tauri's [system prerequisites](https://v2.tauri.app/start/prerequisites/)
- [Task](https://taskfile.dev/) (`brew install go-task`)
- [llama.cpp](https://github.com/ggml-org/llama.cpp) (`brew install llama.cpp`): `task dev` uses the `llama-server` on your `PATH`; `task build` bundles the official build pinned in `scripts/fetch-llama-server.sh` instead.
- An internet connection for the first build: the `sherpa-onnx` crate (speaker labels) downloads its prebuilt static library from the sherpa-onnx GitHub releases. For offline builds, point `SHERPA_ONNX_ARCHIVE_DIR` at a folder with the archive already downloaded.

```bash
git clone https://github.com/mdiazn80/esperanto.git
cd esperanto
task dev
```

Common operations are defined in [Taskfile.yml](Taskfile.yml) ([Task](https://taskfile.dev/), `brew install go-task`). Run `task` to list them. Before opening a pull request, run `task check`: it runs every check (format, lint, types, tests).

## Branches and pull requests

- `main` holds released code. `develop` is the integration branch.
- Create your branch from `develop`: `feature/<short-name>`, `fix/<short-name>` or `docs/<short-name>`.
- Open your pull request against `develop`.
- Keep pull requests focused: one logical change per PR.
- Link the issue it addresses (`Closes #<number>`).
- Fill in the PR description: what changed, why, and how you tested it. Include screenshots for UI changes.
- `task check` must pass before review.

### Commit messages

We follow [Conventional Commits](https://www.conventionalcommits.org/):

```
feat(audio): add system audio capture on macOS
fix(ui): keep auto-scroll when translation is enabled
docs(spec): describe remote provider profiles
```

Common types: `feat`, `fix`, `docs`, `refactor`, `perf`, `test`, `build`, `ci`, `chore`.

## Coding guidelines

### Rust

- Format with `cargo fmt`. Code must pass `cargo clippy -- -D warnings`.
- Never block or allocate in real-time audio callbacks.
- No user-facing strings in Rust: return error codes and let the frontend translate them.
- Never log secrets (API tokens) or audio content.

### TypeScript / React

- Run `pnpm typecheck`, `pnpm lint` and `pnpm format` (Prettier, also sorts Tailwind classes).
- Build UI with [shadcn/ui](https://ui.shadcn.com/) components (`src/components/ui`). Don't add another component library. Add components with `pnpm dlx shadcn@latest add <component>` and avoid hand-editing the generated files.
- Put UI that belongs to one area in its feature folder (`src/features/<feature>/`), components shared by several features in `src/components/`, and state in a `src/store/` zustand store.
- Report failed commands with `notifyError` / `runCommand` from `src/lib/notify.ts`, so the user sees a translated message and no promise rejection goes unhandled.
- Use the generated Tauri bindings (`src/bindings.ts`) instead of hand-written `invoke` calls. Never edit that file: after changing a command or a shared type, run `task bindings` (or `task test:rust` / `task dev`) to regenerate it and commit the result. Keep it up to date: an outdated file makes the frontend call commands with the wrong types.

### Tests

- Add or update tests for every behavior change.
- Run the whole suite before opening a PR with `task test` (Vitest + `cargo test --workspace`), or `task check` for every check.
- The locale test fails if `es` (or any locale) is missing a key that exists in `en`.

## Internationalization

- The app and docs are written in **English**, which is the source of truth for UI strings.
- UI strings live in `src/locales/<lang>/common.json` and are used through `react-i18next`. Never hard-code visible text.
- When you add or change a string, update **every** supported locale (currently `en` and `es`). If you can't translate it, copy the English text and mention it in the PR.
- To add a new UI language, open an issue first.

## Documentation

Keep the docs in sync with the code in the same pull request:

- [README.md](README.md): update it when features, setup or status change.

## AI models and third-party code

- Don't commit model weights or other large binaries. Models are listed in `models/catalog.json` and downloaded at runtime.
- Every new model in the catalog must include its license, source URL and SHA-256 checksum.
- New dependencies must have a license compatible with GPL-3.0. Mention any new dependency and its license in the PR description.

## License

By contributing, you agree that your contributions are licensed under the [GNU General Public License v3.0 or later](LICENSE), the project's license.

## Debugging tips

- After editing the app icon in Icon Composer (`src-tauri/icons/Esperanto.icon`), run `task icons` (or just `scripts/compile-app-icon.sh`) and commit the regenerated `src-tauri/icons/Assets.car`. If `actool` keeps failing with _Bad file descriptor_, quit Xcode's `ibtoold` helper (`killall ibtoold`) and run it again.

- Logs: `~/Library/Logs/com.mdiazn80.esperanto/esperanto.log` (the app) and `llama-server.log` (the model server). In development they are also printed to the terminal.
- Esperanto is single-instance and keeps running in the menu bar when its window is closed. Quit it (menu bar icon › _Quit Esperanto_, or ⌘Q) before testing a new build, or the old one will come to the front instead.
- Debug builds accept a WAV file as the microphone: set the input device to `file:/absolute/path/clip.wav` in `~/Library/Application Support/com.mdiazn80.esperanto/settings.json` (`"inputDevice"`). The file is replayed in real time through the whole pipeline.
- `cargo run -p esperanto-core --example segment_wav -- clip.wav` shows how the VAD splits a recording into phrases.
