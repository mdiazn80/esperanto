import { toast } from "sonner";
import type { ErrorCode } from "@/bindings";
import { errorKey } from "@/lib/errors";
import i18n from "@/lib/i18n";

/** Logs a failed command and shows its translated error in a toast. */
export function notifyError(error: unknown) {
  console.error(error);
  toast.error(i18n.t(errorKey(error)));
}

/** Shows the translated message of an error code in a toast. */
export function notifyErrorCode(code: ErrorCode) {
  toast.error(i18n.t(`errors.${code}`));
}

/** Runs a command whose result isn't needed, reporting a failure. */
export function runCommand(command: Promise<unknown>) {
  command.catch(notifyError);
}
