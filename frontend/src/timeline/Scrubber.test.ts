import { describe, expect, it } from "vitest";
import { pickLabels, type Mark } from "./Scrubber";

const mark = (id: string, frac: number, year = false): Mark => ({ id, label: id, frac, year });

describe("pickLabels", () => {
  it("keeps a year label even when months crowd it", () => {
    const marks = [mark("Jun", 0.1), mark("y2024", 0.105, true), mark("May", 0.11)];
    const ids = pickLabels(marks, 1000).map((m) => m.id);
    expect(ids).toContain("y2024");
    expect(ids).not.toContain("Jun");
  });

  it("never places two labels closer than the minimum gap", () => {
    const marks = Array.from({ length: 200 }, (_, i) => mark(`m${i}`, i / 200));
    const ys = pickLabels(marks, 600).map((m) => m.frac * 600);
    for (let i = 1; i < ys.length; i++) expect((ys[i] ?? 0) - (ys[i - 1] ?? 0)).toBeGreaterThanOrEqual(18);
  });

  it("shows everything when there is room", () => {
    expect(pickLabels([mark("a", 0), mark("b", 0.5)], 1000)).toHaveLength(2);
  });
});
