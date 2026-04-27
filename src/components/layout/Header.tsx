import { useTranslation } from "react-i18next";
import { useLocation } from "react-router-dom";
import { Plus } from "lucide-react";
import { Button } from "@/components/ui/button";
import { useHeaderActions } from "@/contexts/HeaderContext";

interface HeaderProps {
  onQuickAdd?: () => void;
}

export function Header({ onQuickAdd }: HeaderProps) {
  const { t } = useTranslation();
  const location = useLocation();
  const { actions } = useHeaderActions();

  const routeTitles: Record<string, string> = {
    "/": t("navigation.overview"),
    "/accounts": t("navigation.accounts"),
    "/transactions": t("navigation.transactions"),
    "/categories": t("navigation.categories"),
    "/budgets": t("navigation.budgets"),
    "/reports": t("navigation.reports"),
    "/settings": t("navigation.settings"),
  };

  const title = routeTitles[location.pathname] ?? "GlimmerX";

  return (
    <header className="flex h-14 items-center justify-between border-b bg-background px-6">
      <h1 className="text-lg font-semibold">{title}</h1>
      <div className="flex items-center gap-2">
        {actions}
        {onQuickAdd && (
          <Button
            variant="default"
            size="sm"
            onClick={onQuickAdd}
            className="gap-1"
          >
            <Plus className="h-4 w-4" />
            {t("quickAdd.title")}
          </Button>
        )}
      </div>
    </header>
  );
}
