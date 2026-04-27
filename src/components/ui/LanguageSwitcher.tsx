import { Languages } from "lucide-react";
import { useTranslation } from "react-i18next";
import { useLanguageStore } from "@/stores/languageStore";
import type { SupportedLanguage } from "@/i18n";

const LANG_CYCLE: Record<SupportedLanguage, SupportedLanguage> = {
  en: "zh",
  zh: "en",
};
const LANG_LABELS: Record<SupportedLanguage, string> = {
  en: "EN",
  zh: "中文",
};

export function LanguageSwitcher() {
  const { t } = useTranslation();
  const { language, setLanguage } = useLanguageStore();

  return (
    <button
      onClick={() => setLanguage(LANG_CYCLE[language])}
      className="flex w-full items-center gap-3 rounded-md px-3 py-2 text-sm font-medium text-muted-foreground transition-colors hover:bg-accent hover:text-accent-foreground"
      title={t("common.languageSwitch.switchTo", {
        language: LANG_LABELS[LANG_CYCLE[language]],
      })}
    >
      <Languages className="h-4 w-4" />
      {LANG_LABELS[language]}
    </button>
  );
}
