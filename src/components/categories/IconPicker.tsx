import { useState } from "react";
import { useTranslation } from "react-i18next";
import { Button } from "@/components/ui/button";
import {
  Popover,
  PopoverContent,
  PopoverTrigger,
} from "@/components/ui/popover";
import { X } from "lucide-react";

const ICON_CATEGORIES: Record<string, string[]> = {
  food: [
    "🍕", // 外卖/披萨
    "🍔", // 快餐
    "🍜", // 面食
    "🍣", // 日料
    "☕", // 咖啡/茶饮
    "🍺", // 酒水
    "🍰", // 甜点
    "🥗", // 沙拉/轻食
  ],
  transport: [
    "🚗", // 私家车
    "🚌", // 公交
    "🚇", // 地铁
    "🚕", // 打车
    "🚲", // 骑行
    "⛽", // 加油
    "✈️", // 飞机
    "🚄", // 高铁
  ],
  money: [
    "💰", // 现金
    "💵", // 钞票
    "💳", // 银行卡
    "🏦", // 银行
    "📊", // 理财
    "📈", // 投资
  ],
  shopping: [
    "🛒", // 超市
    "🛍️", // 商场
    "👕", // 服装
    "👟", // 鞋
    "💄", // 化妆品
    "🎁", // 礼物
    "📦", // 快递
  ],
  home: [
    "🏠", // 房租/房贷
    "💡", // 水电
    "🔧", // 维修
    "🛁", // 卫浴
    "🚿", // 水费
    "🧹", // 清洁
    "🧴", // 日用品
  ],
  health: [
    "💊", // 药品
    "🏥", // 医院
    "💉", // 体检/疫苗
    "❤️", // 心脏/健康
    "🏃", // 健身
    "💪", // 运动
  ],
  entertainment: [
    "🎬", // 电影
    "🎵", // 音乐
    "🎮", // 游戏
    "📚", // 书籍
    "🎨", // 美术
    "🎭", // 演出
  ],
  work: [
    "💼", // 办公
    "📝", // 文具
    "📅", // 日程
    "💻", // 电脑
    "📱", // 通讯
  ],
  income: [
    "🤑", // 工资
    "🧧", // 红包
    "⭐", // 奖金
    "✨", // 兼职
    "🎉", // 收入
  ],
  other: [
    "❓", // 未知
    "💬", // 备注
    "✏️", // 记录
    "💡", // 想法
  ],
};

interface IconPickerProps {
  value: string;
  onChange: (icon: string | null) => void;
  disabled?: boolean;
}

export function IconPicker({
  value,
  onChange,
  disabled = false,
}: IconPickerProps) {
  const { t } = useTranslation();
  const [open, setOpen] = useState(false);

  const handleSelect = (icon: string) => {
    onChange(icon);
    setOpen(false);
  };

  const handleClear = () => {
    onChange(null);
  };

  return (
    <Popover open={open} onOpenChange={setOpen}>
      <PopoverTrigger asChild>
        <Button
          variant="outline"
          className="h-10 w-full justify-start gap-2"
          disabled={disabled}
        >
          {value ? (
            <span className="text-lg">{value}</span>
          ) : (
            <span className="text-muted-foreground">
              {t("categories.iconPlaceholder")}
            </span>
          )}
          {!value && (
            <span className="ml-auto text-muted-foreground text-xs">
              {t("categories.iconPick")}
            </span>
          )}
        </Button>
      </PopoverTrigger>
      <PopoverContent
        className="w-[480px] p-2"
        align="start"
        onWheel={(e) => e.stopPropagation()}
      >
        <div className="space-y-3 max-h-[320px] overflow-y-auto p-1">
          <div className="flex items-center justify-between">
            <span className="text-sm font-medium">
              {t("categories.iconSelect")}
            </span>
            {value && (
              <Button
                variant="ghost"
                size="sm"
                className="h-6 px-2"
                onClick={handleClear}
              >
                <X className="h-3.5 w-3.5" />
                {t("categories.iconClear")}
              </Button>
            )}
          </div>

          {Object.entries(ICON_CATEGORIES).map(([category, icons]) => (
            <div key={category} className="space-y-1.5">
              <span className="text-xs text-muted-foreground uppercase tracking-wide">
                {t(`categories.iconCategories.${category}`)}
              </span>
              <div className="grid grid-cols-10 gap-1">
                {icons.map((icon) => (
                  <button
                    key={icon}
                    type="button"
                    className={`h-8 w-8 rounded flex items-center justify-center text-lg hover:bg-accent transition-colors ${
                      value === icon ? "bg-accent ring-2 ring-primary" : ""
                    }`}
                    onClick={() => handleSelect(icon)}
                  >
                    {icon}
                  </button>
                ))}
              </div>
            </div>
          ))}
        </div>
      </PopoverContent>
    </Popover>
  );
}
