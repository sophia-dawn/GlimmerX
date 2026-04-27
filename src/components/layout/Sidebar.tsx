import { NavLink } from "react-router-dom";
import { useTranslation } from "react-i18next";
import {
  LayoutDashboard,
  ArrowLeftRight,
  PiggyBank,
  BarChart3,
  Settings,
  Lock,
  Wallet,
} from "lucide-react";
import { useDbStore } from "@/stores/dbStore";
import { dbLock } from "@/utils/api";
import { cn } from "@/lib/utils";

export function Sidebar() {
  const { t } = useTranslation();
  const { setLocked } = useDbStore();

  const handleLock = async () => {
    try {
      await dbLock();
    } catch {
      // ignore
    }
    setLocked();
  };

  const navItems = [
    { label: t("navigation.overview"), to: "/", icon: LayoutDashboard },
    { label: t("navigation.accounts"), to: "/accounts", icon: Wallet },
    {
      label: t("navigation.transactions"),
      to: "/transactions",
      icon: ArrowLeftRight,
    },
    { label: t("navigation.categories"), to: "/categories", icon: PiggyBank },
    { label: t("navigation.budgets"), to: "/budgets", icon: BarChart3 },
    { label: t("navigation.reports"), to: "/reports", icon: BarChart3 },
  ];

  return (
    <aside className="flex h-screen w-56 flex-col border-r bg-background">
      {/* Logo */}
      <div className="flex h-14 items-center border-b px-4">
        <span className="text-lg font-bold tracking-tight">
          {t("common.appName")}
        </span>
      </div>

      {/* Navigation */}
      <nav className="flex-1 space-y-1 p-2">
        {navItems.map((item) => (
          <NavLink
            key={item.to}
            to={item.to}
            end={item.to === "/"}
            className={({ isActive }) =>
              cn(
                "flex items-center gap-3 rounded-md px-3 py-2 text-sm font-medium transition-colors",
                isActive
                  ? "bg-primary text-primary-foreground"
                  : "text-muted-foreground hover:bg-accent hover:text-accent-foreground",
              )
            }
          >
            <item.icon className="h-4 w-4" />
            {item.label}
          </NavLink>
        ))}
      </nav>

      {/* Bottom section */}
      <div className="space-y-1 border-t p-2">
        <NavLink
          to="/settings"
          className={({ isActive }) =>
            cn(
              "flex items-center gap-3 rounded-md px-3 py-2 text-sm font-medium transition-colors",
              isActive
                ? "bg-primary text-primary-foreground"
                : "text-muted-foreground hover:bg-accent hover:text-accent-foreground",
            )
          }
        >
          <Settings className="h-4 w-4" />
          {t("navigation.settings")}
        </NavLink>

        {/* Lock */}
        <button
          onClick={handleLock}
          className="flex w-full items-center gap-3 rounded-md px-3 py-2 text-sm font-medium text-muted-foreground transition-colors hover:bg-accent hover:text-accent-foreground"
        >
          <Lock className="h-4 w-4" />
          {t("navigation.lockScreen")}
        </button>

        {/* Switch Ledger */}
        <button
          onClick={handleLock}
          className="flex w-full items-center gap-3 rounded-md px-3 py-2 text-sm font-medium text-muted-foreground transition-colors hover:bg-accent hover:text-accent-foreground"
        >
          <Wallet className="h-4 w-4" />
          {t("navigation.switchLedger")}
        </button>
      </div>
    </aside>
  );
}
