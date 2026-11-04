import { useCallback, useRef, useState } from "react";

export interface Mark {
  /** Unique key. */
  id: string;
  label: string;
  /** Position along the whole timeline, 0 (newest) to 1 (oldest). */
  frac: number;
  /** Years get a bolder label. */
  year: boolean;
}

interface Props {
  marks: readonly Mark[];
  /** Current scroll position, 0 to 1. */
  position: number;
  /** Label of the section at the top of the viewport. */
  current: string;
  /** Rail width in CSS pixels. */
  width: number;
  onSeek: (frac: number) => void;
}

/** Minimum gap between two labels so they never overlap. */
const MIN_LABEL_GAP_PX = 18;

/** Vertical rail with year/month labels; click or drag to jump anywhere in the library. */
export function Scrubber({ marks, position, current, width, onSeek }: Props) {
  const rail = useRef<HTMLDivElement>(null);
  const [dragging, setDragging] = useState(false);

  const seek = useCallback(
    (clientY: number) => {
      const el = rail.current;
      if (!el) return;
      const box = el.getBoundingClientRect();
      onSeek(Math.min(1, Math.max(0, (clientY - box.top) / box.height)));
    },
    [onSeek],
  );

  const railHeight = rail.current?.clientHeight ?? 0;
  const visible = pickLabels(marks, railHeight);

  return (
    <div
      ref={rail}
      className={`tl-scrubber${dragging ? " dragging" : ""}`}
      style={{ width }}
      role="slider"
      aria-label="Jump to date"
      aria-valuemin={0}
      aria-valuemax={100}
      aria-valuenow={Math.round(position * 100)}
      aria-valuetext={current}
      tabIndex={0}
      onPointerDown={(e) => {
        e.currentTarget.setPointerCapture(e.pointerId);
        setDragging(true);
        seek(e.clientY);
      }}
      onPointerMove={(e) => dragging && seek(e.clientY)}
      onPointerUp={() => setDragging(false)}
      onPointerCancel={() => setDragging(false)}
    >
      {visible.map((m) => (
        <span key={m.id} className={m.year ? "tl-mark year" : "tl-mark"} style={{ top: `${m.frac * 100}%` }}>
          {m.label}
        </span>
      ))}
      <span className="tl-thumb" style={{ top: `${position * 100}%` }}>
        {dragging && <span className="tl-thumb-label">{current}</span>}
      </span>
    </div>
  );
}

/** Chooses labels that never overlap; years are placed first so they always win over months. */
export function pickLabels(marks: readonly Mark[], railHeight: number): Mark[] {
  const placed: number[] = [];
  const chosen = new Set<string>();
  const fits = (y: number): boolean => placed.every((p) => Math.abs(p - y) >= MIN_LABEL_GAP_PX);
  for (const pass of [true, false]) {
    for (const m of marks) {
      if (m.year !== pass) continue;
      const y = m.frac * railHeight;
      if (fits(y)) {
        placed.push(y);
        chosen.add(m.id);
      }
    }
  }
  return marks.filter((m) => chosen.has(m.id));
}
