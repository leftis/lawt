/**
 * Legacy gallery API (one item per file, presigned URLs).
 * Kept until the timeline UI replaces the old gallery; do not extend.
 */
import { API_BASE, apiRequest, setAuthToken } from "./http";

export interface LegacyImage {
  id: string;
  name: string;
  ext: string;
  raw_path: string;
  preview_path: string;
  medium_preview_path: string;
  size_bytes: number;
  updated_at: string;
  gallery: string;
  path: string;
  is_raw: boolean;
}

export interface DownloadInfo {
  download_url: string | null;
  preview_exists: boolean;
  file_name: string;
  expires_in?: number;
  type?: string;
}

export interface BatchUrl {
  key: string;
  download_url: string;
  preview_exists: boolean;
}

export interface TokenResponse {
  access_token: string;
  [k: string]: unknown;
}

const MAX_CACHE_SIZE = 500;
export const previewUrlCache = new Map<string, string>();

function cacheGet(key: string): string | null {
  return previewUrlCache.get(key) ?? null;
}

function cacheSet(key: string, value: string): void {
  if (previewUrlCache.size >= MAX_CACHE_SIZE) {
    const oldest = previewUrlCache.keys().next().value;
    if (oldest !== undefined) previewUrlCache.delete(oldest);
  }
  previewUrlCache.set(key, value);
}

export { setAuthToken };

export async function exchangeCode(code: string, redirectUri: string): Promise<TokenResponse> {
  const response = await fetch(`${API_BASE}/auth/token`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ code, redirect_uri: redirectUri }),
  });
  if (!response.ok) {
    const error = (await response.json().catch(() => ({ error: response.statusText }))) as { error?: string };
    throw new Error(error.error ?? "Token exchange failed");
  }
  return (await response.json()) as TokenResponse;
}

/** Gallery names only (one cheap delimiter listing on the server). */
export async function getGalleryNames(): Promise<string[]> {
  const names = await apiRequest<string[]>("/images?galleries=1");
  return Array.isArray(names) ? names : [];
}

export function getImages(gallery?: string | null): Promise<LegacyImage[]> {
  const params = new URLSearchParams();
  if (gallery) params.set("gallery", gallery);
  const qs = params.toString();
  return apiRequest<LegacyImage[]>(`/images${qs ? `?${qs}` : ""}`);
}

export const listImages = getImages;

const enc = encodeURIComponent;

export function downloadImage(key: string, type = "raw"): Promise<DownloadInfo> {
  return apiRequest<DownloadInfo>(`/images/download?key=${enc(key)}&type=${type}`);
}

export async function getFullJpegUrl(key: string): Promise<string | null> {
  const data = await downloadImage(key, "full");
  return data.preview_exists ? data.download_url : null;
}

export async function getDownloadUrl(key: string): Promise<string | null> {
  const data = await apiRequest<DownloadInfo>(`/images/download?key=${enc(key)}`);
  return data.download_url || null;
}

export async function getPreviewUrl(key: string): Promise<string | null> {
  const cached = cacheGet(key);
  if (cached) return cached;
  const data = await downloadImage(key, "thumbnail");
  if (data.download_url && data.preview_exists) cacheSet(key, data.download_url);
  return data.preview_exists ? data.download_url : null;
}

export async function getMediumUrl(key: string): Promise<string | null> {
  const cacheKey = `${key}::medium`;
  const cached = cacheGet(cacheKey);
  if (cached) return cached;
  const data = await downloadImage(key, "medium");
  if (data.download_url && data.preview_exists) cacheSet(cacheKey, data.download_url);
  return data.preview_exists ? data.download_url : null;
}

export async function getPreviewDownloadUrl(key: string): Promise<{ url: string; fileName: string } | null> {
  const data = await downloadImage(key, "medium");
  if (data.download_url && data.preview_exists) return { url: data.download_url, fileName: data.file_name };
  return null;
}

/** Presigned URLs for many keys in one call. */
export async function batchGetDownloadUrls(requests: Array<{ key: string; type?: string }>): Promise<BatchUrl[]> {
  const data = await apiRequest<{ urls?: BatchUrl[] }>("/images/download/batch", {
    method: "POST",
    body: JSON.stringify({ requests }),
  });
  return data.urls ?? [];
}

export async function regeneratePreview(rawPath: string): Promise<{ status?: string; [k: string]: unknown }> {
  return apiRequest(`/preview-trigger?raw_path=${enc(rawPath)}`, { method: "POST" });
}

const PLACEHOLDER_COLORS = [
  "#f0e6f6", "#e6f0f6", "#d6e4f0", "#f6e6f0", "#e8f0d6",
  "#f0e8d6", "#d6f0ee", "#f6e8d0", "#e0d6f6", "#d6e6f6",
];

/** Stable pastel color from a file name, for placeholder backgrounds. */
export function getColor(name: string): string {
  let hash = 0;
  for (let i = 0; i < name.length; i++) hash = name.charCodeAt(i) + ((hash << 5) - hash);
  return PLACEHOLDER_COLORS[Math.abs(hash) % PLACEHOLDER_COLORS.length] ?? "#e6f0f6";
}

export function formatSize(bytes: number | null | undefined): string {
  if (!bytes) return "—";
  const units = ["B", "KB", "MB", "GB"];
  let value = bytes;
  let i = 0;
  while (value >= 1024 && i < units.length - 1) {
    value /= 1024;
    i++;
  }
  return `${value.toFixed(1)} ${units[i]}`;
}

export function formatDate(dateStr: string | null | undefined): string {
  if (!dateStr) return "—";
  return new Date(dateStr).toLocaleDateString("en-US", { month: "short", day: "numeric", year: "numeric" });
}
