import { useCallback, useEffect, useLayoutEffect, useMemo, useRef, useState } from "react";
import { getTimelineIndex, type Photo, type TimelineIndex } from "../api";
import "./timeline.css";
import { DaySection } from "./DaySection";
import { groupByDay, isFiltering, matches, NO_FILTERS, type Filters } from "./filters";
import { monthLabel, monthOfDay } from "./format";
import { layoutSections, visibleRange, type SectionInput } from "./layout";
import { Scrubber, type Mark } from "./Scrubber";
import { useMonths } from "./useMonths";
import { Viewer } from "./Viewer";

const GAP = 4;
const PADDING = 16;
const SCRUBBER_WIDTH = 72;
const SCRUBBER_WIDTH_NARROW = 44;
/** Render this far beyond the viewport so fast scrolling never shows blank space. */
const OVERSCAN_PX = 1400;
const NARROW_PX = 640;

type FlatItem = Photo | { gap: string };

const isPhoto = (i: FlatItem | undefined): i is Photo => i !== undefined && "id" in i;

interface Props {
  onLogout: () => void;
}

/** Google-Photos-style timeline: every photo by capture day, virtualized, with a date scrubber. */
export default function TimelineView({ onLogout }: Props) {
  const [index, setIndex] = useState<TimelineIndex | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [filters, setFilters] = useState<Filters>(NO_FILTERS);
  const [viewerId, setViewerId] = useState<string | null>(null);
  const [size, setSize] = useState({ width: 0, height: 0 });
  const [scrollTop, setScrollTop] = useState(0);
  const { pages, ensure, errors } = useMonths();
  const scrollRef = useRef<HTMLDivElement>(null);
  const anchor = useRef<{ key: string; delta: number } | null>(null);
  const pendingNav = useRef<-1 | 1 | null>(null);
  const frame = useRef(0);

  // ── Data ────────────────────────────────────────────────────────────
  const loadIndex = useCallback((signal?: AbortSignal) => {
    setError(null);
    getTimelineIndex(signal)
      .then(setIndex)
      .catch((e: unknown) => {
        if (!signal?.aborted) setError(e instanceof Error ? e.message : String(e));
      });
  }, []);

  useEffect(() => {
    const ctl = new AbortController();
    loadIndex(ctl.signal);
    return () => ctl.abort();
  }, [loadIndex]);

  // ── Size ────────────────────────────────────────────────────────────
  useEffect(() => {
    const el = scrollRef.current;
    if (!el) return;
    const ro = new ResizeObserver(() => setSize({ width: el.clientWidth, height: el.clientHeight }));
    ro.observe(el);
    setSize({ width: el.clientWidth, height: el.clientHeight });
    return () => ro.disconnect();
  }, [index]);

  // ── Layout ──────────────────────────────────────────────────────────
  const filtering = isFiltering(filters);

  const byDay = useMemo(() => {
    const all = new Map<string, Photo[]>();
    for (const photos of pages.values()) for (const [day, list] of groupByDay(photos)) all.set(day, list);
    return all;
  }, [pages]);

  /** Share of photos matching the filters among loaded months; sizes months that are not loaded yet. */
  const matchRatio = useMemo(() => {
    let total = 0;
    let hit = 0;
    for (const photos of pages.values())
      for (const p of photos) {
        total++;
        if (matches(p, filters)) hit++;
      }
    return total === 0 ? 1 : hit / total;
  }, [pages, filters]);

  const sections = useMemo(() => {
    const out: Array<SectionInput & { photos?: Photo[] }> = [];
    for (const d of index?.days ?? []) {
      if (pages.has(monthOfDay(d.day))) {
        const photos = (byDay.get(d.day) ?? []).filter((p) => matches(p, filters));
        if (photos.length > 0) out.push({ key: d.day, count: photos.length, photos });
      } else {
        out.push({ key: d.day, count: filtering ? Math.max(1, Math.round(d.n * matchRatio)) : d.n });
      }
    }
    return out;
  }, [index, pages, byDay, filters, filtering, matchRatio]);

  const scrubberWidth = size.width < NARROW_PX ? SCRUBBER_WIDTH_NARROW : SCRUBBER_WIDTH;
  const contentWidth = Math.max(0, size.width - PADDING * 2 - scrubberWidth);
  const targetHeight = size.width < NARROW_PX ? 130 : 220;

  const { boxes, total } = useMemo(
    () =>
      contentWidth > 0
        ? layoutSections(sections, { width: contentWidth, targetHeight, gap: GAP })
        : { boxes: [], total: 0 },
    [sections, contentWidth, targetHeight],
  );
  const boxesRef = useRef(boxes);
  boxesRef.current = boxes;

  const [start, end] = visibleRange(boxes, scrollTop, size.height, OVERSCAN_PX);

  // Keep what the user is looking at fixed when estimated heights are replaced by real ones.
  useLayoutEffect(() => {
    const a = anchor.current;
    const el = scrollRef.current;
    if (!a || !el) return;
    const box = boxes.find((b) => b.key === a.key);
    if (!box) return;
    const target = box.top + a.delta;
    if (Math.abs(el.scrollTop - target) > 1) el.scrollTop = target;
  }, [boxes]);

  // Load the months in view, one either side, or everything while filtering (counts need real data).
  useEffect(() => {
    const wanted = new Set<string>();
    const from = filtering ? 0 : Math.max(0, start - 1);
    const to = filtering ? sections.length : Math.min(sections.length, end + 1);
    for (let i = from; i < to; i++) {
      const s = sections[i];
      if (s) wanted.add(monthOfDay(s.key));
    }
    wanted.forEach(ensure);
  }, [sections, start, end, filtering, ensure]);

  // ── Scrolling ───────────────────────────────────────────────────────
  const onScroll = useCallback(() => {
    if (frame.current) return;
    frame.current = requestAnimationFrame(() => {
      frame.current = 0;
      const el = scrollRef.current;
      if (!el) return;
      setScrollTop(el.scrollTop);
      const [first] = visibleRange(boxesRef.current, el.scrollTop, el.clientHeight, 0);
      const box = boxesRef.current[first];
      anchor.current = box ? { key: box.key, delta: el.scrollTop - box.top } : null;
    });
  }, []);

  useEffect(() => () => cancelAnimationFrame(frame.current), []);

  const seek = useCallback(
    (frac: number) => {
      const el = scrollRef.current;
      if (el) el.scrollTop = frac * Math.max(0, total - size.height);
    },
    [total, size.height],
  );

  const marks = useMemo<Mark[]>(() => {
    const out: Mark[] = [];
    let lastMonth = "";
    let lastYear = "";
    boxes.forEach((b) => {
      const month = monthOfDay(b.key);
      if (month === lastMonth) return;
      lastMonth = month;
      const year = month.slice(0, 4);
      const frac = total > 0 ? b.top / total : 0;
      if (year !== lastYear) {
        lastYear = year;
        out.push({ id: `y${year}`, label: year, frac, year: true });
      } else {
        out.push({ id: month, label: monthLabel(month).split(" ")[0]?.slice(0, 3) ?? month, frac, year: false });
      }
    });
    return out;
  }, [boxes, total]);

  const topKey = boxes[start]?.key;
  const current = topKey ? monthLabel(monthOfDay(topKey)) : "";
  const position = total > size.height ? Math.min(1, scrollTop / (total - size.height)) : 0;

  // ── Viewer navigation ───────────────────────────────────────────────
  const flat = useMemo<FlatItem[]>(() => {
    const out: FlatItem[] = [];
    for (const s of sections) {
      if (s.photos) out.push(...s.photos);
      else {
        const month = monthOfDay(s.key);
        const last = out[out.length - 1];
        if (!(last && "gap" in last && last.gap === month)) out.push({ gap: month });
      }
    }
    return out;
  }, [sections]);

  const flatIndex = viewerId ? flat.findIndex((p) => isPhoto(p) && p.id === viewerId) : -1;
  const viewerPhoto = flatIndex >= 0 ? flat[flatIndex] : undefined;

  const navigate = useCallback(
    (dir: -1 | 1) => {
      const next = flat[flatIndex + dir];
      if (isPhoto(next)) {
        pendingNav.current = null;
        setViewerId(next.id);
      } else if (next) {
        // Neighbouring month is not loaded yet: fetch it and continue when it arrives.
        pendingNav.current = dir;
        ensure(next.gap);
      }
    },
    [flat, flatIndex, ensure],
  );

  useEffect(() => {
    const dir = pendingNav.current;
    if (dir && isPhoto(flat[flatIndex + dir])) navigate(dir);
  }, [flat, flatIndex, navigate]);

  const preload = useMemo(
    () =>
      [flat[flatIndex - 1], flat[flatIndex + 1], flat[flatIndex + 2]].filter(isPhoto).map((p) => p.id),
    [flat, flatIndex],
  );

  const changeFilters = (next: Filters) => {
    setFilters(next);
    anchor.current = null;
    if (scrollRef.current) scrollRef.current.scrollTop = 0;
    setScrollTop(0);
  };

  // ── Render ──────────────────────────────────────────────────────────
  const failedMonths = errors.size;
  return (
    <div className="tl-root">
      <header className="tl-header">
        <span className="tl-logo">
          <span className="logo-icon">RV</span> RawVault
        </span>
        <div className="tl-controls">
          <select
            className="filter-select"
            aria-label="Gallery"
            value={filters.gallery ?? ""}
            onChange={(e) => changeFilters({ ...filters, gallery: e.target.value || null })}
          >
            <option value="">All galleries</option>
            {index?.galleries.map((g) => (
              <option key={g} value={g}>
                {g}
              </option>
            ))}
          </select>
          <label className="tl-check">
            <input
              type="checkbox"
              checked={filters.rawOnly}
              onChange={(e) => changeFilters({ ...filters, rawOnly: e.target.checked })}
            />
            RAW only
          </label>
          {index && <span className="tl-count">{index.total.toLocaleString("en-GB")} photos</span>}
          <button type="button" className="btn-icon" onClick={onLogout} title="Log out" aria-label="Log out">
            ↗
          </button>
        </div>
      </header>

      {error && (
        <div className="tl-state" role="alert">
          <p>Could not load the library: {error}</p>
          <button type="button" className="btn btn-primary" onClick={() => loadIndex()}>
            Retry
          </button>
        </div>
      )}
      {!error && !index && (
        <div className="tl-state">
          <div className="spinner" />
        </div>
      )}
      {index && index.total === 0 && (
        <div className="tl-state" role="status">
          <h2>Your timeline is empty</h2>
          <p>
            Photos appear here after the indexing pipeline has processed your library. Until then the classic
            gallery still shows everything.
          </p>
          <a className="btn btn-primary" href="?ui=legacy">
            Open the classic gallery
          </a>
        </div>
      )}
      {failedMonths > 0 && <div className="tl-toast">Some months failed to load; scroll to retry.</div>}

      {index && index.total > 0 && (
        <div className="tl-body">
          <div ref={scrollRef} className="tl-scroll" onScroll={onScroll}>
            <div className="tl-canvas" style={{ height: total }}>
              {boxes.slice(start, end).map((box, i) => {
                const s = sections[start + i];
                if (!s) return null;
                return (
                  <div
                    key={box.key}
                    className="tl-section"
                    style={{ top: box.top, height: box.height, left: PADDING, width: contentWidth }}
                  >
                    <DaySection
                      day={box.key}
                      count={s.count}
                      gap={GAP}
                      onOpen={setViewerId}
                      {...(box.rows && s.photos ? { rows: box.rows, photos: s.photos } : {})}
                    />
                  </div>
                );
              })}
            </div>
            {filtering && sections.length === 0 && pages.size > 0 && (
              <div className="tl-state">
                <p>No photos match these filters.</p>
              </div>
            )}
          </div>
          <Scrubber marks={marks} position={position} current={current} width={scrubberWidth} onSeek={seek} />
        </div>
      )}

      {viewerPhoto && isPhoto(viewerPhoto) && (
        <Viewer
          photo={viewerPhoto}
          hasPrev={flatIndex > 0}
          hasNext={flatIndex < flat.length - 1}
          preload={preload}
          onNavigate={navigate}
          onClose={() => setViewerId(null)}
        />
      )}
    </div>
  );
}
