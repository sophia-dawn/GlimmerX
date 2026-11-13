import type { TFunction } from "i18next";

/**
 * Backend errors are already friendly, localized messages (AGENTS.md hard
 * rule: the frontend must display them verbatim). This helper keeps its
 * historical name and signature so all call sites stay unchanged; it never
 * translates, strips or reconstructs backend text.
 */
export function translateErrorMessage(err: unknown, _t: TFunction): string {
  if (typeof err === "string") return err;
  if (err instanceof Error) return err.message;
  return _t("common.errorGeneric");
}
