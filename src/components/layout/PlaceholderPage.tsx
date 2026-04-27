import { useTranslation } from "react-i18next";

export function PlaceholderPage({ title }: { title: string }) {
  const { t } = useTranslation();
  return (
    <div className="flex h-full items-center justify-center">
      <p className="text-muted-foreground">
        {t("placeholder.inDevelopment", { title })}
      </p>
    </div>
  );
}
