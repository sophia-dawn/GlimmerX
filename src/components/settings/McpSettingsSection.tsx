import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { mcpGetConfig, mcpResetToken, mcpSetConfig } from "@/utils/api";
import { translateErrorMessage } from "@/utils/errorTranslation";
import type { McpTrustMode } from "@/types";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Switch } from "@/components/ui/switch";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { Loader2, Copy, RefreshCw, Check } from "lucide-react";

const TRUST_MODES: { value: McpTrustMode; labelKey: string }[] = [
  { value: "readonly", labelKey: "mcp.trustReadonly" },
  { value: "confirm", labelKey: "mcp.trustConfirm" },
  { value: "trust", labelKey: "mcp.trustTrust" },
];

/**
 * The "automation" section of the settings page (design doc 25, §8.1).
 *
 * The MCP server exposes the ledger to external agents over loopback HTTP
 * with a Bearer token, so every control here is a security decision: the
 * toggle, the port, the trust tier, and the token itself.
 */
export function McpSettingsSection() {
  const { t } = useTranslation();
  const [enabled, setEnabled] = useState(false);
  const [port, setPort] = useState("17823");
  const [actualPort, setActualPort] = useState<number | null>(null);
  const [running, setRunning] = useState(false);
  const [trust, setTrust] = useState<McpTrustMode>("confirm");
  const [token, setToken] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [saveStatus, setSaveStatus] = useState<"idle" | "saving" | "saved">(
    "idle",
  );
  const [copied, setCopied] = useState<"none" | "token" | "config">("none");

  // Load once on mount. The `t` from useTranslation is not referentially
  // stable across renders, so a `[t]` dep would re-run the fetch after every
  // state change and clobber the user's edits with the stored config.
  useEffect(() => {
    let alive = true;
    (async () => {
      try {
        const config = await mcpGetConfig();
        if (!alive) return;
        setEnabled(config.enabled);
        setPort(String(config.port));
        setActualPort(config.actualPort);
        setRunning(config.running);
        setTrust(config.trustMode);
        setToken(config.token);
      } catch (err) {
        if (alive) setError(translateErrorMessage(err, t));
      }
    })();
    return () => {
      alive = false;
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  const handleSave = async (patch: {
    enabled?: boolean;
    port?: number;
    trust?: McpTrustMode;
  }) => {
    setSaveStatus("saving");
    setError(null);
    try {
      const config = await mcpSetConfig(patch);
      setEnabled(config.enabled);
      setPort(String(config.port));
      setActualPort(config.actualPort);
      setRunning(config.running);
      setTrust(config.trustMode);
      setSaveStatus("saved");
      setTimeout(() => setSaveStatus("idle"), 2000);
    } catch (err) {
      setError(translateErrorMessage(err, t));
      setSaveStatus("idle");
    }
  };

  const handleResetToken = async () => {
    setError(null);
    try {
      const config = await mcpResetToken();
      setToken(config.token);
      setActualPort(config.actualPort);
      setRunning(config.running);
    } catch (err) {
      setError(translateErrorMessage(err, t));
    }
  };

  const copy = async (text: string, what: "token" | "config") => {
    try {
      await navigator.clipboard.writeText(text);
      setCopied(what);
      setTimeout(() => setCopied("none"), 2000);
    } catch {
      // Clipboard access can be denied; the field remains selectable.
    }
  };

  const mcpConfigJson = JSON.stringify(
    {
      mcpServers: {
        glimmerx: {
          type: "http",
          url: `http://127.0.0.1:${actualPort ?? port}/mcp`,
          headers: { Authorization: `Bearer ${token}` },
        },
      },
    },
    null,
    2,
  );

  const portNumber = Number(port);
  const portValid =
    Number.isInteger(portNumber) && portNumber > 0 && portNumber <= 65535;

  return (
    <>
      <label className="text-sm font-medium">{t("mcp.title")}</label>
      <div className="space-y-4 rounded-md border p-4">
        <div className="flex items-center justify-between">
          <div className="space-y-1">
            <Label>{t("mcp.enable")}</Label>
            <p className="text-xs text-muted-foreground">
              {running
                ? t("mcp.runningOn", { port: actualPort ?? port })
                : t("mcp.notRunning")}
            </p>
          </div>
          <Switch
            checked={enabled}
            onCheckedChange={(checked) => {
              setEnabled(checked);
              handleSave({ enabled: checked });
            }}
          />
        </div>

        <div className="space-y-2">
          <Label>{t("mcp.port")}</Label>
          <div className="flex gap-2">
            <Input
              value={port}
              onChange={(e) => setPort(e.target.value)}
              inputMode="numeric"
              className="font-mono"
            />
            <Button
              type="button"
              variant="outline"
              size="sm"
              disabled={!portValid || saveStatus === "saving"}
              onClick={() => handleSave({ port: portNumber })}
            >
              {saveStatus === "saving" && (
                <Loader2 className="h-4 w-4 animate-spin" />
              )}
              {saveStatus === "saved" && <Check className="h-4 w-4" />}
              {t("mcp.applyPort")}
            </Button>
          </div>
        </div>

        <div className="space-y-2">
          <Label>{t("mcp.trustMode")}</Label>
          <Select
            value={trust}
            onValueChange={(value: McpTrustMode) => {
              setTrust(value);
              handleSave({ trust: value });
            }}
          >
            <SelectTrigger className="w-full">
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              {TRUST_MODES.map((mode) => (
                <SelectItem key={mode.value} value={mode.value}>
                  {t(mode.labelKey)}
                </SelectItem>
              ))}
            </SelectContent>
          </Select>
          <p className="text-xs text-muted-foreground">{t("mcp.trustHint")}</p>
        </div>

        <div className="space-y-2">
          <Label>{t("mcp.token")}</Label>
          <div className="flex gap-2">
            <Input
              value={token}
              readOnly
              className="font-mono text-xs"
              onFocus={(e) => e.target.select()}
            />
            <Button
              type="button"
              variant="outline"
              size="sm"
              onClick={() => copy(token, "token")}
            >
              {copied === "token" ? (
                <Check className="h-4 w-4" />
              ) : (
                <Copy className="h-4 w-4" />
              )}
            </Button>
            <Button
              type="button"
              variant="outline"
              size="sm"
              onClick={handleResetToken}
              title={t("mcp.resetToken")}
            >
              <RefreshCw className="h-4 w-4" />
            </Button>
          </div>
        </div>

        <div className="space-y-2">
          <Label>{t("mcp.clientConfig")}</Label>
          <pre className="overflow-x-auto rounded-sm border bg-muted/50 p-3 font-mono text-xs leading-relaxed">
            {mcpConfigJson}
          </pre>
          <Button
            type="button"
            variant="outline"
            size="sm"
            onClick={() => copy(mcpConfigJson, "config")}
          >
            {copied === "config" ? (
              <Check className="h-4 w-4" />
            ) : (
              <Copy className="h-4 w-4" />
            )}
            {t("mcp.copyConfig")}
          </Button>
        </div>

        {error && <p className="text-sm text-destructive">{error}</p>}
      </div>
    </>
  );
}
