import { memo, useState } from "react";
import { renditionUrl, type Photo } from "../api";

interface Props {
  photo: Photo;
  width: number;
  height: number;
  onOpen: (id: string) => void;
}

/** Stable background from the photo id, shown until the thumbnail arrives (or if it never does). */
function placeholderColor(id: string): string {
  let hash = 0;
  for (let i = 0; i < id.length; i++) hash = (hash * 31 + id.charCodeAt(i)) >>> 0;
  return `hsl(${hash % 360} 18% 22%)`;
}

/** One photo in the grid. Fixed size from the row layout, so loading never shifts the page. */
export const Tile = memo(function Tile({ photo, width, height, onOpen }: Props) {
  const [failed, setFailed] = useState(false);
  const showImage = photo.has_thumbnail && !failed;

  return (
    <button
      type="button"
      className="tl-tile"
      style={{ width, height, background: placeholderColor(photo.id) }}
      onClick={() => onOpen(photo.id)}
      aria-label={`${photo.stem}.${photo.ext}`}
    >
      {showImage ? (
        <img
          src={renditionUrl(photo.id, "thumbnail")}
          alt=""
          width={width}
          height={height}
          loading="lazy"
          decoding="async"
          onError={() => setFailed(true)}
        />
      ) : (
        <span className="tl-tile-ext">{photo.ext.toUpperCase()}</span>
      )}
      {photo.is_raw && <span className="tl-badge">RAW</span>}
    </button>
  );
});
