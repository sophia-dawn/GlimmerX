import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { REPORT_LIST } from "@/constants/report";
import { useTranslation } from "react-i18next";

interface ReportTypeSelectorProps {
  selected: string;
  onSelect: (id: string) => void;
}

export function ReportTypeSelector({
  selected,
  onSelect,
}: ReportTypeSelectorProps) {
  const { t } = useTranslation();

  return (
    <Select value={selected} onValueChange={onSelect}>
      <SelectTrigger className="w-[180px]">
        <SelectValue />
      </SelectTrigger>
      <SelectContent>
        {REPORT_LIST.map((item) => (
          <SelectItem key={item.id} value={item.id}>
            {t(item.nameKey)}
          </SelectItem>
        ))}
      </SelectContent>
    </Select>
  );
}
