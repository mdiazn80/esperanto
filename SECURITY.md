# Security Policy

Esperanto processes audio from your microphone and your system, and it can store API tokens for remote inference servers. We take its security seriously.

## Supported versions

Esperanto has not been released yet. Once releases start, security fixes will target the **latest release** only.

| Version          | Supported   |
| ---------------- | ----------- |
| Latest release   | ✅          |
| Older releases   | ❌          |
| `develop` branch | Best effort |

## Reporting a vulnerability

**Don't report security vulnerabilities through public GitHub issues, discussions or pull requests.**

Report them privately through GitHub's [private vulnerability reporting](https://github.com/mdiazn80/esperanto/security/advisories/new) ("Report a vulnerability" in the repository's **Security** tab).

Please include:

- A description of the issue and its impact.
- Steps to reproduce, or a proof of concept.
- The affected version or commit, and your OS version.
- Any suggested fix, if you have one.

### What to expect

- Acknowledgement within **7 days**.
- An initial assessment within **14 days**.
- We'll keep you updated, agree on a disclosure date with you, and credit you in the advisory unless you prefer to stay anonymous.

Esperanto is maintained by volunteers, so timelines are best effort.

## Scope

Examples of issues we especially want to hear about:

- Audio, transcripts or API tokens leaking outside the user's machine, or to a host other than the configured provider.
- API tokens readable outside the OS keychain, or exposed in logs or to the webview.
- The local inference sidecar being reachable from other machines or usable by other local processes without its key.
- A remote profile sending audio or its token over plain HTTP to a non-local address, or to a host other than the one configured.
- Model downloads that skip checksum verification, or allow path traversal or code execution.
- Tauri capability, IPC or Content Security Policy bypasses.
- Vulnerabilities in the update or signing process.

Out of scope:

- The quality or accuracy of AI model output (transcription or translation mistakes, hallucinations). Report these as regular issues.
- Vulnerabilities in third-party remote servers that users configure themselves.
- Issues that need an already compromised machine or physical access.
