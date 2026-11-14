import { NavLink } from "react-router-dom";
import { useTranslation } from "react-i18next";
import {
  LayoutDashboard,
  ArrowLeftRight,
  PiggyBank,
  Target,
  BarChart3,
  Settings,
  Lock,
  Wallet,
} from "lucide-react";
import { lockSession } from "@/utils/session";
import { cn } from "@/lib/utils";
import { toast } from "sonner";

export function Sidebar() {
  const { t } = useTranslation();
  const handleLock = async () => {
    // The UI locks either way, but a key that refused to drop still leaves an
    // open connection behind — never close this silently.
    if (!(await lockSession())) {
      toast.error(t("common.lockFailed"));
    }
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
    { label: t("navigation.savings"), to: "/savings", icon: Target },
    { label: t("navigation.reports"), to: "/reports", icon: BarChart3 },
  ];

  return (
    // The app is a window, but a printout is a page: navigation is hidden by
    // `@media print` in globals.css, keyed on this attribute.
    <aside
      data-print="hide"
      className="flex h-screen w-52 shrink-0 flex-col border-r bg-card"
    >
      {/* Logo */}
      <div className="flex h-11 items-center border-b px-3">
        <span className="text-sm font-medium tracking-[0.02em]">
          {t("common.appName")}
        </span>
      </div>

      {/* Navigation */}
      <nav className="flex-1 space-y-0.5 p-1.5">
        {navItems.map((item) => (
          <NavLink
            key={item.to}
            to={item.to}
            end={item.to === "/"}
            className={({ isActive }) =>
              cn(
                "flex items-center gap-2 border-l-2 px-2 py-1.5 text-sm transition-colors",
                isActive
                  ? "border-signal bg-signal/10 font-medium text-signal"
                  : "border-transparent text-muted-foreground hover:bg-accent hover:text-accent-foreground",
              )
            }
          >
            <item.icon className="size-3.5" strokeWidth={1.75} />
            {item.label}
          </NavLink>
        ))}
      </nav>

      {/* Bottom section */}
      <div className="space-y-0.5 border-t p-1.5">
        <NavLink
          to="/settings"
          className={({ isActive }) =>
            cn(
              "flex items-center gap-2 border-l-2 px-2 py-1.5 text-sm transition-colors",
              isActive
                ? "border-signal bg-signal/10 font-medium text-signal"
                : "border-transparent text-muted-foreground hover:bg-accent hover:text-accent-foreground",
            )
          }
        >
          <Settings className="size-3.5" strokeWidth={1.75} />
          {t("navigation.settings")}
        </NavLink>

        {/* Lock */}
        <button
          onClick={handleLock}
          className="flex w-full items-center gap-2 border-l-2 border-transparent px-2 py-1.5 text-sm text-muted-foreground transition-colors hover:bg-accent hover:text-accent-foreground"
        >
          <Lock className="size-3.5" strokeWidth={1.75} />
          {t("navigation.lockScreen")}
        </button>

        {/* Switch Ledger */}
        <button
          onClick={handleLock}
          className="flex w-full items-center gap-2 border-l-2 border-transparent px-2 py-1.5 text-sm text-muted-foreground transition-colors hover:bg-accent hover:text-accent-foreground"
        >
          <Wallet className="size-3.5" strokeWidth={1.75} />
          {t("navigation.switchLedger")}
        </button>
      </div>
    </aside>
  );
}
