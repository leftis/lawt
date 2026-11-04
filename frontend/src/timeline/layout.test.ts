import { describe, expect, it } from "vitest";
import { estimateRowsHeight, justify, layoutSections, visibleRange } from "./layout";

const opts = { width: 1000, targetHeight: 200, gap: 4 };

describe("justify", () => {
  it("fills full rows exactly and never drops or repeats a photo", () => {
    const aspects = [1.5, 1.5, 0.66, 1.5, 1, 1.78, 1.5, 0.75, 1.5, 1.5, 1.5];
    const rows = justify(aspects, opts);
    const seen = rows.flatMap((r) => r.tiles.map((t) => t.index));
    expect(seen).toEqual(aspects.map((_, i) => i));
    for (const row of rows.slice(0, -1)) {
      const used = row.tiles.reduce((a, t) => a + t.width, 0) + (row.tiles.length - 1) * opts.gap;
      expect(used).toBe(opts.width);
      expect(row.height).toBeLessThanOrEqual(opts.targetHeight);
    }
  });

  it("does not stretch a short last row", () => {
    const rows = justify([1.5], opts);
    expect(rows).toHaveLength(1);
    expect(rows[0]?.height).toBe(opts.targetHeight);
    expect(rows[0]?.tiles[0]?.width).toBe(300);
  });

  it("clamps absurd aspect ratios", () => {
    const rows = justify([100, 0.0001], opts);
    for (const t of rows.flatMap((r) => r.tiles)) {
      expect(t.width).toBeGreaterThan(0);
      expect(t.height).toBeGreaterThan(0);
    }
  });

  it("returns nothing for no photos", () => {
    expect(justify([], opts)).toEqual([]);
  });
});

describe("layoutSections", () => {
  it("stacks sections with monotonic offsets and a matching total", () => {
    const { boxes, total } = layoutSections(
      [
        { key: "a", count: 40 },
        { key: "b", count: 3, photos: [{ width: 6000, height: 4000 }, { width: 4000, height: 6000 }, { width: 5000, height: 5000 }] },
        { key: "c", count: 0 },
      ],
      opts,
    );
    expect(boxes[0]?.top).toBe(0);
    expect(boxes[1]?.top).toBe(boxes[0]?.height);
    expect(boxes[1]?.rows).toBeDefined();
    expect(boxes[0]?.rows).toBeUndefined();
    const last = boxes[2];
    expect((last?.top ?? 0) + (last?.height ?? 0)).toBe(total);
  });

  it("estimates more height for more photos", () => {
    expect(estimateRowsHeight(100, opts)).toBeGreaterThan(estimateRowsHeight(10, opts));
    expect(estimateRowsHeight(0, opts)).toBe(0);
  });
});

describe("estimates", () => {
  it("are close to real layouts before anything has loaded", () => {
    const shapes = Array.from({ length: 60 }, (_, i) => (i % 3 === 0 ? { width: 4000, height: 6000 } : { width: 6000, height: 4000 }));
    const real = layoutSections([{ key: "a", count: 60, photos: shapes }], opts).boxes[0]?.height ?? 0;
    const est = layoutSections([{ key: "a", count: 60 }], opts).boxes[0]?.height ?? 0;
    expect(Math.abs(est - real) / real).toBeLessThan(0.15);
  });

  it("learn the real height per photo from loaded sections", () => {
    const shapes = Array.from({ length: 30 }, () => ({ width: 6000, height: 4000 }));
    const loaded = { key: "a", count: 30, photos: shapes };
    const { boxes } = layoutSections([loaded, { key: "b", count: 30 }], opts);
    // Same photo count and same kind of photos: the estimate must match the real height closely.
    const real = boxes[0]?.height ?? 0;
    const est = boxes[1]?.height ?? 0;
    expect(Math.abs(est - real) / real).toBeLessThan(0.02);
  });
});

describe("visibleRange", () => {
  const { boxes } = layoutSections(
    Array.from({ length: 50 }, (_, i) => ({ key: String(i), count: 20 })),
    opts,
  );

  it("starts at the section under the viewport top", () => {
    const [start] = visibleRange(boxes, 0, 800, 0);
    expect(start).toBe(0);
    const mid = boxes[20];
    const [s] = visibleRange(boxes, (mid?.top ?? 0) + 1, 800, 0);
    expect(s).toBe(20);
  });

  it("includes overscan and always at least one section", () => {
    const [s0, e0] = visibleRange(boxes, 0, 800, 0);
    const [, e1] = visibleRange(boxes, 0, 800, 2000);
    expect(e1).toBeGreaterThan(e0);
    expect(e0).toBeGreaterThan(s0);
    expect(visibleRange([], 0, 800, 0)).toEqual([0, 0]);
  });

  it("clamps past the end", () => {
    const [s, e] = visibleRange(boxes, 1e9, 800, 0);
    expect(s).toBe(boxes.length);
    expect(e).toBe(boxes.length);
  });
});
