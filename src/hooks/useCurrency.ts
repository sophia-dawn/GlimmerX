import { useTranslation } from "react-i18next";

export function useCurrency() {
  const { t } = useTranslation();
  return {
    symbol: t("common.currencySymbol"),
    locale: t("common.currencyLocale"),
  };
}
