import { todayLocalDate } from "@/utils/date";

export function exportReportToCsv(
  data: Record<string, unknown>[],
  filename: string,
): void {
  if (!data.length) return;

  const firstRow = data[0];
  if (!firstRow) return;

  const headers = Object.keys(firstRow);
  const csvContent = [
    headers.join(","),
    ...data.map((row) => headers.map((h) => formatCsvValue(row[h])).join(",")),
  ].join("\n");

  const blob = new Blob([csvContent], { type: "text/csv;charset=utf-8" });
  const url = URL.createObjectURL(blob);
  const link = document.createElement("a");
  link.href = url;
  link.download = `${filename}_${todayLocalDate()}.csv`;
  link.click();
  URL.revokeObjectURL(url);
}

function formatCsvValue(value: unknown): string {
  if (value === null || value === undefined) return "";
  const str = String(value);
  if (str.includes(",") || str.includes('"')) {
    return `"${str.replace(/"/g, '""')}"`;
  }
  return str;
}
