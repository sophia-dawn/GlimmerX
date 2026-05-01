import { useState, useEffect } from "react";
import { useNavigate } from "react-router-dom";
import { useForm } from "react-hook-form";
import { useTranslation, Trans } from "react-i18next";
import { toast } from "sonner";
import {
  dbCreate,
  dbUnlock,
  dbLock,
  dbCheckExists,
  dbRemoveRecent,
  dbPing,
} from "@/utils/api";
import { translateErrorMessage } from "@/utils/errorTranslation";
import { useDbStore } from "@/stores/dbStore";
import { useLanguageStore } from "@/stores/languageStore";
import type { SupportedLanguage } from "@/i18n";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import {
  Card,
  CardContent,
  CardDescription,
  CardFooter,
  CardHeader,
  CardTitle,
} from "@/components/ui/card";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { FileWarning, Trash2 } from "lucide-react";
import { open as dialogOpen, save } from "@tauri-apps/plugin-dialog";
import { documentDir, homeDir, resolve } from "@tauri-apps/api/path";

const DEFAULT_DB_FILENAME = "glimmerx.db";

const LANGUAGE_OPTIONS: { value: SupportedLanguage; label: string }[] = [
  { value: "en", label: "English" },
  { value: "zh", label: "中文" },
];

type Mode = "select" | "create" | "open";

interface FormData {
  password: string;
  confirmPassword: string;
}

interface RecentDbEntry {
  path: string;
  label: string;
  lastOpened: string;
  exists: boolean;
}

function isValidPassword(password: string): boolean {
  return password.length >= 3;
}

// ── Page component ────────────────────────────────────────────

