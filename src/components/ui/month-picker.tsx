import { zhCN, enUS } from "date-fns/locale";
import type { Locale } from "date-fns/locale";
import { useTranslation } from "react-i18next";
import { CalendarIcon } from "lucide-react";
import { cn } from "@/lib/utils";
import { Button } from "@/components/ui/button";
import { Calendar } from "@/components/ui/calendar";
import {
  Popover,
  PopoverContent,
  PopoverTrigger,
} from "@/components/ui/popover";

const LOCALES: Record<string, Locale> = {
  en: enUS,
  zh: zhCN,
};

interface MonthPickerProps {
  value?: string;
  onChange: (month: string) => void;
  placeholder?: string;
  disabled?: boolean;
  className?: string;
  fromYear?: number;
  toYear?: number;
}

export function MonthPicker({
  value,
  onChange,
  placeholder,
  disabled = false,
  className,
  fromYear = 2020,
  toYear,
}: MonthPickerProps) {
  const { t, i18n } = useTranslation();
  const locale = LOCALES[i18n.language] ?? enUS;
  const maxYear = toYear ?? new Date().getFullYear();
  const displayPlaceholder = placeholder ?? t("common.monthPicker.placeholder");

  const selectedDate = value
    ? new Date(
        parseInt(value.split("-")[0] ?? "2020"),
        parseInt(value.split("-")[1] ?? "1") - 1,
        1,
      )
    : undefined;

  const formatMonthDisplay = (date: Date): string => {
    const year = date.getFullYear();
    const month = date.getMonth() + 1;
    return `${year} ${t(`reports.monthNames.${month}`)}`;
  };

  const handleSelect = (date: Date | undefined) => {
    if (date) {
      const year = date.getFullYear();
      const month = date.getMonth() + 1;
      onChange(`${year}-${String(month).padStart(2, "0")}`);
    }
  };

  const defaultMonth = selectedDate ?? new Date();

  return (
    <Popover>
      <PopoverTrigger asChild>
        <Button
          variant="outline"
          disabled={disabled}
          className={cn(
            "w-full justify-start text-left font-normal",
            !value && "text-muted-foreground",
            className,
          )}
        >
          <CalendarIcon className="mr-2 h-4 w-4" />
          {selectedDate ? formatMonthDisplay(selectedDate) : displayPlaceholder}
        </Button>
      </PopoverTrigger>
      <PopoverContent className="w-auto p-0" align="start">
        <Calendar
          mode="single"
          selected={selectedDate}
          onSelect={handleSelect}
          defaultMonth={defaultMonth}
          fromYear={fromYear}
          toYear={maxYear}
          captionLayout="dropdown"
          locale={locale}
        />
      </PopoverContent>
    </Popover>
  );
}
