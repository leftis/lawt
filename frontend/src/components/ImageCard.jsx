/**
 * ImageCard Component
 * ─────────────────────
 * Displays a thumbnail preview with file info, and download button.
 * Uses IntersectionObserver for lazy loading to avoid excess API calls.
 * All file types (JPEG/PNG/RAW/TIFF) now get small (~1000px) previews.
 * Uses batch-fetched preview URLs when available to eliminate individual Lambda calls.
 * 
 * Props:
 *   showCheckbox, isChecked, onToggle — for multi-select mode
 *   previewCache — Map of raw_path -> { previewUrl, previewExists } from batch fetch
 */
import { useState, useEffect, useRef } from "react";
import { getPreviewUrl, getPreviewDownloadUrl, formatSize, formatDate, getColor, regeneratePreview, previewUrlCache } from "../api";

// ── Lazy-load registry for batched loading ─────────────────────────────
const lazyLoadRegistry = new Map(); // key -> [{ ref, loaded, ... }]
let observer = null;

function ensureObserver() {
  if (observer) return observer;
  observer = new IntersectionObserver(
    (entries) => {
      entries.forEach((entry) => {
        if (entry.isIntersecting) {
          const card = entry.target;
          card.classList.add("is-visible");
          observer.unobserve(card);
        }
      });
    },
    { rootMargin: "200px" }
  );
  return observer;
}

function registerLazyCard(cardEl, rawPath) {
  ensureObserver().observe(cardEl);
}

function unregisterLazyCard(cardEl) {
  if (observer) {
    observer.unobserve(cardEl);
  }
}

/**
 * Loads a preview URL from the API when scrolled into view.
 * All file types (JPEG, PNG, RAW, TIFF) now get .preview.png thumbnails.
 */
