import { useState } from "react";
import { useTranslation } from "react-i18next";
import {
  useThemeStore,
  type ThemeMode,
  type ThemePreset,
} from "@/stores/themeStore";
import { useLanguageStore } from "@/stores/languageStore";
import { useSecurityStore, AUTO_LOCK_CHOICES } from "@/stores/securityStore";
import { ChangePasswordDialog } from "@/components/ChangePasswordDialog";
import { DataManagementSection } from "@/components/DataManagementSection";
import { AiSettingsSection } from "@/components/settings/AiSettingsSection";
import { AuditPanel } from "@/components/settings/AuditPanel";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import type { SupportedLanguage } from "@/i18n";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { cn } from "@/lib/utils";
import { Sun, Moon, Monitor, Check, Stethoscope } from "lucide-react";

const THEME_ITEMS: { value: ThemeMode; icon: typeof Sun }[] = [
  { value: "light", icon: Sun },
  { value: "dark", icon: Moon },
  { value: "system", icon: Monitor },
];

const PRESET_ITEMS: {
  value: ThemePreset;
  labelKey: string;
  descKey: string;
}[] = [
  {
    value: "terminal",
    labelKey: "settings.presetTerminal",
    descKey: "settings.presetDescTerminal",
  },
  {
    value: "terminal-cool",
    labelKey: "settings.presetTerminalCool",
    descKey: "settings.presetDescTerminalCool",
  },
  {
    value: "paper",
    labelKey: "settings.presetPaper",
    descKey: "settings.presetDescPaper",
  },
  {
    value: "contrast",
    labelKey: "settings.presetContrast",
    descKey: "settings.presetDescContrast",
  },
];

/**
 * A preset is previewed by *being* a preset.
 *
 * The tile carries `data-preset` and `data-mode` itself, and `.preset-preview`
 * redeclares the handful of tokens it paints with — a token resolves its `var()`
 * references where it is declared, so overriding `--hue` alone would repaint
 * nothing. The upside is that the thumbnail is the real palette, in both modes,
 * with no hardcoded hex to drift away from the theme the way the old colour
 * swatches did.
 */
function PresetPreview({ preset }: { preset: ThemePreset }) {
  return (
    <div className="grid grid-cols-2 gap-px overflow-hidden rounded-sm border bg-border">
      {(["light", "dark"] as const).map((mode) => (
        <div
          key={mode}
          data-preset={preset}
          data-mode={mode}
          className="preset-preview flex h-11 items-center gap-1.5 bg-background px-1.5"
        >
          <div className="h-6 w-8 shrink-0 rounded-xs border bg-card" />
          <div className="flex min-w-0 flex-1 flex-col gap-1">
            <span className="h-1 w-full rounded-full bg-signal" />
            <span className="h-1 w-2/3 rounded-full bg-muted-foreground/40" />
            <span className="h-1 w-1/2 rounded-full bg-muted-foreground/20" />
          </div>
        </div>
      ))}
    </div>
  );
}

const LANGUAGE_OPTIONS: { value: string; label: string }[] = [
  { value: "en", label: "English" },
  { value: "zh", label: "中文" },
];

export function SettingsPage() {
  const { t } = useTranslation();
  const { mode, setMode, preset, setPreset } = useThemeStore();
  const { language, setLanguage } = useLanguageStore();
  const { autoLockMinutes, setAutoLockMinutes } = useSecurityStore();
  const [passwordDialogOpen, setPasswordDialogOpen] = useState(false);
  const [auditOpen, setAuditOpen] = useState(false);

  return (
    <div className="h-full overflow-auto">
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
                    ? "border-signal bg-signal/5 text-foreground"
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

        {/* Preset: one look, not a hue wheel */}
        <div className="space-y-3">
          <label className="text-sm font-medium">{t("settings.preset")}</label>
          <div className="grid grid-cols-2 gap-2">
            {PRESET_ITEMS.map((item) => {
              const active = preset === item.value;
              return (
                <button
                  key={item.value}
                  type="button"
                  onClick={() => setPreset(item.value)}
                  aria-pressed={active}
                  className={cn(
                    "rounded-md border p-2 text-left transition-colors",
                    active
                      ? "border-signal bg-signal/5"
                      : "border-border hover:bg-accent",
                  )}
                >
                  <PresetPreview preset={item.value} />
                  <div className="mt-2 flex items-center justify-between gap-2">
                    <span className="text-sm font-medium">
                      {t(item.labelKey)}
                    </span>
                    {active && <Check className="size-3.5 text-signal" />}
                  </div>
                  <span className="mt-0.5 block text-xs leading-4 text-muted-foreground">
                    {t(item.descKey)}
                  </span>
                </button>
              );
            })}
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
        <div className="space-y-4">
          <label className="text-sm font-medium">
            {t("settings.security")}
          </label>

          <div className="space-y-2">
            <label className="text-sm">{t("settings.autoLock")}</label>
            <Select
              value={String(autoLockMinutes)}
              onValueChange={(v) => setAutoLockMinutes(Number(v))}
            >
              <SelectTrigger className="w-full">
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                {AUTO_LOCK_CHOICES.map((minutes) => (
                  <SelectItem key={minutes} value={String(minutes)}>
                    {minutes === 0
                      ? t("settings.autoLockOff")
                      : t("settings.autoLockMinutes", { count: minutes })}
                  </SelectItem>
                ))}
              </SelectContent>
            </Select>
            <p className="text-xs text-muted-foreground">
              {t("settings.autoLockDesc")}
            </p>
          </div>

          <button
            onClick={() => setPasswordDialogOpen(true)}
            className="flex items-center justify-center rounded-md border px-3 py-2 text-sm font-medium transition-colors hover:bg-accent"
          >
            {t("settings.changePassword")}
          </button>
        </div>

        {/* Data health. The audit is a diagnostic, not a report: it answers
            "is the ledger internally consistent", which belongs next to the
            backup and the password rather than beside the cash-flow charts. */}
        <div className="space-y-3">
          <label className="text-sm font-medium">
            {t("settings.dataAudit")}
          </label>
          <p className="text-xs text-muted-foreground">
            {t("settings.dataAuditDesc")}
          </p>
          <button
            type="button"
            onClick={() => setAuditOpen(true)}
            className="flex items-center gap-2 rounded-md border px-3 py-2 text-sm font-medium transition-colors hover:bg-accent"
          >
            <Stethoscope className="size-3.5" strokeWidth={1.75} />
            {t("settings.runAudit")}
          </button>
        </div>

        {/* AI Bookkeeping */}
        <div className="space-y-3">
          <AiSettingsSection />
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

      <Dialog open={auditOpen} onOpenChange={setAuditOpen}>
        <DialogContent className="max-h-[80vh] max-w-2xl overflow-auto">
          <DialogHeader>
            <DialogTitle>{t("reports.audit.title")}</DialogTitle>
            <DialogDescription>{t("settings.dataAuditDesc")}</DialogDescription>
          </DialogHeader>
          <AuditPanel />
        </DialogContent>
      </Dialog>
    </div>
  );
}
