import { useState } from "react";
import { useTranslation } from "react-i18next";
import {
  useThemeStore,
  type ThemeMode,
  type AccentColor,
  type AtmosphericTheme,
} from "@/stores/themeStore";
import { useLanguageStore } from "@/stores/languageStore";
import { ChangePasswordDialog } from "@/components/ChangePasswordDialog";
import { DataManagementSection } from "@/components/DataManagementSection";
import type { SupportedLanguage } from "@/i18n";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { cn } from "@/lib/utils";
import { Sun, Moon, Monitor } from "lucide-react";

const THEME_ITEMS: { value: ThemeMode; icon: typeof Sun }[] = [
  { value: "light", icon: Sun },
  { value: "dark", icon: Moon },
  { value: "system", icon: Monitor },
];

const COLOR_ITEMS: {
  value: AccentColor;
  labelKey: string;
  colorValue: string;
}[] = [
  {
    value: "neutral",
    labelKey: "settings.colorNeutral",
    colorValue: "#404040",
  },
  { value: "blue", labelKey: "settings.colorBlue", colorValue: "#3b82f6" },
  { value: "green", labelKey: "settings.colorGreen", colorValue: "#22c55e" },
  { value: "purple", labelKey: "settings.colorPurple", colorValue: "#a855f7" },
  { value: "orange", labelKey: "settings.colorOrange", colorValue: "#f97316" },
  { value: "rose", labelKey: "settings.colorRose", colorValue: "#f43f5e" },
  { value: "red", labelKey: "settings.colorRed", colorValue: "#ef4444" },
  { value: "amber", labelKey: "settings.colorAmber", colorValue: "#f59e0b" },
  { value: "yellow", labelKey: "settings.colorYellow", colorValue: "#eab308" },
  { value: "lime", labelKey: "settings.colorLime", colorValue: "#84cc16" },
  {
    value: "emerald",
    labelKey: "settings.colorEmerald",
    colorValue: "#10b981",
  },
  { value: "teal", labelKey: "settings.colorTeal", colorValue: "#14b8a6" },
  { value: "cyan", labelKey: "settings.colorCyan", colorValue: "#06b6d4" },
  { value: "sky", labelKey: "settings.colorSky", colorValue: "#0ea5e9" },
  { value: "indigo", labelKey: "settings.colorIndigo", colorValue: "#6366f1" },
  { value: "violet", labelKey: "settings.colorViolet", colorValue: "#8b5cf6" },
  {
    value: "fuchsia",
    labelKey: "settings.colorFuchsia",
    colorValue: "#d946ef",
  },
  { value: "pink", labelKey: "settings.colorPink", colorValue: "#ec4899" },
];

const ATMOSPHERE_ITEMS: {
  value: AtmosphericTheme;
  labelKey: string;
  descKey: string;
}[] = [
  {
    value: "none",
    labelKey: "settings.atmosphereNone",
    descKey: "settings.atmosphereDescNone",
  },
  {
    value: "warm",
    labelKey: "settings.atmosphereWarm",
    descKey: "settings.atmosphereDescWarm",
  },
  {
    value: "cool",
    labelKey: "settings.atmosphereCool",
    descKey: "settings.atmosphereDescCool",
  },
  {
    value: "ocean",
    labelKey: "settings.atmosphereOcean",
    descKey: "settings.atmosphereDescOcean",
  },
  {
    value: "forest",
    labelKey: "settings.atmosphereForest",
    descKey: "settings.atmosphereDescForest",
  },
  {
    value: "midnight",
    labelKey: "settings.atmosphereMidnight",
    descKey: "settings.atmosphereDescMidnight",
  },
  {
    value: "rose-atmosphere",
    labelKey: "settings.atmosphereRose",
    descKey: "settings.atmosphereDescRose",
  },
];

const LANGUAGE_OPTIONS: { value: string; label: string }[] = [
  { value: "en", label: "English" },
  { value: "zh", label: "中文" },
];