function ImagePreview({ image, showRaw, regenKey, previewCache, onFetchMissingUrl }) {
  const [src, setSrc] = useState(null);
  const [loading, setLoading] = useState(false);
  const [isIntersecting, setIsIntersecting] = useState(false);
  const cardRef = useRef(null);
  const ext = (image.ext || image.name.split(".").pop() || "").toUpperCase();
  const color = getColor(image.name);
  const hasFetched = useRef(false);

  // Check if we have a batch-cached preview URL
  const cachedEntry = previewCache?.[image.raw_path];
  const cachedUrl = cachedEntry?.previewUrl || null;
  // Only treat as confirmed-no-preview when the user hasn't just triggered a regen.
  // After regen (regenKey > 0) we ignore the stale cache entry and poll fresh.
  const confirmedNoPreview = cachedEntry !== undefined && !cachedEntry.previewExists && regenKey === 0;

  // When regenKey increments: clear stale cache, then poll the signer directly
  // every 2s for up to 30s so we catch the async Lambda finishing.
  useEffect(() => {
    if (!regenKey || regenKey === 0) return;

    const cache = previewUrlCache;
    if (cache) cache.delete(image.raw_path);
    hasFetched.current = false;
    setSrc(null);
    setLoading(true);

    let cancelled = false;
    let attempts = 0;
    const MAX = 15; // 15 × 2 s = 30 s

    const poll = async () => {
      if (cancelled) return;
      attempts++;
      try {
        const url = await getPreviewUrl(image.raw_path);
        if (url) {
          if (!cancelled) { setSrc(url); setLoading(false); }
          return;
        }
      } catch { /* ignore transient errors */ }
      if (attempts < MAX) {
        setTimeout(poll, 2000);
      } else if (!cancelled) {
        setLoading(false); // give up — show placeholder
      }
    };

    // First check after 3 s (give the Lambda time to start)
    setTimeout(poll, 3000);
    return () => { cancelled = true; };
  }, [regenKey, image.raw_path]);

  // Trigger preview fetch only when card is in viewport
  useEffect(() => {
    const el = cardRef.current;
    if (!el) return;

    registerLazyCard(el, image.raw_path);

    const onVisible = () => {
      const rect = el.getBoundingClientRect();
      if (rect.top < window.innerHeight + 200 && rect.bottom > -200) {
        setIsIntersecting(true);
      }
    };

    onVisible();
    window.addEventListener("scroll", onVisible, { passive: true });
    window.addEventListener("resize", onVisible, { passive: true });

    return () => {
      window.removeEventListener("scroll", onVisible);
      window.removeEventListener("resize", onVisible);
      unregisterLazyCard(el);
    };
  }, [image.raw_path]);

  // Fetch preview URL when visible (works for all file types now)
  useEffect(() => {
    if (!isIntersecting || showRaw) return;

    // If batch already confirmed no preview for this key, skip all polling
    if (confirmedNoPreview) return;

    // Always check cache first
    if (cachedUrl) {
      setSrc(cachedUrl);
      return;
    }

    // Not in cache yet - trigger lazy batch fetch (no individual API call)
    if (hasFetched.current) return;
    hasFetched.current = true;

    setLoading(true);
    // Trigger lazy batch fetch for this image
    if (onFetchMissingUrl) {
      onFetchMissingUrl(image.raw_path);
    }
    // Poll for the cached URL (lazy batch will populate it)
    const pollTimer = setInterval(() => {
      const nextEntry = previewCache?.[image.raw_path];
      // When regenKey > 0 the cache entry is stale — don't stop on previewExists:false
      if (nextEntry !== undefined && !nextEntry.previewExists && regenKey === 0) {
        clearInterval(pollTimer);
        setLoading(false);
        return;
      }
      const nextCached = nextEntry?.previewUrl;
      if (nextCached) {
        clearInterval(pollTimer);
        setSrc(nextCached);
        setLoading(false);
      }
    }, 100);
    // Timeout after 5 seconds if cache doesn't populate
    setTimeout(() => {
      clearInterval(pollTimer);
      if (!src) setLoading(false);
    }, 5000);
  }, [isIntersecting, showRaw, image.raw_path, regenKey, cachedUrl, confirmedNoPreview, onFetchMissingUrl, previewCache, src]);

  if (showRaw) {
    return (
      <div style={{
        width: "100%", height: "100%",
        background: `linear-gradient(135deg, ${color}, ${color}dd)`,
        display: "flex", alignItems: "center", justifyContent: "center",
        flexDirection: "column", gap: "8px",
      }}>
        <span style={{ fontSize: "2rem", opacity: 0.6, color: "#333", fontFamily: "var(--font-mono)" }}>
          {ext || "RAW"}
        </span>
        <span style={{ fontSize: "0.625rem", color: "#555", opacity: 0.7, fontFamily: "var(--font-mono)" }}>
          RAW
        </span>
      </div>
    );
  }

  return (
    <div ref={cardRef}>
      {!src && (
        <div style={{
          width: "100%", height: "100%",
          background: `linear-gradient(135deg, ${color}, ${color}dd)`,
          display: "flex", alignItems: "center", justifyContent: "center",
        }}>
          {loading && <div className="spinner" />}
        </div>
      )}
      {src && (
        <img
          src={src}
          alt={image.name}
          loading="lazy"
          style={{
            width: "100%", height: "100%",
            objectFit: "cover",
            opacity: loading ? 0 : 1,
            transition: "opacity 0.3s ease",
          }}
        />
      )}
      {!src && !loading && (
        <div style={{
          width: "100%", height: "100%",
          background: `linear-gradient(135deg, ${color}, ${color}dd)`,
          display: "flex", alignItems: "center", justifyContent: "center",
        }}>
          <span style={{
            fontSize: "0.625rem",
            color: "#555",
            opacity: 0.5,
            fontFamily: "var(--font-mono)",
          }}>
            Preview unavailable
          </span>
        </div>
      )}
    </div>
  );
}

