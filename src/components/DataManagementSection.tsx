import { useState } from "react";
import { useTranslation } from "react-i18next";
import { save } from "@tauri-apps/plugin-dialog";
import { toast } from "sonner";
import { dbBackup } from "@/utils/api";
import { Button } from "@/components/ui/button";
import { Label } from "@/components/ui/label";
import { todayLocalDate } from "@/utils/date";

/**
 * Data management = database backup only.
 *
 * CSV / Beancount import and export were removed on 2026-11-13. A single-currency
 * hard-delete ledger has no round-trip story worth maintaining; the encrypted
 * database copy below is the only escape hatch, and it is the one that actually
 * restores everything (accounts, postings, settings, keys).
 */
export function DataManagementSection() {
  const { t } = useTranslation();
  const [loading, setLoading] = useState(false);

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
    </div>
  );
}