export function SettingsPage() {
  const { t } = useTranslation();
  const { mode, setMode, color, setColor, atmosphere, setAtmosphere } =
    useThemeStore();
  const { language, setLanguage } = useLanguageStore();
  const [passwordDialogOpen, setPasswordDialogOpen] = useState(false);

  return (
    <div className="h-full overflow-auto p-6">
      <div className="max-w-md space-y-8">
        {/* Theme */}
        <div className="space-y-3">
          <label className="text-sm font-medium">{t("settings.theme")}</label>
          <div className="flex gap-2">
            {THEME_ITEMS.map((item) => (
              <button
                key={item.value}
                onClick={() => setMode(item.value)}
                className={cn(
                  "flex flex-1 items-center justify-center gap-1.5 rounded-md border px-3 py-2 text-sm font-medium transition-colors",
                  mode === item.value
                    ? "border-primary bg-primary/10 text-primary"
                    : "text-muted-foreground hover:bg-accent",
                )}
              >
                <item.icon className="h-4 w-4" />
                {t(
                  `navigation.theme${item.value.charAt(0).toUpperCase() + item.value.slice(1)}`,
                )}
              </button>
            ))}
          </div>
        </div>

        {/* Color */}
        <div className="space-y-3">
          <label className="text-sm font-medium">{t("settings.color")}</label>
          <div className="grid grid-cols-6 gap-2">
            {COLOR_ITEMS.map((item) => (
              <button
                key={item.value}
                onClick={() => setColor(item.value)}
                className={cn(
                  "flex h-8 w-8 items-center justify-center rounded-full border-2 transition-all",
                  color === item.value
                    ? "border-primary scale-110 ring-2 ring-primary/30"
                    : "border-muted hover:border-primary/50",
                )}
                title={t(item.labelKey)}
                aria-label={t(item.labelKey)}
              >
                <span
                  className="h-5 w-5 rounded-full"
                  style={{ backgroundColor: item.colorValue }}
                />
              </button>
            ))}
          </div>
        </div>

        {/* Atmosphere */}
        <div className="space-y-3">
          <label className="text-sm font-medium">
            {t("settings.atmosphere")}
          </label>
          <div className="grid grid-cols-3 gap-3">
            {ATMOSPHERE_ITEMS.map((item) => (
              <button
                key={item.value}
                onClick={() => setAtmosphere(item.value)}
                className={cn(
                  "flex flex-col items-center justify-center rounded-md border p-3 text-sm transition-all",
                  atmosphere === item.value
                    ? "border-primary bg-primary/10"
                    : "border-muted hover:border-primary/50 hover:bg-accent",
                )}
              >
                <span className="font-medium">{t(item.labelKey)}</span>
                <span className="text-xs text-muted-foreground">
                  {t(item.descKey)}
                </span>
              </button>
            ))}
          </div>
        </div>

        {/* Language */}
        <div className="space-y-3">
          <label className="text-sm font-medium">
            {t("settings.language")}
          </label>
          <Select
            value={language}
            onValueChange={(v) => setLanguage(v as SupportedLanguage)}
          >
            <SelectTrigger className="w-full">
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              {LANGUAGE_OPTIONS.map((opt) => (
                <SelectItem key={opt.value} value={opt.value}>
                  {opt.label}
                </SelectItem>
              ))}
            </SelectContent>
          </Select>
        </div>

        {/* Security */}
        <div className="space-y-3">
          <label className="text-sm font-medium">
            {t("settings.security")}
          </label>
          <button
            onClick={() => setPasswordDialogOpen(true)}
            className="flex items-center justify-center rounded-md border px-3 py-2 text-sm font-medium transition-colors hover:bg-accent"
          >
            {t("settings.changePassword")}
          </button>
        </div>

        {/* Data Management */}
        <div className="space-y-3">
          <DataManagementSection />
        </div>
      </div>

      <ChangePasswordDialog
        open={passwordDialogOpen}
        onOpenChange={setPasswordDialogOpen}
      />
    </div>
  );
}
