import type { Photo } from "../api";

export interface Filters {
  /** Show only this gallery (camera folder), or all when `null`. */
  gallery: string | null;
  /** Show only camera RAW originals. */
  rawOnly: boolean;
}

export const NO_FILTERS: Filters = { gallery: null, rawOnly: false };

export function isFiltering(f: Filters): boolean {
  return f.gallery !== null || f.rawOnly;
}

export function matches(photo: Photo, f: Filters): boolean {
  if (f.gallery !== null && photo.gallery !== f.gallery) return false;
  if (f.rawOnly && !photo.is_raw) return false;
  return true;
}

/** Groups a month's photos by calendar day (`YYYY-MM-DD`), keeping their order. */
export function groupByDay(photos: readonly Photo[]): Map<string, Photo[]> {
  const days = new Map<string, Photo[]>();
  for (const p of photos) {
    const day = p.taken_at.slice(0, 10);
    const list = days.get(day);
    if (list) list.push(p);
    else days.set(day, [p]);
  }
  return days;
}
