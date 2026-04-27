import { useState } from "react";
import { useTranslation } from "react-i18next";
import { open, save } from "@tauri-apps/plugin-dialog";
import { toast } from "sonner";
import {
  dbBackup,
  exportTransactionsCsv,
  exportTransactionsBeancount,
  importTransactionsCsv,
} from "@/utils/api";
import { Button } from "@/components/ui/button";
import { Checkbox } from "@/components/ui/checkbox";
import { Label } from "@/components/ui/label";
import { Input } from "@/components/ui/input";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { todayLocalDate, formatLocalDate } from "@/utils/date";

type DateRangePreset = "thisMonth" | "thisYear" | "allTime" | "custom";

interface DateRange {
  startDate?: string;
  endDate?: string;
}

function getDateRange(
  preset: DateRangePreset,
  customStart?: string,
  customEnd?: string,
): DateRange {
  const now = new Date();
  const currentYear = now.getFullYear();
  const currentMonth = now.getMonth();

  switch (preset) {
    case "thisMonth": {
      const start = new Date(currentYear, currentMonth, 1);
      const end = new Date(currentYear, currentMonth + 1, 0);
      return {
        startDate: formatLocalDate(start),
        endDate: formatLocalDate(end),
      };
    }
    case "thisYear": {
      return {
        startDate: `${currentYear}-01-01`,
        endDate: todayLocalDate(),
      };
    }
    case "allTime": {
      return {};
    }
    case "custom": {
      return {
        startDate: customStart,
        endDate: customEnd,
      };
    }
    default:
      return {};
  }
}

