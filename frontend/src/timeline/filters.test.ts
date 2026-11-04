import { describe, expect, it } from "vitest";
import type { Photo } from "../api";
import { groupByDay, isFiltering, matches, NO_FILTERS } from "./filters";

const photo = (over: Partial<Photo>): Photo => ({
  id: "a",
  original_key: "all images/g/a.raf",
  gallery: "g",
  stem: "a",
  ext: "raf",
  is_raw: true,
  width: 6000,
  height: 4000,
  taken_at: "2024-06-15T10:00:00",
  taken_at_source: "exif",
  size_bytes: 1,
  has_thumbnail: true,
  has_full: true,
  ...over,
});

describe("filters", () => {
  it("passes everything with no filters", () => {
    expect(isFiltering(NO_FILTERS)).toBe(false);
    expect(matches(photo({}), NO_FILTERS)).toBe(true);
  });

  it("filters by gallery and RAW", () => {
    expect(matches(photo({ gallery: "x" }), { gallery: "g", rawOnly: false })).toBe(false);
    expect(matches(photo({ is_raw: false }), { gallery: null, rawOnly: true })).toBe(false);
    expect(matches(photo({}), { gallery: "g", rawOnly: true })).toBe(true);
  });

  it("groups by day and keeps order", () => {
    const days = groupByDay([
      photo({ id: "1", taken_at: "2024-06-15T18:00:00" }),
      photo({ id: "2", taken_at: "2024-06-15T09:00:00" }),
      photo({ id: "3", taken_at: "2024-06-14T09:00:00" }),
    ]);
    expect([...days.keys()]).toEqual(["2024-06-15", "2024-06-14"]);
    expect(days.get("2024-06-15")?.map((p) => p.id)).toEqual(["1", "2"]);
  });
});
