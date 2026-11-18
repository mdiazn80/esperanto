import type { CommandError, ErrorCode } from "@/bindings";

// Keep in sync with `ErrorCode` in src-tauri/src/error.rs (the type check
// below fails if a code is missing).
const ERROR_CODES = {
  internal: true,
  settingsLoadFailed: true,
  settingsSaveFailed: true,
  microphonePermissionDenied: true,
  noInputDevice: true,
  noOutputDevice: true,
  inputDeviceNotFound: true,
  audioStreamFailed: true,
  sessionAlreadyRunning: true,
  modelNotFound: true,
  downloadInProgress: true,
  downloadFailed: true,
  downloadCancelled: true,
  modelCorrupted: true,
  insufficientDiskSpace: true,
  modelDeleteFailed: true,
  modelNotInstalled: true,
  llamaServerNotFound: true,
  llamaServerFailed: true,
  inferenceFailed: true,
  providerNotFound: true,
  providerInvalid: true,
  providerInsecureUrl: true,
  providerTokenMissing: true,
  providerUnreachable: true,
  providerUnauthorized: true,
  providerRejected: true,
  keychainFailed: true,
  historyNotFound: true,
  historyFailed: true,
  exportFailed: true,
} satisfies Record<ErrorCode, true>;

function isCommandError(error: unknown): error is CommandError {
  return (
    typeof error === "object" &&
    error !== null &&
    "code" in error &&
    typeof error.code === "string" &&
    error.code in ERROR_CODES
  );
}

/** The error code of a failed Tauri command, if it has a known one. */
export function errorCode(error: unknown): ErrorCode | null {
  return isCommandError(error) ? error.code : null;
}

/** Translation key for an error returned by a Tauri command. */
export function errorKey(error: unknown): `errors.${ErrorCode | "unknown"}` {
  return isCommandError(error) ? `errors.${error.code}` : "errors.unknown";
}