export function DataManagementSection() {
  const { t } = useTranslation();
  const [loading, setLoading] = useState(false);
  const [dateRangePreset, setDateRangePreset] =
    useState<DateRangePreset>("thisMonth");
  const [startDate, setStartDate] = useState<string>(todayLocalDate());
  const [endDate, setEndDate] = useState<string>(todayLocalDate());
  const [createMissingAccounts, setCreateMissingAccounts] = useState(false);
  const [skipDuplicates, setSkipDuplicates] = useState(true);

  const handleBackup = async () => {
    setLoading(true);
    try {
      const backupPath = await save({
        title: t("settings.backupDatabase"),
        defaultPath: `glimmerx_backup_${todayLocalDate()}.db`,
        filters: [{ name: "Database", extensions: ["db"] }],
      });

      if (backupPath) {
        await dbBackup(backupPath);
        toast.success(t("settings.backupSuccess"));
      }
    } catch (err: unknown) {
      console.error("Backup failed:", err);
      const message = err instanceof Error ? err.message : String(err);
      toast.error(message);
    } finally {
      setLoading(false);
    }
  };

  const handleExportCsv = async () => {
    setLoading(true);
    try {
      const outputPath = await save({
        title: t("settings.exportCsv"),
        defaultPath: `transactions_${todayLocalDate()}.csv`,
        filters: [{ name: "CSV", extensions: ["csv"] }],
      });

      if (outputPath) {
        const range = getDateRange(dateRangePreset, startDate, endDate);
        const result = await exportTransactionsCsv(
          outputPath,
          range.startDate,
          range.endDate,
        );
        toast.success(
          t("settings.exportSuccess", {
            count: result.transactionCount,
            postings: result.postingCount,
          }),
        );
      }
    } catch (err: unknown) {
      console.error("CSV export failed:", err);
      const message = err instanceof Error ? err.message : String(err);
      toast.error(message);
    } finally {
      setLoading(false);
    }
  };

  const handleExportBeancount = async () => {
    setLoading(true);
    try {
      const outputPath = await save({
        title: t("settings.exportBeancount"),
        defaultPath: `transactions_${todayLocalDate()}.beancount`,
        filters: [{ name: "Beancount", extensions: ["beancount"] }],
      });

      if (outputPath) {
        const range = getDateRange(dateRangePreset, startDate, endDate);
        const result = await exportTransactionsBeancount(
          outputPath,
          range.startDate,
          range.endDate,
        );
        toast.success(
          t("settings.exportSuccess", {
            count: result.transactionCount,
            postings: result.postingCount,
          }),
        );
      }
    } catch (err: unknown) {
      console.error("Beancount export failed:", err);
      const message = err instanceof Error ? err.message : String(err);
      toast.error(message);
    } finally {
      setLoading(false);
    }
  };

  const handleImport = async () => {
    if (loading) return;
    setLoading(true);

    try {
      const filePath = await open({
        title: t("settings.importCsv"),
        filters: [{ name: "CSV", extensions: ["csv"] }],
      });

      if (filePath) {
        const result = await importTransactionsCsv(filePath as string, {
          createMissingAccounts,
          skipDuplicates,
        });

        if (result.importedCount > 0) {
          toast.success(
            t("settings.importSuccess", { count: result.importedCount }),
          );
        }
        if (result.skippedCount > 0) {
          toast.info(
            t("settings.importSkipped", { count: result.skippedCount }),
          );
        }
        if (result.errorCount > 0) {
          console.error("[import] Failed transactions:", result.errors);

          const maxDisplay = 3;
          const displayErrors = result.errors.slice(0, maxDisplay);
          const hiddenCount = result.errorCount - maxDisplay;

          let errorSummary = displayErrors
            .map((e) => `${e.transactionId.slice(0, 8)}...: ${e.message}`)
            .join("\n");

          if (hiddenCount > 0) {
            errorSummary += `\n...and ${hiddenCount} more errors`;
          }

          toast.error(
            `${t("settings.importErrors", { count: result.errorCount })}\n\n${errorSummary}`,
            { duration: 10000 },
          );
        }
        if (result.createdAccounts.length > 0) {
          toast.info(
            t("settings.newAccountsCreated", {
              accounts: result.createdAccounts.join(", "),
            }),
          );
        }
      }
    } catch (err: unknown) {
      console.error("Import failed:", err);
      const message = err instanceof Error ? err.message : String(err);
      toast.error(message);
    } finally {
      setLoading(false);
    }
  };

  return (
    <div className="space-y-6">
      <div className="space-y-2">
        <Label className="text-base font-medium">
          {t("settings.dataManagement")}
        </Label>
        <p className="text-sm text-muted-foreground">
          {t("settings.backupDatabaseDesc")}
        </p>
      </div>

      <div className="space-y-2">
        <Button onClick={handleBackup} disabled={loading} variant="outline">
          {loading ? t("common.processing") : t("settings.backupDatabase")}
        </Button>
      </div>

      <div className="space-y-4">
        <div className="space-y-2">
          <Label>{t("settings.dateRange")}</Label>
          <Select
            value={dateRangePreset}
            onValueChange={(v) => setDateRangePreset(v as DateRangePreset)}
          >
            <SelectTrigger className="w-[180px]">
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              <SelectItem value="thisMonth">
                {t("settings.thisMonth")}
              </SelectItem>
              <SelectItem value="thisYear">{t("settings.thisYear")}</SelectItem>
              <SelectItem value="allTime">{t("settings.allTime")}</SelectItem>
              <SelectItem value="custom">
                {t("settings.customRange")}
              </SelectItem>
            </SelectContent>
          </Select>
        </div>

        {dateRangePreset === "custom" && (
          <div className="grid grid-cols-2 gap-4">
            <div className="space-y-2">
              <Label>{t("settings.startDate")}</Label>
              <Input
                type="date"
                value={startDate}
                onChange={(e) => setStartDate(e.target.value)}
                max={endDate}
              />
            </div>
            <div className="space-y-2">
              <Label>{t("settings.endDate")}</Label>
              <Input
                type="date"
                value={endDate}
                onChange={(e) => setEndDate(e.target.value)}
                min={startDate}
                max={todayLocalDate()}
              />
            </div>
          </div>
        )}

        <div className="flex gap-2">
          <Button
            onClick={handleExportCsv}
            disabled={loading}
            variant="outline"
          >
            {loading ? t("common.processing") : t("settings.exportCsv")}
          </Button>
          <Button
            onClick={handleExportBeancount}
            disabled={loading}
            variant="outline"
          >
            {loading ? t("common.processing") : t("settings.exportBeancount")}
          </Button>
        </div>
      </div>

      <div className="space-y-3">
        <Label>{t("settings.importCsv")}</Label>

        <div className="flex items-center space-x-2">
          <Checkbox
            id="create-missing"
            checked={createMissingAccounts}
            onCheckedChange={(checked) =>
              setCreateMissingAccounts(checked === true)
            }
          />
          <label htmlFor="create-missing" className="text-sm cursor-pointer">
            {t("settings.createMissingAccounts")}
          </label>
        </div>

        <div className="flex items-center space-x-2">
          <Checkbox
            id="skip-duplicates"
            checked={skipDuplicates}
            onCheckedChange={(checked) => setSkipDuplicates(checked === true)}
          />
          <label htmlFor="skip-duplicates" className="text-sm cursor-pointer">
            {t("settings.skipDuplicates")}
          </label>
        </div>

        <Button
          variant="outline"
          onClick={handleImport}
          disabled={loading}
          className="w-full"
        >
          {loading ? t("common.processing") : t("settings.importCsv")}
        </Button>
      </div>
    </div>
  );
}
