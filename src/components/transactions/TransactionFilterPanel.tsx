import { useState, useEffect } from "react";
import { useTranslation } from "react-i18next";
import { X } from "lucide-react";
import type { EnhancedTransactionFilter, AccountDto, Category } from "@/types";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { DatePicker } from "@/components/ui/date-picker";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";

interface TransactionFilterPanelProps {
  filter: EnhancedTransactionFilter;
  accounts: AccountDto[];
  categories: Category[];
  onClearFilter: () => void;
  onChange: (filter: Partial<EnhancedTransactionFilter>) => void;
}

const ALL_VALUE = "__all__";

export function TransactionFilterPanel({
  filter,
  accounts,
  categories,
  onClearFilter,
  onChange,
}: TransactionFilterPanelProps) {
  const { t } = useTranslation();

  const [minAmountDisplay, setMinAmountDisplay] = useState("");
  const [maxAmountDisplay, setMaxAmountDisplay] = useState("");

  const formatAmountDisplay = (cents: number | undefined) => {
    if (cents === undefined) return "";
    return (cents / 100).toFixed(2);
  };

  useEffect(() => {
    setMinAmountDisplay(formatAmountDisplay(filter.minAmount));
    setMaxAmountDisplay(formatAmountDisplay(filter.maxAmount));
  }, [filter.minAmount, filter.maxAmount]);

  const hasActiveFilters =
    filter.fromDate ||
    filter.toDate ||
    filter.minAmount !== undefined ||
    filter.maxAmount !== undefined ||
    filter.accountId ||
    filter.categoryId;

  const handleFromDateChange = (date: string) => {
    onChange({ fromDate: date || undefined });
  };

  const handleToDateChange = (date: string) => {
    onChange({ toDate: date || undefined });
  };

  const handleMinAmountBlur = () => {
    const value = minAmountDisplay.trim();
    if (value === "") {
      onChange({ minAmount: undefined });
    } else {
      const amount = parseFloat(value);
      if (!isNaN(amount) && amount >= 0) {
        onChange({ minAmount: Math.round(amount * 100) });
      }
    }
  };

  const handleMaxAmountBlur = () => {
    const value = maxAmountDisplay.trim();
    if (value === "") {
      onChange({ maxAmount: undefined });
    } else {
      const amount = parseFloat(value);
      if (!isNaN(amount) && amount >= 0) {
        onChange({ maxAmount: Math.round(amount * 100) });
      }
    }
  };

  const handleAccountChange = (value: string) => {
    onChange({ accountId: value === ALL_VALUE ? undefined : value });
  };

  const handleCategoryChange = (value: string) => {
    onChange({ categoryId: value === ALL_VALUE ? undefined : value });
  };

  return (
    <div className="px-6 py-3 border-b bg-muted/20">
      <div className="flex items-center gap-4 flex-wrap">
        <div className="flex items-center gap-2">
          <Label className="text-xs shrink-0">
            {t("transactions.filter.date")}
          </Label>
          <DatePicker
            value={filter.fromDate ?? ""}
            onChange={handleFromDateChange}
            placeholder={t("transactions.filter.from")}
            className="h-8 w-auto"
          />
          <span className="text-xs text-muted-foreground">~</span>
          <DatePicker
            value={filter.toDate ?? ""}
            onChange={handleToDateChange}
            placeholder={t("transactions.filter.to")}
            className="h-8 w-auto"
          />
        </div>

        <div className="flex items-center gap-2">
          <Label className="text-xs shrink-0">
            {t("transactions.filter.amount")}
          </Label>
          <Input
            type="number"
            step="any"
            min="0"
            placeholder={t("transactions.filter.min")}
            value={minAmountDisplay}
            onChange={(e) => setMinAmountDisplay(e.target.value)}
            onBlur={handleMinAmountBlur}
            className="h-8 w-20"
          />
          <span className="text-xs text-muted-foreground">~</span>
          <Input
            type="number"
            step="any"
            min="0"
            placeholder={t("transactions.filter.max")}
            value={maxAmountDisplay}
            onChange={(e) => setMaxAmountDisplay(e.target.value)}
            onBlur={handleMaxAmountBlur}
            className="h-8 w-20"
          />
        </div>

        <div className="flex items-center gap-2">
          <Label className="text-xs shrink-0">
            {t("transactions.filter.account")}
          </Label>
          <Select
            value={filter.accountId ?? ALL_VALUE}
            onValueChange={handleAccountChange}
          >
            <SelectTrigger size="sm" className="w-[140px]">
              <SelectValue placeholder={t("common.all")} />
            </SelectTrigger>
            <SelectContent>
              <SelectItem value={ALL_VALUE}>{t("common.all")}</SelectItem>
              {accounts.map((a) => (
                <SelectItem key={a.id} value={a.id}>
                  {a.name}
                </SelectItem>
              ))}
            </SelectContent>
          </Select>
        </div>

        <div className="flex items-center gap-2">
          <Label className="text-xs shrink-0">
            {t("transactions.filter.category")}
          </Label>
          <Select
            value={filter.categoryId ?? ALL_VALUE}
            onValueChange={handleCategoryChange}
          >
            <SelectTrigger size="sm" className="w-[140px]">
              <SelectValue placeholder={t("common.all")} />
            </SelectTrigger>
            <SelectContent>
              <SelectItem value={ALL_VALUE}>{t("common.all")}</SelectItem>
              {categories.map((c) => (
                <SelectItem key={c.id} value={c.id}>
                  {c.icon ? `${c.icon} ${c.name}` : c.name}
                </SelectItem>
              ))}
            </SelectContent>
          </Select>
        </div>

        {hasActiveFilters && (
          <Button variant="ghost" size="sm" onClick={onClearFilter}>
            <X className="h-3 w-3 mr-1" />
            {t("common.clear")}
          </Button>
        )}
      </div>
    </div>
  );
}
