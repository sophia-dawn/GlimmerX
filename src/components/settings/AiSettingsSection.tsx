import { useState, useEffect } from "react";
import { useTranslation } from "react-i18next";
import { useQuery } from "@tanstack/react-query";
import {
  accountList,
  getSetting,
  setSetting,
  aiTestConnection,
} from "@/utils/api";
import { translateErrorMessage } from "@/utils/errorTranslation";
import { QUERY_CONFIG } from "@/constants/query";
import type { AiProvider } from "@/types";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { Loader2, Check } from "lucide-react";

const PROVIDER_DEFAULTS: Record<AiProvider, string> = {
  openai: "https://api.openai.com/v1",
  deepseek: "https://api.deepseek.com/v1",
  ollama: "http://localhost:11434/v1",
  custom: "",
};

const PROVIDERS: { value: AiProvider; labelKey: string }[] = [
  { value: "openai", labelKey: "ai.settings.providerOpenai" },
  { value: "deepseek", labelKey: "ai.settings.providerDeepseek" },
  { value: "ollama", labelKey: "ai.settings.providerOllama" },
  { value: "custom", labelKey: "ai.settings.providerCustom" },
];

export function AiSettingsSection() {
  const { t } = useTranslation();
  const [provider, setProvider] = useState<AiProvider>("openai");
  const [baseUrl, setBaseUrl] = useState("");
  const [apiKey, setApiKey] = useState("");
  const [model, setModel] = useState("");
  const [defaultSourceAccountId, setDefaultSourceAccountId] = useState("");
  const [showKey, setShowKey] = useState(false);
  const [saveStatus, setSaveStatus] = useState<"idle" | "saving" | "saved">(
    "idle",
  );
  const [testStatus, setTestStatus] = useState<
    "idle" | "testing" | "success" | "error"
  >("idle");
  const [error, setError] = useState<string | null>(null);

  const { data: accounts = [] } = useQuery({
    queryKey: ["accounts"],
    queryFn: accountList,
    ...QUERY_CONFIG.FINANCIAL,
  });

  const assetAccounts = accounts.filter(
    (a) => a.account_type === "asset" && a.is_active,
  );

  useEffect(() => {
    (async () => {
      try {
        const [p, url, key, m, d] = await Promise.all([
          getSetting("ai.provider"),
          getSetting("ai.base_url"),
          getSetting("ai.api_key"),
          getSetting("ai.model"),
          getSetting("ai.default_source_account_id"),
        ]);
        if (p) setProvider(p as AiProvider);
        if (url) setBaseUrl(url);
        if (key) setApiKey(key);
        if (m) setModel(m);
        if (d) setDefaultSourceAccountId(d);
      } catch (err) {
        setError(translateErrorMessage(err, t));
      }
    })();
  }, [t]);

  const handleProviderChange = (value: AiProvider) => {
    setProvider(value);
    setBaseUrl(PROVIDER_DEFAULTS[value]);
  };

  const handleSave = async (): Promise<boolean> => {
    setSaveStatus("saving");
    setError(null);
    try {
      await Promise.all([
        setSetting("ai.provider", provider),
        setSetting("ai.base_url", baseUrl),
        setSetting("ai.api_key", apiKey),
        setSetting("ai.model", model),
        setSetting("ai.default_source_account_id", defaultSourceAccountId),
      ]);
      setSaveStatus("saved");
      setTimeout(() => setSaveStatus("idle"), 2000);
      return true;
    } catch (err) {
      setError(translateErrorMessage(err, t));
      setSaveStatus("idle");
      return false;
    }
  };

  const handleTest = async () => {
    const savedOk = await handleSave();
    if (!savedOk) return;
    setTestStatus("testing");
    setError(null);
    try {
      await aiTestConnection();
      setTestStatus("success");
      setTimeout(() => setTestStatus("idle"), 3000);
    } catch (err) {
      setError(translateErrorMessage(err, t));
      setTestStatus("error");
    }
  };

  return (
    <>
      <label className="text-sm font-medium">{t("ai.settings.title")}</label>
      <div className="space-y-4 rounded-md border p-4">
        <div className="space-y-2">
          <Label>{t("ai.settings.provider")}</Label>
          <Select value={provider} onValueChange={handleProviderChange}>
            <SelectTrigger className="w-full">
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              {PROVIDERS.map((p) => (
                <SelectItem key={p.value} value={p.value}>
                  {t(p.labelKey)}
                </SelectItem>
              ))}
            </SelectContent>
          </Select>
        </div>

        <div className="space-y-2">
          <Label>{t("ai.settings.baseUrl")}</Label>
          <Input
            value={baseUrl}
            onChange={(e) => setBaseUrl(e.target.value)}
            placeholder="https://api.openai.com/v1"
          />
        </div>

        <div className="space-y-2">
          <Label>{t("ai.settings.apiKey")}</Label>
          <div className="flex gap-2">
            <Input
              type={showKey ? "text" : "password"}
              value={apiKey}
              onChange={(e) => setApiKey(e.target.value)}
              placeholder="sk-..."
            />
            <Button
              type="button"
              variant="outline"
              size="sm"
              onClick={() => setShowKey(!showKey)}
            >
              {showKey ? t("ai.settings.hideKey") : t("ai.settings.showKey")}
            </Button>
          </div>
        </div>

        <div className="space-y-2">
          <Label>{t("ai.settings.model")}</Label>
          <Input
            value={model}
            onChange={(e) => setModel(e.target.value)}
            placeholder="gpt-4o-mini"
          />
        </div>

        <div className="space-y-2">
          <Label>{t("ai.settings.defaultSourceAccount")}</Label>
          <Select
            value={defaultSourceAccountId || undefined}
            onValueChange={setDefaultSourceAccountId}
          >
            <SelectTrigger className="w-full">
              <SelectValue placeholder={t("transactions.selectAccount")} />
            </SelectTrigger>
            <SelectContent>
              {assetAccounts.map((account) => (
                <SelectItem key={account.id} value={account.id}>
                  {account.name}
                </SelectItem>
              ))}
            </SelectContent>
          </Select>
        </div>

        {error && <p className="text-sm text-destructive">{error}</p>}

        <div className="flex gap-2">
          <Button
            type="button"
            variant="outline"
            size="sm"
            onClick={handleTest}
            disabled={testStatus === "testing"}
          >
            {testStatus === "testing" && (
              <Loader2 className="h-4 w-4 animate-spin" />
            )}
            {testStatus === "success" && <Check className="h-4 w-4" />}
            {testStatus === "testing"
              ? t("ai.recognizing")
              : testStatus === "success"
                ? t("ai.settings.testSuccess")
                : t("ai.settings.testConnection")}
          </Button>
          <Button
            type="button"
            size="sm"
            onClick={handleSave}
            disabled={saveStatus === "saving"}
          >
            {saveStatus === "saving" && (
              <Loader2 className="h-4 w-4 animate-spin" />
            )}
            {saveStatus === "saved" && <Check className="h-4 w-4" />}
            {t("ai.settings.save")}
          </Button>
        </div>
      </div>
    </>
  );
}
