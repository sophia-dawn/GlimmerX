import { format, formatDistanceToNow, parseISO } from "date-fns";
import { enUS, zhCN } from "date-fns/locale";
import { getCurrentLanguage } from "@/i18n";
import type { DateRangePreset } from "@/types/report";

function getDateLocale() {
  const lng = getCurrentLanguage();
  return lng === "zh" ? zhCN : enUS;
}

function parseDateSafe(dateStr: string): Date {
  const datePart = dateStr.split("T")[0] ?? dateStr;
  const parts = datePart.split("-");
  const year = Number(parts[0] ?? "0");
  const month = Number(parts[1] ?? "0");
  const day = Number(parts[2] ?? "0");
  return new Date(year, month - 1, day);
}

export function formatLocalDate(date: Date): string {
  const year = date.getFullYear();
  const month = String(date.getMonth() + 1).padStart(2, "0");
  const day = String(date.getDate()).padStart(2, "0");
  return `${year}-${month}-${day}`;
}

export function todayLocalDate(): string {
  return formatLocalDate(new Date());
}

export function currentMonthBoundsLocal(): { start: string; end: string } {
  const now = new Date();
  const year = now.getFullYear();
  const month = now.getMonth();
  const start = new Date(year, month, 1);
  const end = new Date(year, month + 1, 0);
  return {
    start: formatLocalDate(start),
    end: formatLocalDate(end),
  };
}

export function formatDateShort(isoDate: string): string {
  const locale = getDateLocale();
  const pattern = locale === zhCN ? "M月d日" : "MMM d";
  return format(parseDateSafe(isoDate), pattern, { locale });
}

export function formatDateLong(isoDate: string): string {
  const locale = getDateLocale();
  const pattern = locale === zhCN ? "yyyy年M月d日" : "MMMM d, yyyy";
  return format(parseDateSafe(isoDate), pattern, { locale });
}

export function formatDate(isoDate: string): string {
  const locale = getDateLocale();
  const pattern = locale === zhCN ? "yyyy年M月d日" : "MMM d, yyyy";
  return format(parseISO(isoDate), pattern, { locale });
}

export function formatTime(isoDate: string): string {
  return format(parseISO(isoDate), "HH:mm", { locale: getDateLocale() });
}

export function formatDateTimeShort(isoDate: string): string {
  const locale = getDateLocale();
  const pattern = locale === zhCN ? "M月d日 HH:mm" : "MMM d, HH:mm";
  return format(parseISO(isoDate), pattern, { locale });
}

export function formatDateTime(isoDate: string): string {
  const locale = getDateLocale();
  const pattern =
    locale === zhCN ? "yyyy年M月d日 HH:mm:ss" : "yyyy-MM-dd HH:mm:ss";
  return format(parseISO(isoDate), pattern, { locale });
}

export function formatMonthYear(isoDate: string): string {
  const locale = getDateLocale();
  const pattern = locale === zhCN ? "yyyy年M月" : "MMMM yyyy";
  return format(parseISO(isoDate), pattern, { locale });
}

export function formatRelative(isoDate: string): string {
  return formatDistanceToNow(parseISO(isoDate), {
    addSuffix: true,
    locale: getDateLocale(),
  });
}

/**
 * 根据 DateRangePreset 预设返回对应的日期范围
 * @param preset 日期范围预设
 * @returns 包含 startDate 和 endDate 的对象
 */
export function getDefaultRangePresetDates(preset: DateRangePreset): {
  startDate: string;
  endDate: string;
} {
  const today = todayLocalDate();
  const now = new Date();
  const currentYear = now.getFullYear();
  const currentMonth = now.getMonth();

  switch (preset) {
    case "currentMonth": {
      const { start, end } = currentMonthBoundsLocal();
      return { startDate: start, endDate: end };
    }
    case "lastMonth": {
      const lastMonthStart = new Date(currentYear, currentMonth - 1, 1);
      const lastMonthEnd = new Date(currentYear, currentMonth, 0);
      return {
        startDate: formatLocalDate(lastMonthStart),
        endDate: formatLocalDate(lastMonthEnd),
      };
    }
    case "currentYear": {
      return { startDate: `${currentYear}-01-01`, endDate: today };
    }
    case "lastYear": {
      return {
        startDate: `${currentYear - 1}-01-01`,
        endDate: `${currentYear - 1}-12-31`,
      };
    }
    case "last3Months": {
      const start3 = new Date(currentYear, currentMonth - 2, 1);
      return {
        startDate: formatLocalDate(start3),
        endDate: today,
      };
    }
    case "last6Months": {
      const start6 = new Date(currentYear, currentMonth - 5, 1);
      return {
        startDate: formatLocalDate(start6),
        endDate: today,
      };
    }
    case "last12Months": {
      const start12 = new Date(currentYear - 1, currentMonth, 1);
      return {
        startDate: formatLocalDate(start12),
        endDate: today,
      };
    }
    default: {
      // 对于 "custom" 或未知预设，返回当前月
      const { start, end } = currentMonthBoundsLocal();
      return { startDate: start, endDate: end };
    }
  }
}