export function UnlockPage() {
  const { t } = useTranslation();
  const { recentDbs, loadRecentDbs, setUnlocked, isUnlocked, setLocked } =
    useDbStore();
  const { language, setLanguage } = useLanguageStore();
  const navigate = useNavigate();
  const [mode, setMode] = useState<Mode>("select");
  const [selectedPath, setSelectedPath] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);
  const [recentList, setRecentList] = useState<RecentDbEntry[]>([]);

  const {
    register,
    handleSubmit,
    watch,
    formState: { errors },
    reset,
  } = useForm<FormData>();

  useEffect(() => {
    loadRecentDbs().then(() => {
      // recentDbs from store will update, but we need local state too
    });
  }, [loadRecentDbs]);

  // Sync recentDbs from store
  useEffect(() => {
    setRecentList(recentDbs);
  }, [recentDbs]);

  const isCreateMode = mode === "create";

  const handleSelectNewDb = async () => {
    let defaultPath = DEFAULT_DB_FILENAME;
    try {
      const docDir = await documentDir();
      defaultPath = await resolve(docDir, DEFAULT_DB_FILENAME);
    } catch {
      try {
        const home = await homeDir();
        defaultPath = await resolve(home, DEFAULT_DB_FILENAME);
      } catch {
        void 0;
      }
    }
    const path = await save({
      title: t("unlock.selectDbPath"),
      filters: [{ name: "SQLite Database", extensions: ["db"] }],
      defaultPath,
    });
    if (path) {
      setSelectedPath(path as string);
      setMode("create");
      reset();
    }
  };

  const handleSelectExistingDb = async () => {
    let defaultPath: string | undefined;
    try {
      const docDir = await documentDir();
      defaultPath = docDir;
    } catch {
      try {
        defaultPath = await homeDir();
      } catch {
        void 0;
      }
    }
    const path = await dialogOpen({
      title: t("unlock.selectDbFile"),
      filters: [{ name: "SQLite Database", extensions: ["db"] }],
      multiple: false,
      defaultPath,
    });
    if (path) {
      const exists = await dbCheckExists(path as string);
      if (!exists) {
        toast.error(t("unlock.fileNotFound"));
        return;
      }
      setSelectedPath(path as string);
      setMode("open");
      reset();
    }
  };

  const handleSelectRecent = async (path: string) => {
    const exists = await dbCheckExists(path);
    if (!exists) {
      toast.error(t("unlock.ledgerNoLongerExists"));
      await dbRemoveRecent(path);
      loadRecentDbs();
      return;
    }
    setSelectedPath(path);
    setMode("open");
    reset();
  };

  const handleRemoveRecent = async (path: string, e: React.MouseEvent) => {
    e.stopPropagation();
    await dbRemoveRecent(path);
    loadRecentDbs();
    toast.success(t("unlock.removedFromRecent"));
  };

  const handleBack = () => {
    setMode("select");
    setSelectedPath(null);
    reset();
  };

  const onSubmit = async (data: FormData) => {
    if (!selectedPath) {
      toast.error(t("unlock.selectPathFirst"));
      return;
    }

    if (isCreateMode && !isValidPassword(data.password)) {
      toast.error(t("unlock.passwordMinLength"));
      return;
    }

    setLoading(true);
    try {
      // If currently unlocked, close existing database first
      if (isUnlocked) {
        console.log("[UnlockPage] closing current database before switching");
        try {
          await dbLock();
        } catch (lockError) {
          console.error(
            "[UnlockPage] failed to close current database:",
            lockError,
          );
          // Continue anyway - user wants to switch databases
        }
        setLocked();
      }

      if (isCreateMode) {
        const info = await dbCreate(selectedPath, data.password);
        await dbPing();
        setUnlocked(info.path, true);
        toast.success(t("unlock.ledgerCreated"));
        navigate("/onboarding");
      } else {
        const info = await dbUnlock(selectedPath, data.password);
        await dbPing();
        setUnlocked(info.path);
        toast.success(t("unlock.ledgerUnlocked"));
        navigate("/");
      }
    } catch (err: unknown) {
      const message = translateErrorMessage(err, t);
      toast.error(message);
      console.error("Unlock failed:", err);
    } finally {
      setLoading(false);
    }
  };

  // ── Mode: select ──────────────────────────────────────
  if (mode === "select") {
    return (
      <div className="flex h-screen w-full items-center justify-center bg-background">
        <div className="absolute right-4 top-4">
          <Select
            value={language}
            onValueChange={(v) => setLanguage(v as SupportedLanguage)}
          >
            <SelectTrigger className="w-[120px]">
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

        <Card className="w-full max-w-md">
          <CardHeader className="text-center">
            <CardTitle className="text-2xl">{t("unlock.title")}</CardTitle>
            <CardDescription>{t("unlock.selectAction")}</CardDescription>
          </CardHeader>
          <CardContent className="space-y-3">
            <Button
              className="w-full"
              onClick={handleSelectNewDb}
              variant="default"
            >
              {t("unlock.createNewLedger")}
            </Button>
            <Button
              className="w-full"
              onClick={handleSelectExistingDb}
              variant="outline"
            >
              {t("unlock.openExistingLedger")}
            </Button>
          </CardContent>
          {recentList.length > 0 && (
            <CardFooter className="flex flex-col gap-2 border-t pt-4">
              <p className="text-xs text-muted-foreground">
                {t("unlock.recentlyOpened")}
              </p>
              {recentList.map((db) => {
                const isMissing = db.exists === false;
                return (
                  <div
                    key={db.path}
                    className={`flex w-full items-center justify-between rounded-md px-2 py-1.5 text-sm ${isMissing ? "opacity-60" : "hover:bg-accent"} cursor-pointer`}
                    onClick={() => !isMissing && handleSelectRecent(db.path)}
                  >
                    <div className="flex min-w-0 flex-col">
                      <span
                        className={`truncate font-medium ${isMissing ? "line-through text-muted-foreground" : ""}`}
                      >
                        {isMissing && (
                          <FileWarning className="mr-1 inline h-3 w-3 text-destructive" />
                        )}
                        {db.label}
                      </span>
                      <span
                        className="truncate text-xs text-muted-foreground"
                        title={db.path}
                      >
                        {db.path}
                      </span>
                    </div>
                    <Button
                      variant="ghost"
                      size="xs"
                      className="shrink-0 text-destructive"
                      onClick={(e) => handleRemoveRecent(db.path, e)}
                    >
                      <Trash2 className="h-3.5 w-3.5" />
                    </Button>
                  </div>
                );
              })}
            </CardFooter>
          )}
        </Card>
      </div>
    );
  }

  // ── Mode: create or open ──────────────────────────────
  return (
    <div className="flex h-screen w-full items-center justify-center bg-background">
      <div className="absolute right-4 top-4">
        <Select
          value={language}
          onValueChange={(v) => setLanguage(v as SupportedLanguage)}
        >
          <SelectTrigger className="w-[120px]">
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

      <Card className="w-full max-w-md">
        <CardHeader className="text-center">
          <CardTitle className="text-2xl">{t("unlock.title")}</CardTitle>
          <CardDescription>
            {isCreateMode ? (
              <>
                {t("unlock.createLedger")}
                <span className="mt-1 block truncate text-xs text-muted-foreground">
                  {selectedPath}
                </span>
              </>
            ) : (
              <>
                {t("unlock.enterPasswordToUnlock")}
                <span className="mt-1 block truncate text-xs text-muted-foreground">
                  {selectedPath}
                </span>
              </>
            )}
          </CardDescription>
        </CardHeader>
        <CardContent>
          <form onSubmit={handleSubmit(onSubmit)} className="space-y-4">
            {/* Password cannot be recovered warning (create only) */}
            {isCreateMode && (
              <div className="rounded-md border border-destructive/30 bg-destructive/5 px-3 py-2.5 text-sm">
                <p className="font-medium text-destructive">
                  {t("unlock.rememberYourPassword")}
                </p>
                <p className="mt-1 text-xs text-muted-foreground">
                  <Trans
                    i18nKey="unlock.passwordWarning"
                    components={{
                      1: <span className="font-medium text-destructive" />,
                    }}
                  />
                </p>
              </div>
            )}

            {/* Password field */}
            <div className="space-y-2">
              <Label htmlFor="password">{t("unlock.password")}</Label>
              <Input
                id="password"
                type="password"
                placeholder={
                  isCreateMode
                    ? t("unlock.setEncryptionPassword")
                    : t("unlock.enterPassword")
                }
                autoComplete={
                  isCreateMode ? "new-password" : "current-password"
                }
                autoFocus={!isCreateMode}
                {...register("password", {
                  required: t("unlock.passwordRequired"),
                  minLength: {
                    value: 3,
                    message: t("unlock.passwordMinLength"),
                  },
                })}
              />
              {errors.password && (
                <p className="text-sm text-destructive">
                  {errors.password.message}
                </p>
              )}
            </div>

            {/* Confirm password (create only) */}
            {isCreateMode && (
              <div className="space-y-2">
                <Label htmlFor="confirmPassword">
                  {t("unlock.confirmPassword")}
                </Label>
                <Input
                  id="confirmPassword"
                  type="password"
                  placeholder={t("unlock.reenterPassword")}
                  autoComplete="new-password"
                  {...register("confirmPassword", {
                    required: t("unlock.confirmPasswordRequired"),
                    validate: (val) =>
                      val === watch("password") ||
                      t("unlock.passwordsDoNotMatch"),
                  })}
                />
                {errors.confirmPassword && (
                  <p className="text-sm text-destructive">
                    {errors.confirmPassword.message}
                  </p>
                )}
              </div>
            )}

            {/* Actions */}
            <div className="flex gap-2">
              <Button
                type="button"
                variant="outline"
                className="flex-1"
                onClick={handleBack}
              >
                {t("common.back")}
              </Button>
              <Button type="submit" className="flex-1" disabled={loading}>
                {loading
                  ? isCreateMode
                    ? t("unlock.creating")
                    : t("common.processing")
                  : isCreateMode
                    ? t("common.create")
                    : t("unlock.enterPasswordToUnlock")}
              </Button>
            </div>
          </form>
        </CardContent>
      </Card>
    </div>
  );
}
