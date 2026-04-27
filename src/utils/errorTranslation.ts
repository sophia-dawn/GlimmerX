import type { TFunction } from "i18next";

/**
 * Translate error messages from backend.
 * Backend returns either:
 * - Error codes prefixed with "errors." (e.g., "errors.accountIsSystem")
 * - Legacy English messages (e.g., "Account not found")
 *
 * Error codes are translated via i18next, legacy messages displayed as-is.
 */
export function translateErrorMessage(err: unknown, t: TFunction): string {
  let message: string;
  if (typeof err === "string") {
    message = err.replace(/^Validation error: /, "");
  } else if (err instanceof Error) {
    message = err.message;
  } else {
    return t("common.errorGeneric");
  }

  if (message.startsWith("errors.")) {
    const translated = t(message);
    if (translated === message) {
      return t(message.slice(7), message);
    }
    return translated;
  }

  return message;
}
