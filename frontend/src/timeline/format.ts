/** Date formatting for the timeline. Days are camera wall-clock dates, so never shift by time zone. */

const dayFormat = new Intl.DateTimeFormat("en-GB", {
  weekday: "short",
  day: "numeric",
  month: "short",
  year: "numeric",
  timeZone: "UTC",
});

const monthFormat = new Intl.DateTimeFormat("en-GB", { month: "long", year: "numeric", timeZone: "UTC" });

/** `2024-06-15` → `Sat, 15 Jun 2024`. */
export function dayLabel(day: string): string {
  return dayFormat.format(new Date(`${day}T00:00:00Z`));
}

/** `2024-06` → `June 2024`. */
export function monthLabel(month: string): string {
  return monthFormat.format(new Date(`${month}-01T00:00:00Z`));
}

/** `2024-06-15` → `2024-06`. */
export function monthOfDay(day: string): string {
  return day.slice(0, 7);
}

export function formatBytes(bytes: number): string {
  const units = ["B", "KB", "MB", "GB"];
  let value = bytes;
  let i = 0;
  while (value >= 1024 && i < units.length - 1) {
    value /= 1024;
    i++;
  }
  return `${value.toFixed(value >= 100 || i === 0 ? 0 : 1)} ${units[i]}`;
}