export default function ImageCard({
  image,
  onView,
  onDownload,
  // Multi-select props
  showCheckbox = false,
  isChecked = false,
  onToggle,
  // Compact mode: hide info + actions, show only image (for masonry)
  compact = false,
  // Preview URL cache from batch fetch
  previewCache = {},
  // Callback to trigger lazy batch fetching for missing URLs
  onFetchMissingUrl,
}) {
  const [regenerating, setRegenerating] = useState(false);
  const [regenKey, setRegenKey] = useState(0);
  const [isHovered, setIsHovered] = useState(false);

  const handleDownload = async (e) => {
    e.stopPropagation();
    try {
      const result = await getPreviewDownloadUrl(image.raw_path);
      if (result) {
        const a = document.createElement("a");
        a.href = result.url;
        a.download = result.fileName;
        a.target = "_blank";
        a.rel = "noopener noreferrer";
        document.body.appendChild(a);
        a.click();
        document.body.removeChild(a);
      } else {
        onDownload(image.raw_path);
      }
    } catch {
      onDownload(image.raw_path);
    }
  };

  const handleRegenerate = async (e) => {
    e.stopPropagation();
    setRegenerating(true);
    try {
      await regeneratePreview(image.raw_path);
      // Increment regenKey — ImagePreview will poll the signer every 2s for up to 30s
      setRegenKey((prev) => prev + 1);
    } catch (err) {
      alert(`Preview regeneration failed: ${err.message}`);
    } finally {
      setRegenerating(false);
    }
  };

  const ext = (image.ext || image.name.split(".").pop() || "").toUpperCase();

  return (
    <div
      className={`image-card${compact ? ' compact' : ''}`}
      style={{
        position: "relative",
        cursor: "pointer",
      }}
      onMouseEnter={() => compact && setIsHovered(true)}
      onMouseLeave={() => compact && setIsHovered(false)}
      onClick={() => {
        if (showCheckbox && onToggle) {
          onToggle();
        } else {
          onView(image);
        }
      }}
    >
      {/* ── Checkbox overlay (top-right) ──────────────────────────── */}
      {showCheckbox && (
        <div
          style={{
            position: "absolute",
            top: "8px", right: "8px", zIndex: 10,
            width: "24px", height: "24px",
            borderRadius: "4px",
            border: isChecked ? "2px solid #10b981" : "2px solid rgba(255,255,255,0.8)",
            background: isChecked ? "#10b981" : "rgba(0,0,0,0.3)",
            display: "flex", alignItems: "center", justifyContent: "center",
            cursor: "pointer", backdropFilter: "blur(4px)",
          }}
          onClick={(e) => {
            e.stopPropagation();
            if (onToggle) onToggle();
          }}
        >
          {isChecked && (
            <span style={{ color: "white", fontSize: "14px", fontWeight: "bold", lineHeight: 1 }}>✓</span>
          )}
        </div>
      )}

      {/* ── Selected outline ──────────────────────────────────────── */}
      {showCheckbox && isChecked && (
        <div style={{
          position: "absolute", inset: 0,
          border: "2px solid #10b981",
          borderRadius: "var(--radius)",
          pointerEvents: "none", zIndex: 5,
        }} />
      )}

      {/* ── Preview area ──────────────────────────────────────────── */}
      <div className="image-preview" style={{ position: "relative" }}>
        <ImagePreview image={image} showRaw={false} regenKey={regenKey} previewCache={previewCache} onFetchMissingUrl={onFetchMissingUrl} />
        {!compact && (
          <span style={{
            position: "absolute", top: "6px", left: "6px", zIndex: 5,
            background: "#fff", color: "#000", padding: "2px 6px",
            borderRadius: "4px", fontSize: "0.625rem",
            fontFamily: "var(--font-mono)", fontWeight: 600,
            letterSpacing: "0.05em",
          }}>{ext}</span>
        )}
      </div>

      {/* ── File info (non-compact only) ──────────────────────────── */}
      {!compact && (
        <div className="image-info">
          <div className="image-name" title={image.raw_path}>{image.name}</div>
          <div className="image-meta">
            <span>{formatSize(image.size_bytes)}</span>
            <span>{formatDate(image.updated_at)}</span>
          </div>
        </div>
      )}

      {/* ── Action buttons (always visible on top-right) ──────────── */}
      {compact && (
        <div style={{
          position: "absolute", top: "6px", right: "6px", zIndex: 10,
          display: "flex",
          gap: "2px",
        }}>
          <button
            className="btn-icon"
            onClick={(e) => { e.stopPropagation(); handleDownload(e); }}
            title="Download preview"
            style={{
              background: "rgba(0,0,0,0.5)",
              border: "none",
              color: "#fff",
              width: "28px",
              height: "28px",
              borderRadius: "4px",
              cursor: "pointer",
              fontSize: "0.875rem",
              display: "flex",
              alignItems: "center",
              justifyContent: "center",
            }}
          >↓</button>
          <button
            className="btn-icon"
            onClick={(e) => { e.stopPropagation(); handleRegenerate(e); }}
            title="Regenerate preview"
            disabled={regenerating}
            style={{
              background: "rgba(0,0,0,0.5)",
              border: "none",
              color: "#fff",
              width: "28px",
              height: "28px",
              borderRadius: "4px",
              cursor: regenerating ? "not-allowed" : "pointer",
              fontSize: "0.875rem",
              display: "flex",
              alignItems: "center",
              justifyContent: "center",
              opacity: regenerating ? 0.5 : 1,
            }}
          >↻</button>
        </div>
      )}
    </div>
  );
}
