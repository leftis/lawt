/** Timeline endpoints: the day index, month pages and rendition URLs. */
import type { MonthPage } from "./generated/MonthPage";
import type { TimelineIndex } from "./generated/TimelineIndex";
import { API_BASE, apiRequest } from "./http";

export type { MonthPage, TimelineIndex };
export type { Photo } from "./generated/Photo";
export type { TimelineDay } from "./generated/TimelineDay";

export type RenditionName = "thumbnail" | "medium" | "full";

/** Everything needed to lay out the whole scroll bar; small, load once. */
export function getTimelineIndex(signal?: AbortSignal): Promise<TimelineIndex> {
  return apiRequest<TimelineIndex>("/timeline", { signal });
}

/** All photos of a month (`YYYY-MM`), newest first. */
export function getMonth(month: string, signal?: AbortSignal): Promise<MonthPage> {
  return apiRequest<MonthPage>(`/timeline/months/${encodeURIComponent(month)}`, { signal });
}

/**
 * Stable URL of a generated rendition.
 *
 * Served from `VITE_RENDITION_BASE_URL` (the delivery CDN for the derived bucket) when set,
 * otherwise from the API base, which is what the local dev server does.
 * The URL depends only on the photo id, so browsers and the CDN can cache it forever.
 * Deliberately separate from `VITE_CDN_HOST`, which belongs to the legacy photo-bucket CDN.
 */
export function renditionUrl(id: string, rendition: RenditionName): string {
  const base = import.meta.env.VITE_RENDITION_BASE_URL || API_BASE;
  return `${base.replace(/\/$/, "")}/derived/${id}/${rendition}.jpeg`;
}
