import { useEffect } from "react";
import { downloadImage, renditionUrl, type Photo } from "../api";
import { dayLabel, formatBytes } from "./format";

interface Props {
  photo: Photo;
  hasPrev: boolean;
  hasNext: boolean;
  /** Ids of neighbours to preload so arrow keys feel instant. */
  preload: readonly string[];
  onNavigate: (dir: -1 | 1) => void;
  onClose: () => void;
}

async function openOriginal(key: string): Promise<void> {
  const info = await downloadImage(key, "raw");
  if (info.download_url) window.open(info.download_url, "_blank", "noopener,noreferrer");
}

/** Fullscreen viewer: medium rendition, keyboard navigation, neighbour preloading. */
export function Viewer({ photo, hasPrev, hasNext, preload, onNavigate, onClose }: Props) {
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") onClose();
      else if (e.key === "ArrowLeft") onNavigate(-1);
      else if (e.key === "ArrowRight") onNavigate(1);
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [onClose, onNavigate]);

  useEffect(() => {
    for (const id of preload) new Image().src = renditionUrl(id, "medium");
  }, [preload]);

  const taken = photo.taken_at.replace("T", " ").slice(0, 16);

  return (
    <div className="tl-viewer" role="dialog" aria-modal="true" aria-label={`${photo.stem}.${photo.ext}`}>
      <button type="button" className="tl-viewer-close" onClick={onClose} aria-label="Close">
        ✕
      </button>
      <button
        type="button"
        className="tl-viewer-nav prev"
        onClick={() => onNavigate(-1)}
        disabled={!hasPrev}
        aria-label="Previous photo"
      >
        ‹
      </button>
      <img
        key={photo.id}
        className="tl-viewer-img"
        src={renditionUrl(photo.id, "medium")}
        alt={`${photo.stem}.${photo.ext}`}
        decoding="async"
      />
      <button
        type="button"
        className="tl-viewer-nav next"
        onClick={() => onNavigate(1)}
        disabled={!hasNext}
        aria-label="Next photo"
      >
        ›
      </button>
      <div className="tl-viewer-bar">
        <div>
          <strong>
            {photo.stem}.{photo.ext}
          </strong>
          <span>
            {dayLabel(photo.taken_at.slice(0, 10))} · {taken.slice(11)} · {photo.gallery} · {photo.width}×
            {photo.height} · {formatBytes(photo.size_bytes)}
          </span>
        </div>
        <div className="tl-viewer-actions">
          {photo.has_full && (
            <a className="btn btn-secondary" href={renditionUrl(photo.id, "full")} target="_blank" rel="noreferrer">
              Full JPEG
            </a>
          )}
          <button
            type="button"
            className="btn btn-primary"
            onClick={() => void openOriginal(photo.original_key).catch((e: unknown) => alert(String(e)))}
          >
            Download original
          </button>
        </div>
      </div>
    </div>
  );
}
