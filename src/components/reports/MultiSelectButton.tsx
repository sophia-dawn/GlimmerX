import { useState } from "react";
import { useTranslation } from "react-i18next";
import { Button } from "@/components/ui/button";
import {
  Popover,
  PopoverContent,
  PopoverTrigger,
} from "@/components/ui/popover";
import {
  Command,
  CommandEmpty,
  CommandGroup,
  CommandInput,
  CommandItem,
  CommandList,
} from "@/components/ui/command";
import { Check, ChevronsUpDown } from "lucide-react";
import { cn } from "@/lib/utils";

interface Option {
  id: string;
  name: string;
}

interface MultiSelectButtonProps {
  label: string;
  options: Option[];
  selected: string[] | undefined;
  onSelect: (ids: string[] | undefined) => void;
  className?: string;
}

export function MultiSelectButton({
  label,
  options,
  selected,
  onSelect,
  className,
}: MultiSelectButtonProps) {
  const { t } = useTranslation();
  const [open, setOpen] = useState(false);
  const selectedSet = new Set(selected || []);
  const selectedCount = selectedSet.size;

  const toggleOption = (id: string) => {
    if (selectedSet.has(id)) {
      selectedSet.delete(id);
    } else {
      selectedSet.add(id);
    }
    const newIds = Array.from(selectedSet);
    onSelect(newIds.length > 0 ? newIds : undefined);
  };

  const selectAll = () => {
    onSelect(options.map((o) => o.id));
  };

  const clearAll = () => {
    onSelect(undefined);
  };

  const displayText =
    selectedCount === 0
      ? t("reports.common.allItems", { label })
      : selectedCount <= 2
        ? options
            .filter((o) => selectedSet.has(o.id))
            .map((o) => o.name)
            .join(", ")
        : t("reports.common.selectedCount", { count: selectedCount, label });

  return (
    <Popover open={open} onOpenChange={setOpen}>
      <PopoverTrigger asChild>
        <Button
          variant="outline"
          size="sm"
          role="combobox"
          aria-expanded={open}
          className={cn("w-[160px] justify-between", className)}
        >
          <span className="truncate">{displayText}</span>
          <ChevronsUpDown className="ml-2 h-4 w-4 shrink-0 opacity-50" />
        </Button>
      </PopoverTrigger>
      <PopoverContent className="w-[200px] p-0">
        <Command>
          <CommandInput
            placeholder={t("reports.common.searchItems", { label })}
          />
          <CommandList>
            <CommandEmpty>{t("reports.common.noResults")}</CommandEmpty>
            <CommandGroup>
              <div className="flex justify-between px-2 py-1 border-b">
                <Button variant="ghost" size="sm" onClick={selectAll}>
                  {t("reports.common.selectAll")}
                </Button>
                <Button variant="ghost" size="sm" onClick={clearAll}>
                  {t("reports.common.clear")}
                </Button>
              </div>
              {options.map((option) => (
                <CommandItem
                  key={option.id}
                  onSelect={() => toggleOption(option.id)}
                >
                  <Check
                    className={cn(
                      "mr-2 h-4 w-4",
                      selectedSet.has(option.id) ? "opacity-100" : "opacity-0",
                    )}
                  />
                  {option.name}
                </CommandItem>
              ))}
            </CommandGroup>
          </CommandList>
        </Command>
      </PopoverContent>
    </Popover>
  );
}
