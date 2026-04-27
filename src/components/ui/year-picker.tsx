import { useState, useMemo } from "react";
import { useTranslation } from "react-i18next";
import { CalendarIcon, ChevronLeft, ChevronRight } from "lucide-react";
import { cn } from "@/lib/utils";
import { Button } from "@/components/ui/button";
import {
  Popover,
  PopoverContent,
  PopoverTrigger,
} from "@/components/ui/popover";

interface YearPickerProps {
  value?: number;
  onChange: (year: number) => void;
  placeholder?: string;
  disabled?: boolean;
  className?: string;
  minYear?: number;
  maxYear?: number;
}

export function YearPicker({
  value,
  onChange,
  placeholder,
  disabled = false,
  className,
  minYear = 1900,
  maxYear = 2100,
}: YearPickerProps) {
  const { t } = useTranslation();
  const [open, setOpen] = useState(false);
  const currentYear = new Date().getFullYear();
  const [decadeStart, setDecadeStart] = useState(
    value ? Math.floor(value / 10) * 10 : Math.floor(currentYear / 10) * 10,
  );

  const displayPlaceholder = placeholder ?? t("common.yearPicker.placeholder");

  const formatYear = (y: number) =>
    t("reports.yearSummary.yearFormat", { year: y });

  const years = useMemo(() => {
    const result: number[] = [];
    for (let y = decadeStart; y < decadeStart + 10; y++) {
      if (y >= minYear && y <= maxYear) {
        result.push(y);
      }
    }
    return result;
  }, [decadeStart, minYear, maxYear]);

  const canGoPrev = decadeStart - 10 >= minYear;
  const canGoNext = decadeStart + 10 <= maxYear;

  const handlePrevDecade = (e: React.MouseEvent) => {
    e.preventDefault();
    e.stopPropagation();
    setDecadeStart((prev) => Math.max(minYear, prev - 10));
  };

  const handleNextDecade = (e: React.MouseEvent) => {
    e.preventDefault();
    e.stopPropagation();
    setDecadeStart((prev) => Math.min(maxYear - 9, prev + 10));
  };

  const handleSelect = (year: number) => {
    onChange(year);
    setOpen(false);
  };

  return (
    <Popover open={open} onOpenChange={setOpen}>
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
          {value ? formatYear(value) : displayPlaceholder}
        </Button>
      </PopoverTrigger>
      <PopoverContent className="w-auto p-3" align="start">
        <div className="space-y-3">
          <div className="flex items-center justify-between">
            <Button
              variant="ghost"
              size="icon"
              onClick={handlePrevDecade}
              disabled={!canGoPrev}
              className="h-7 w-7"
            >
              <ChevronLeft className="h-4 w-4" />
            </Button>
            <span className="font-medium text-sm">
              {decadeStart} - {decadeStart + 9}
            </span>
            <Button
              variant="ghost"
              size="icon"
              onClick={handleNextDecade}
              disabled={!canGoNext}
              className="h-7 w-7"
            >
              <ChevronRight className="h-4 w-4" />
            </Button>
          </div>

          <div className="grid grid-cols-5 gap-1">
            {years.map((year) => (
              <Button
                key={year}
                variant={value === year ? "default" : "ghost"}
                size="sm"
                onClick={() => handleSelect(year)}
                className="h-8 w-12"
              >
                {year}
              </Button>
            ))}
          </div>
        </div>
      </PopoverContent>
    </Popover>
  );
}
