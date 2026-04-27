import { format, type Locale } from "date-fns";
import { zhCN, enUS } from "date-fns/locale";
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

interface DatePickerProps {
  value?: string;
  onChange: (date: string) => void;
  placeholder?: string;
  disabled?: boolean;
  className?: string;
  maxDate?: Date;
}

export function DatePicker({
  value,
  onChange,
  placeholder,
  disabled = false,
  className,
  maxDate = new Date(),
}: DatePickerProps) {
  const { t, i18n } = useTranslation();
  const locale = LOCALES[i18n.language] ?? enUS;
  const selectedDate = value ? new Date(value) : undefined;
  const displayPlaceholder = placeholder ?? t("common.datePicker.placeholder");

  const handleSelect = (date: Date | undefined) => {
    if (date) {
      onChange(format(date, "yyyy-MM-dd"));
    }
  };

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
          {value
            ? format(selectedDate!, "PPP", { locale })
            : displayPlaceholder}
        </Button>
      </PopoverTrigger>
      <PopoverContent className="w-auto p-0" align="start">
        <Calendar
          mode="single"
          selected={selectedDate}
          onSelect={handleSelect}
          disabled={(date) => date > maxDate}
          initialFocus
          locale={locale}
        />
      </PopoverContent>
    </Popover>
  );
}
