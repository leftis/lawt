import { memo } from "react";
import type { Photo } from "../api";
import { dayLabel } from "./format";
import type { Row } from "./layout";
import { Tile } from "./Tile";

interface Props {
  day: string;
  count: number;
  /** Rows and photos when the month is loaded; a skeleton is shown otherwise. */
  rows?: readonly Row[];
  photos?: readonly Photo[];
  gap: number;
  onOpen: (id: string) => void;
}

/** One day: sticky header plus justified rows of tiles (or a skeleton while its month loads). */
export const DaySection = memo(function DaySection({ day, count, rows, photos, gap, onOpen }: Props) {
  return (
    <section className="tl-day" aria-label={dayLabel(day)}>
      <h2 className="tl-day-header">
        {dayLabel(day)} <span>{count}</span>
      </h2>
      {rows && photos ? (
        rows.map((row, r) => (
          <div key={r} className="tl-row" style={{ height: row.height, marginBottom: gap, gap }}>
            {row.tiles.map((t) => {
              const photo = photos[t.index];
              return photo ? <Tile key={photo.id} photo={photo} width={t.width} height={t.height} onOpen={onOpen} /> : null;
            })}
          </div>
        ))
      ) : (
        <div className="tl-skeleton" aria-hidden="true" />
      )}
    </section>
  );
});
