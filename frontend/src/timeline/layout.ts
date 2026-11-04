/** Pure layout math for the timeline: justified rows, section offsets, visible range. */

export interface Tile {
  /** Index into the section's photo list. */
  index: number;
  width: number;
  height: number;
}

export interface Row {
  tiles: Tile[];
  height: number;
}

export interface LayoutOptions {
  /** Content width in CSS pixels. */
  width: number;
  /** Preferred row height; full rows land at or below it. */
  targetHeight: number;
  /** Space between tiles and rows. */
  gap: number;
}

const MIN_ASPECT = 0.2;
const MAX_ASPECT = 5;

/** Average width/height assumed for photos that have not loaded yet (typical 3:2 mix). */
export const ESTIMATED_ASPECT = 1.4;

export function aspectOf(photo: { width: number; height: number }): number {
  if (photo.width <= 0 || photo.height <= 0) return 1;
  return Math.min(MAX_ASPECT, Math.max(MIN_ASPECT, photo.width / photo.height));
}

/**
 * Packs photos into rows of equal height that fill `width` exactly.
 * The final row keeps `targetHeight` instead of stretching.
 */
export function justify(aspects: readonly number[], { width, targetHeight, gap }: LayoutOptions): Row[] {
  const rows: Row[] = [];
  let pending: Array<{ index: number; aspect: number }> = [];
  let sum = 0;

  const close = (height: number, fill: boolean): void => {
    const gaps = (pending.length - 1) * gap;
    const tiles: Tile[] = pending.map((p) => ({
      index: p.index,
      width: Math.floor(p.aspect * height),
      height: Math.floor(height),
    }));
    if (fill && tiles.length > 0) {
      // Give the rounding remainder to the last tile so the row ends flush.
      const used = tiles.reduce((acc, t) => acc + t.width, 0) + gaps;
      const last = tiles[tiles.length - 1];
      if (last) last.width += Math.max(0, Math.floor(width) - used);
    }
    rows.push({ tiles, height: Math.floor(height) });
    pending = [];
    sum = 0;
  };

  aspects.forEach((raw, index) => {
    const aspect = Math.min(MAX_ASPECT, Math.max(MIN_ASPECT, raw));
    pending.push({ index, aspect });
    sum += aspect;
    const rowHeight = (width - (pending.length - 1) * gap) / sum;
    if (rowHeight <= targetHeight) close(rowHeight, true);
  });
  if (pending.length > 0) close(targetHeight, false);
  return rows;
}

/** Height of `rows` including the gaps between them and one trailing gap. */
export function rowsHeight(rows: readonly Row[], gap: number): number {
  return rows.reduce((acc, r) => acc + r.height + gap, 0);
}

/**
 * Rough height of `count` photos not yet loaded, so the scroll bar is stable up front.
 * Mirrors `justify`: a row closes at the first photo count whose natural height falls to
 * the target or below, and then holds `k` photos of `ESTIMATED_ASPECT`.
 */
export function estimateRowsHeight(count: number, { width, targetHeight, gap }: LayoutOptions): number {
  if (count <= 0) return 0;
  const perRow = Math.max(1, Math.ceil((width + gap) / (ESTIMATED_ASPECT * targetHeight + gap)));
  const rowHeight = Math.min(targetHeight, (width - (perRow - 1) * gap) / (perRow * ESTIMATED_ASPECT));
  return Math.ceil(count / perRow) * (rowHeight + gap);
}

export interface SectionInput {
  key: string;
  count: number;
  /** Real photo shapes when the month is loaded. */
  photos?: ReadonlyArray<{ width: number; height: number }>;
}

export interface SectionBox {
  key: string;
  top: number;
  height: number;
  /** Present once the section's photos are known. */
  rows?: Row[];
}

/** Space above the rows of a section for the day header. */
export const HEADER_HEIGHT = 48;
/** Space below each section. */
export const SECTION_GAP = 12;

export function layoutSections(
  sections: readonly SectionInput[],
  options: LayoutOptions,
): { boxes: SectionBox[]; total: number } {
  // Pass 1: real layout for loaded sections, and how tall a photo really is on average.
  const laidOut = sections.map((s) => (s.photos ? justify(s.photos.map(aspectOf), options) : undefined));
  let loadedPhotos = 0;
  let loadedBody = 0;
  sections.forEach((s, i) => {
    const rows = laidOut[i];
    if (rows && s.photos) {
      loadedPhotos += s.photos.length;
      loadedBody += rowsHeight(rows, options.gap);
    }
  });
  const perPhoto = loadedPhotos > 0 ? loadedBody / loadedPhotos : undefined;

  // Pass 2: stack sections, estimating the unloaded ones from what the loaded ones taught us.
  const boxes: SectionBox[] = [];
  let top = 0;
  sections.forEach((s, i) => {
    const rows = laidOut[i];
    const body = rows
      ? rowsHeight(rows, options.gap)
      : perPhoto !== undefined
        ? s.count * perPhoto
        : estimateRowsHeight(s.count, options);
    const height = Math.round(HEADER_HEIGHT + body + SECTION_GAP);
    boxes.push(rows ? { key: s.key, top, height, rows } : { key: s.key, top, height });
    top += height;
  });
  return { boxes, total: top };
}

/**
 * Half-open range `[start, end)` of sections intersecting the viewport plus `overscan` pixels
 * on each side. Assumes `boxes` are sorted by `top` (they are, by construction).
 */
export function visibleRange(
  boxes: readonly SectionBox[],
  scrollTop: number,
  viewport: number,
  overscan: number,
): [number, number] {
  if (boxes.length === 0) return [0, 0];
  const from = scrollTop - overscan;
  const to = scrollTop + viewport + overscan;
  let lo = 0;
  let hi = boxes.length;
  while (lo < hi) {
    const mid = (lo + hi) >> 1;
    const b = boxes[mid];
    if (b && b.top + b.height <= from) lo = mid + 1;
    else hi = mid;
  }
  const start = lo;
  let end = start;
  while (end < boxes.length && (boxes[end]?.top ?? Infinity) < to) end++;
  return [start, Math.max(end, Math.min(start + 1, boxes.length))];
}
