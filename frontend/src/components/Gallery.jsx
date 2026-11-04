/**
 * Gallery Component
 * ───────────────────
 * Main gallery view with masonry/grid/list layout, filtering, and fullscreen viewer.
 * Multi-select mode: checkboxes on each image, batch "Generate Previews" at top.
 * Batch jobs persist to localStorage and auto-resume on page refresh.
 * Concurrency-limited (3 concurrent) with staggered delays to control costs.
 */
import { useState, useEffect, useRef, useCallback } from "react";
import ImageCard from "./ImageCard";
import { listImages, getGalleryNames, downloadImage, getPreviewUrl, getMediumUrl, formatSize, getColor, regeneratePreview, getPreviewDownloadUrl, batchGetDownloadUrls, getFullJpegUrl } from "../api";

const BATCH_JOB_KEY = "rawvaultBatchJob";
const CONCURRENCY = 3;
const STAGGER_DELAY = 500; // ms between batches

/**
 * Loads and displays a preview image in the fullscreen viewer.
 * Uses medium preview (1920px long edge, JPEG) for fast loading.
 * Uses batch-cached URLs when available.
 */
function ViewerImage({ image, previewCache }) {
  const [src, setSrc] = useState(null);
  const [loading, setLoading] = useState(true);
  const hasFetched = useRef(false);

  useEffect(() => {
    if (hasFetched.current) return;
    hasFetched.current = true;

    // Always fetch medium (1920px JPEG) for the fullscreen viewer.
    // Never use the thumbnail cache — that's 400px and looks bad at full size.
    getMediumUrl(image.raw_path)
      .then((url) => {
        if (url) { setSrc(url); setLoading(false); return; }
        // Medium not generated yet — fall back to thumbnail so viewer isn't blank
        const cached = previewCache[image.raw_path];
        if (cached?.previewUrl) { setSrc(cached.previewUrl); setLoading(false); return; }
        setLoading(false);
      })
      .catch(() => setLoading(false));
  }, [image.raw_path, previewCache]);

  const ext = (image.ext || image.name.split(".").pop() || "").toUpperCase();

  if (loading && !src) {
    return (
      <div style={{
        width: "100%", maxHeight: "70vh", maxWidth: "90vw",
        background: "var(--bg-secondary)",
        display: "flex", alignItems: "center", justifyContent: "center",
      }}>
        <div className="spinner" />
      </div>
    );
  }

  if (src) {
    return (
      <img src={src} alt={image.name} style={{ maxWidth: "90vw", maxHeight: "70vh", objectFit: "contain" }} />
    );
  }

  const color = getColor(image.name);
  return (
    <div style={{
      width: "100%", maxHeight: "70vh", maxWidth: "90vw",
      background: `linear-gradient(135deg, ${color}, ${color}dd)`,
      display: "flex", alignItems: "center", justifyContent: "center", flexDirection: "column", gap: "8px",
    }}>
      <span style={{ fontSize: "3rem", opacity: 0.5, color: "#333", fontFamily: "var(--font-mono)" }}>{ext || "RAW"}</span>
      <span style={{ fontSize: "0.875rem", color: "#555", opacity: 0.6, fontFamily: "var(--font-mono)" }}>
        {ext ? "RAW FILE" : "PREVIEW UNAVAILABLE"}
      </span>
    </div>
  );
}

/**
 * Cost estimation modal for batch preview generation.
 */
function CostEstimationModal({ totalImages, onConfirm, onCancel }) {
  const costPerInvocation = 0.00017;
  const totalCost = totalImages * costPerInvocation;
  const estimatedMinutes = Math.ceil(totalImages / (CONCURRENCY * 60 / 2));

  return (
    <div style={{
      position: "fixed", top: 0, left: 0, right: 0, bottom: 0,
      background: "rgba(0,0,0,0.5)", display: "flex", alignItems: "center",
      justifyContent: "center", zIndex: 2000,
    }} onClick={onCancel}>
      <div style={{
        background: "white", borderRadius: "var(--radius-lg)", padding: "32px",
        maxWidth: "420px", width: "90%",
        boxShadow: "0 20px 60px rgba(0,0,0,0.3)",
      }} onClick={(e) => e.stopPropagation()}>
        <h3 style={{ margin: "0 0 16px", fontSize: "1.125rem" }}>⚡ Batch Preview Generation</h3>
        <div style={{ marginBottom: "20px" }}>
          {[
            ["Images to process", totalImages.toString()],
            ["Estimated cost", `$${totalCost.toFixed(2)}`],
            ["Estimated time", `~${estimatedMinutes} min (3 concurrent)`],
            ["Output per image", "Small PNG + Medium JPEG"],
          ].map(([label, value]) => (
            <div key={label} style={{
              display: "flex", justifyContent: "space-between", marginBottom: "8px", fontSize: "0.875rem",
            }}>
              <span style={{ color: "var(--text-secondary)" }}>{label}</span>
              <span style={{ fontWeight: 600 }}>
                {label === "Estimated cost" ? (
                  <span style={{ color: "var(--accent)" }}>{value}</span>
                ) : value}
              </span>
            </div>
          ))}
        </div>
        <div style={{ display: "flex", gap: "12px", justifyContent: "flex-end" }}>
          <button className="btn btn-secondary" onClick={onCancel}>Cancel</button>
          <button className="btn btn-primary" onClick={onConfirm}>
            Generate {totalCost > 1 ? `($${totalCost.toFixed(2)})` : ""}
          </button>
        </div>
      </div>
    </div>
  );
}

export default function Gallery() {
  const [images, setImages] = useState([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState(null);
  const [selectedGallery, setSelectedGallery] = useState(() => {
    const saved = localStorage.getItem("selectedGallery");
    return saved || null;
  });
  const [galleries, setGalleries] = useState([]);
  const [viewMode, setViewMode] = useState("masonry");
  const [viewerImage, setViewerImage] = useState(null);

  // Preview URL cache: Map<raw_path, { previewUrl, mediumUrl }>
  const [previewCache, setPreviewCache] = useState({});

  // ── Lazy batch URL fetching ────────────────────────────────────────
  // Accumulates raw_paths as user scrolls, fetches in batches of 100
  const pendingBatchRef = useRef([]);
  const lazyBatchTimerRef = useRef(null);

  // Called when an ImageCard needs a URL but it's not in the cache
  const fetchMissingUrlBatch = useCallback(async (rawPath) => {
    // Already cached or already pending - do nothing
    if (previewCache[rawPath]?.previewUrl) return;
    if (pendingBatchRef.current.includes(rawPath)) return;

    // Add to pending batch
    pendingBatchRef.current.push(rawPath);

    // Debounce: wait for more paths to accumulate, then fire batch
    if (lazyBatchTimerRef.current) {
      clearTimeout(lazyBatchTimerRef.current);
    }
    lazyBatchTimerRef.current = setTimeout(async () => {
      const toFetch = pendingBatchRef.current.splice(0, 100); // batch of 100
      try {
        const requests = toFetch.map(key => ({ key, type: "thumbnail" }));
        const urls = await batchGetDownloadUrls(requests);
        setPreviewCache(prev => {
          const next = { ...prev };
          urls.forEach(u => {
            // Always record the entry so ImageCard knows the check ran
            // Only set previewUrl when a real preview was found
            next[u.key] = {
              previewUrl: u.preview_exists ? u.download_url : null,
              previewExists: u.preview_exists,
            };
          });
          return next;
        });
      } catch (err) {
        console.error("Lazy batch fetch failed for", toFetch.length, "images:", err.message);
      }
    }, 800); // 800ms debounce to accumulate a batch
  }, [previewCache]);

  const [multiSelectMode, setMultiSelectMode] = useState(false);
  const [selectedIds, setSelectedIds] = useState(new Set());
  const [batchRunning, setBatchRunning] = useState(false);
  const [batchProgress, setBatchProgress] = useState(null);
  const [showCostEstimate, setShowCostEstimate] = useState(false);
  const [batchTargetImages, setBatchTargetImages] = useState(0);
  const [pendingRawPaths, setPendingRawPaths] = useState([]); // raw paths for batch
  const [resumeInfo, setResumeInfo] = useState(null); // { from, total }

  // Ref to avoid stale closures in async callbacks
  const galleryRef = useRef(selectedGallery);
  const imagesRef = useRef(images);
  useEffect(() => { galleryRef.current = selectedGallery; }, [selectedGallery]);
  useEffect(() => { imagesRef.current = images; }, [images]);

  // ── Data fetching ──────────────────────────────────────────────────
  useEffect(() => { initGalleries(); }, []);

  // ── Resume batch job on page load ──────────────────────────────────
  useEffect(() => {
    const saved = localStorage.getItem(BATCH_JOB_KEY);
    if (!saved) return;

    try {
      const job = JSON.parse(saved);
      if (job.gallery === galleryRef.current && job.total > 0 && job.currentIndex < job.currentIndex) {
        // Don't auto-resume
      } else {
        localStorage.removeItem(BATCH_JOB_KEY);
      }
    } catch {
      localStorage.removeItem(BATCH_JOB_KEY);
    }
  }, [images.length]);

  // Monotonic id so a slow response for a previous gallery can't overwrite a newer one
  const fetchSeqRef = useRef(0);

  // Initial load: fetch gallery names only, then just the selected gallery's images.
  const initGalleries = async () => {
    setLoading(true); setError(null);
    try {
      const names = await getGalleryNames();
      setGalleries(names);
      if (names.length === 0) { setSelectedGallery(null); setImages([]); setLoading(false); return; }
      const saved = localStorage.getItem("selectedGallery");
      const initial = names.includes(saved) ? saved : names[0];
      setSelectedGallery(initial);
      await fetchImages(initial);
    } catch (err) {
      console.error("initGalleries error:", err);
      setError(err.message);
      setLoading(false);
    }
  };

  const fetchImages = async (gallery = selectedGallery) => {
    const seq = ++fetchSeqRef.current;
    setLoading(true); setError(null);
    try {
      const data = await listImages(gallery);
      if (seq !== fetchSeqRef.current) return; // superseded by a newer request
      if (Array.isArray(data)) setImages(data);
    } catch (err) {
      if (seq !== fetchSeqRef.current) return;
      console.error("fetchImages error:", err);
      setError(err.message);
    }
    finally { if (seq === fetchSeqRef.current) setLoading(false); }
  };

  const safeImages = Array.isArray(images) ? images : [];
  // When a gallery is selected the server already filtered by prefix — no client filter needed
  const filteredImages = selectedGallery ? safeImages : [];

  // ── Batch job persistence ──────────────────────────────────────────
  const saveBatchJob = (rawPaths, currentIndex, completed, failed, skipped) => {
    const job = {
      rawPaths,
      total: rawPaths.length,
      currentIndex,
      completed: completed + (currentIndex - completed - failed), // actually completed count
      failed,
      skipped,
      gallery: selectedGallery,
      timestamp: Date.now(),
    };
    // Recalculate true completed count
    job.completed = currentIndex - failed - skipped;
    localStorage.setItem(BATCH_JOB_KEY, JSON.stringify(job));
  };

  const clearBatchJob = () => {
    localStorage.removeItem(BATCH_JOB_KEY);
    setResumeInfo(null);
  };

  // ── Concurrency-limited batch processor ────────────────────────────
  const runBatch = async (rawPaths, concurrency = CONCURRENCY) => {
    let completed = 0, failed = 0, skipped = 0;
    const total = rawPaths.length;

    // Process in concurrency-limited batches
    for (let i = 0; i < total; i += concurrency) {
      const batch = rawPaths.slice(i, Math.min(i + concurrency, total));

      await Promise.all(batch.map(async (rawPath, batchIndex) => {
        const globalIndex = i + batchIndex;

        // Stagger delay between batches (not the first batch)
        if (i > 0) {
          await new Promise(r => setTimeout(r, STAGGER_DELAY));
        }

        try {
          const result = await regeneratePreview(rawPath);
          if (result?.status === "skipped") {
            skipped++;
          } else {
            completed++;
          }
        } catch {
          failed++;
        }

        const processed = completed + skipped;
        const current = globalIndex + 1;
        setBatchProgress({ current, total, completed, failed, skipped });
        saveBatchJob(rawPaths, current, completed, failed, skipped);
      }));
    }

    return { completed, failed, skipped, total };
  };

  // ── Handlers ───────────────────────────────────────────────────────
  const handleView = (image) => { if (!multiSelectMode) setViewerImage(image); };

  const handleDownload = async (rawPath) => {
    try {
      const { download_url, file_name } = await downloadImage(rawPath);
      const a = document.createElement("a");
      a.href = download_url; a.download = file_name;
      a.target = "_blank"; a.rel = "noopener noreferrer";
      document.body.appendChild(a); a.click();
      document.body.removeChild(a);
    } catch (err) { alert(`Download failed: ${err.message}`); }
  };

  const handleGalleryChange = (gallery) => {
    setSelectedGallery(gallery); setMultiSelectMode(false); setSelectedIds(new Set());
    localStorage.setItem("selectedGallery", gallery);
    fetchImages(gallery);
  };

  const toggleMultiSelect = () => {
    if (multiSelectMode) { setMultiSelectMode(false); setSelectedIds(new Set()); }
    else { setMultiSelectMode(true); setSelectedIds(new Set()); }
  };

  const toggleImageSelection = (imageId) => {
    const next = new Set(selectedIds);
    if (next.has(imageId)) next.delete(imageId); else next.add(imageId);
    setSelectedIds(next);
  };

  const toggleSelectAll = () => {
    const allIds = new Set(filteredImages.map((img) => img.id));
    if (selectedIds.size === allIds.size) setSelectedIds(new Set());
    else setSelectedIds(allIds);
  };

  // ── Batch generation ───────────────────────────────────────────────
  const openBatchConfirm = () => {
    setBatchTargetImages(filteredImages.length);
    setShowCostEstimate(true);
  };

  const confirmBatch = async () => {
    setShowCostEstimate(false);
    const rawPaths = filteredImages.map((img) => img.raw_path);
    setPendingRawPaths(rawPaths);
    setBatchRunning(true);
    setBatchProgress({ current: 0, total: rawPaths.length, completed: 0, failed: 0, skipped: 0 });

    const result = await runBatch(rawPaths);
    setSelectedIds(new Set()); setMultiSelectMode(false);
    await fetchImages();
    clearBatchJob();
    alert(`${result.completed} generated${result.failed > 0 ? ` · ${result.failed} failed` : ""}${result.skipped > 0 ? ` · ${result.skipped} skipped` : ""}`);
    setBatchRunning(false); setBatchProgress(null); setPendingRawPaths([]);
  };

  const handleBatchRegenerate = async () => {
    if (selectedIds.size === 0) return;
    const selectedImages = safeImages.filter((img) => selectedIds.has(img.id));
    const rawPaths = selectedImages.map((img) => img.raw_path);
    setPendingRawPaths(rawPaths);
    setBatchRunning(true);
    setBatchProgress({ current: 0, total: rawPaths.length, completed: 0, failed: 0, skipped: 0 });

    const result = await runBatch(rawPaths);
    setSelectedIds(new Set()); setMultiSelectMode(false);
    await fetchImages();
    clearBatchJob();
    alert(`${result.completed} generated${result.failed > 0 ? ` · ${result.failed} failed` : ""}${result.skipped > 0 ? ` · ${result.skipped} skipped` : ""}`);
    setBatchRunning(false); setBatchProgress(null); setPendingRawPaths([]);
  };

  // Resume button handler
  const handleResume = async () => {
    if (!resumeInfo || !batchProgress) return;
    const remaining = pendingRawPaths.slice(batchProgress.current);
    setResumeInfo(null);
    await runBatch(remaining);
    await fetchImages();
    clearBatchJob();
    setBatchRunning(false); setBatchProgress(null); setPendingRawPaths([]);
  };

  const selectedCount = selectedIds.size;

  // ── Render ─────────────────────────────────────────────────────────
  return (
    <>
      {showCostEstimate && (
        <CostEstimationModal
          totalImages={batchTargetImages}
          onConfirm={confirmBatch}
          onCancel={() => setShowCostEstimate(false)}
        />
      )}

      <div className="gallery-page">
        <div className="container">
          {/* Batch progress */}
          {batchRunning && batchProgress && (
            <div style={{
              marginBottom: "16px", padding: "16px 20px",
              background: "var(--accent-subtle)", border: "2px solid var(--accent)",
              borderRadius: "var(--radius)",
            }}>
              <div style={{ display: "flex", alignItems: "center", justifyContent: "space-between", marginBottom: "8px" }}>
                <span style={{ fontWeight: 600, fontSize: "0.875rem" }}>⚡ {resumeInfo ? "Resuming…" : "Generating Previews…"}</span>
                <span style={{ fontSize: "0.875rem", fontFamily: "var(--font-mono)", color: "var(--accent)" }}>
                  {batchProgress.current}/{batchProgress.total}
                  {batchProgress.failed > 0 && <span style={{ color: "#ef4444", marginLeft: "8px" }}>{batchProgress.failed} failed</span>}
                </span>
              </div>
              <div style={{ height: "6px", background: "#e5e7eb", borderRadius: "3px", overflow: "hidden" }}>
                <div style={{
                  height: "100%", width: `${(batchProgress.current / batchProgress.total) * 100}%`,
                  background: batchProgress.failed === 0 ? "#10b981" : "#f59e0b",
                  borderRadius: "3px", transition: "width 0.3s ease",
                }} />
              </div>
              <div style={{ display: "flex", justifyContent: "space-between", fontSize: "0.75rem", color: "var(--text-secondary)", marginTop: "6px" }}>
                <span>{batchProgress.completed} succeeded{batchProgress.skipped > 0 ? ` · ${batchProgress.skipped} skipped` : ""}</span>
                {resumeInfo && <span style={{ color: "var(--accent)", fontWeight: 500 }}>Resumed from {resumeInfo.from}</span>}
              </div>
            </div>
          )}

          {/* Resume banner — shown when a batch was interrupted */}
          {!batchRunning && resumeInfo && (
            <div style={{
              marginBottom: "16px", padding: "16px 20px",
              background: "#fff8e1", border: "2px solid #ffc107",
              borderRadius: "var(--radius)",
              display: "flex", alignItems: "center", justifyContent: "space-between",
            }}>
              <div>
                <span style={{ fontWeight: 600, fontSize: "0.875rem" }}>⚡ Previous batch job found</span>
                <span style={{ fontSize: "0.8125rem", color: "var(--text-secondary)", marginLeft: "8px" }}>
                  {resumeInfo.from}/{resumeInfo.total} images processed
                </span>
              </div>
              <button className="btn btn-primary" style={{ padding: "6px 16px", fontSize: "0.8125rem" }}
                onClick={handleResume}>Resume ({batchProgress ? batchProgress.total - batchProgress.current : 0} remaining)</button>
            </div>
          )}

          {/* Multi-select toolbar */}
          {multiSelectMode && (
            <div style={{
              marginBottom: "16px", padding: "12px 16px", background: "var(--accent-subtle)",
              border: "1px solid var(--accent)", borderRadius: "var(--radius)",
              display: "flex", alignItems: "center", justifyContent: "space-between",
              flexWrap: "wrap", gap: "12px",
            }}>
              <div style={{ display: "flex", alignItems: "center", gap: "16px" }}>
                <label style={{ display: "flex", alignItems: "center", gap: "6px", cursor: "pointer", fontSize: "0.8125rem" }}>
                  <input type="checkbox"
                    checked={selectedIds.size > 0 && selectedIds.size === filteredImages.length}
                    onChange={toggleSelectAll}
                    style={{ cursor: "pointer" }} />
                  <span style={{ fontWeight: 500 }}>
                    {selectedCount > 0 ? `${selectedCount} selected` : `Select all (${filteredImages.length})`}
                  </span>
                </label>
                <button className="btn btn-primary" style={{ padding: "6px 16px", fontSize: "0.8125rem", fontWeight: 600 }}
                  onClick={handleBatchRegenerate} disabled={selectedCount === 0}>
                  ⚡ Generate Previews ({selectedCount})
                </button>
              </div>
              <button className="btn btn-secondary" style={{ padding: "6px 12px", fontSize: "0.75rem" }}
                onClick={() => { setMultiSelectMode(false); setSelectedIds(new Set()); }}>✕ Cancel</button>
            </div>
          )}

          {/* Toolbar */}
          <div className="gallery-toolbar">
            <h2>
              {selectedGallery}
              <span style={{ marginLeft: 8, color: "var(--text-secondary)", fontSize: "0.875rem" }}>
                ({filteredImages.length})
              </span>
            </h2>
            <div className="gallery-filters">
              <select className="filter-select" value={selectedGallery}
                onChange={(e) => handleGalleryChange(e.target.value)} disabled={galleries.length === 0}>
                {galleries.map((g) => (<option key={g} value={g}>{g}</option>))}
              </select>

              <button className="btn btn-secondary" style={{ padding: "6px 12px", fontSize: "0.75rem" }}
                onClick={openBatchConfirm} title="Generate previews for ALL images (shows cost estimate)">
                ⚡ Generate All
              </button>

              <button className="btn-icon" onClick={toggleMultiSelect}
                title={multiSelectMode ? "Cancel selection" : "Select multiple images"}
                style={{ background: multiSelectMode ? "var(--accent)" : "transparent",
                  color: multiSelectMode ? "white" : "var(--text-secondary)" }}>☑</button>

              <div className="view-toggle">
                <button className={viewMode === "masonry" ? "active" : ""} onClick={() => setViewMode("masonry")} title="Masonry">⊞</button>
                <button className={viewMode === "grid" ? "active" : ""} onClick={() => setViewMode("grid")} title="Grid">⊟</button>
                <button className={viewMode === "list" ? "active" : ""} onClick={() => setViewMode("list")} title="List">☰</button>
              </div>
            </div>
          </div>

          {/* Content */}
          {loading ? (
            <div style={{ display: "flex", justifyContent: "center", padding: "80px 0" }}><div className="spinner" /></div>
          ) : error ? (
            <div className="empty-state">
              <div className="empty-state-icon">⚠️</div>
              <h3>Failed to load</h3><p>{error}</p>
              <button className="btn btn-primary" onClick={() => fetchImages()}>Retry</button>
            </div>
          ) : filteredImages.length === 0 ? (
            <div className="empty-state">
              <div className="empty-state-icon">📷</div>
              <h3>No images found</h3><p>Upload RAW files to get started</p>
            </div>
          ) : (
            <>
              {viewMode === "masonry" && (
                <div className="image-grid masonry">
                  {filteredImages.map((img) => (
                    <ImageCard key={img.id} image={img} onView={handleView} onDownload={handleDownload}
                      compact={true}
                      previewCache={previewCache}
                      onFetchMissingUrl={fetchMissingUrlBatch}
                      showCheckbox={multiSelectMode} isChecked={selectedIds.has(img.id)}
                      onToggle={() => toggleImageSelection(img.id)} />
                  ))}
                </div>
              )}

              {viewMode === "grid" && (
                <div className="image-grid">
                  {filteredImages.map((img) => (
                    <ImageCard key={img.id} image={img} onView={handleView} onDownload={handleDownload}
                      previewCache={previewCache}
                      onFetchMissingUrl={fetchMissingUrlBatch}
                      showCheckbox={multiSelectMode} isChecked={selectedIds.has(img.id)}
                      onToggle={() => toggleImageSelection(img.id)} />
                  ))}
                </div>
              )}

              {viewMode === "list" && (
                <div style={{ display: "flex", flexDirection: "column", gap: "8px" }}>
                  {filteredImages.map((img) => {
                    const ext = (img.ext || img.name.split(".").pop() || "").toUpperCase();
                    const isSelected = selectedIds.has(img.id);
                    return (
                      <div key={img.id} className="card" style={{
                        display: "flex", alignItems: "center", padding: "12px 16px",
                        cursor: multiSelectMode ? "default" : "pointer",
                        border: isSelected ? "2px solid var(--accent)" : undefined,
                      }}
                        onClick={() => { if (multiSelectMode) toggleImageSelection(img.id); else handleView(img); }}>
                        {multiSelectMode && (
                          <input type="checkbox" checked={isSelected}
                            onChange={(e) => { e.stopPropagation(); toggleImageSelection(img.id); }}
                            onClick={(e) => e.stopPropagation()}
                            style={{ marginRight: "12px", cursor: "pointer" }} />
                        )}
                        <div style={{
                          width: "48px", height: "32px", background: getColor(img.name),
                          borderRadius: "4px", marginRight: "12px", display: "flex",
                          alignItems: "center", justifyContent: "center", fontSize: "0.5rem",
                          fontFamily: "var(--font-mono)", color: "#333", opacity: 0.8,
                        }}>{ext}</div>
                        <div style={{ flex: 1, minWidth: 0 }}>
                          <div style={{ fontWeight: 500, fontSize: "0.875rem", whiteSpace: "nowrap",
                            overflow: "hidden", textOverflow: "ellipsis" }}>{img.path}</div>
                          <div style={{ fontSize: "0.75rem", color: "var(--text-secondary)" }}>{img.gallery}</div>
                        </div>
                        <div style={{ fontSize: "0.75rem", color: "var(--text-secondary)", marginRight: "16px", whiteSpace: "nowrap" }}>
                          {formatSize(img.size_bytes)}
                        </div>
                        <button className="btn-icon"
                          onClick={(e) => { e.stopPropagation(); handleDownload(img.raw_path); }}
                          title="Download RAW">↓</button>
                      </div>
                    );
                  })}
                </div>
              )}
            </>
          )}
        </div>
      </div>

      {/* Fullscreen viewer overlay */}
      {viewerImage && (
        <div className="image-viewer-overlay" onClick={() => setViewerImage(null)}>
          <button className="btn-icon" onClick={() => setViewerImage(null)}
            style={{ fontSize: "1.5rem", position: "absolute", top: "16px", right: "16px", zIndex: 1001 }}>✕</button>
          <ViewerImage image={viewerImage} previewCache={previewCache} />
          <div className="image-viewer-info">
            <div className="image-viewer-name">{viewerImage.path}</div>
            <div className="image-viewer-meta">
              {formatSize(viewerImage.size_bytes)} · {viewerImage.gallery} · {(viewerImage.ext || viewerImage.name.split(".").pop() || "").toUpperCase()}
            </div>
            <div style={{ display: "flex", gap: "8px", marginTop: "12px", justifyContent: "center", flexWrap: "wrap" }}>
              <button className="btn btn-secondary"
                onClick={async (e) => {
                  e.stopPropagation();
                  try {
                    const result = await getPreviewDownloadUrl(viewerImage.raw_path);
                    if (result) {
                      const a = document.createElement("a");
                      a.href = result.url; a.download = result.fileName;
                      a.target = "_blank"; a.rel = "noopener noreferrer";
                      document.body.appendChild(a); a.click(); document.body.removeChild(a);
                    } else { handleDownload(viewerImage.raw_path); }
                  } catch (err) { handleDownload(viewerImage.raw_path); }
                }}>
                Download Preview
              </button>
              <button className="btn btn-secondary"
                onClick={async (e) => {
                  e.stopPropagation();
                  try {
                    const url = await getFullJpegUrl(viewerImage.raw_path);
                    if (url) {
                      const a = document.createElement("a");
                      a.href = url;
                      a.download = viewerImage.name + ".full.jpeg";
                      a.target = "_blank"; a.rel = "noopener noreferrer";
                      document.body.appendChild(a); a.click(); document.body.removeChild(a);
                    } else {
                      alert("Full JPEG not yet generated — click Regenerate Preview first.");
                    }
                  } catch (err) { alert(`Download failed: ${err.message}`); }
                }}>
                Download Full JPEG
              </button>
              <button className="btn btn-secondary"
                onClick={(e) => { e.stopPropagation(); handleDownload(viewerImage.raw_path); }}>
                Download Original
              </button>
              <button className="btn btn-outline" id={`regen-${viewerImage.id}`} style={{ padding: "8px 16px", borderRadius: "8px", border: "1px solid var(--accent)", color: "var(--accent)", background: "transparent", cursor: "pointer", fontSize: "0.8125rem" }}
                onClick={async (e) => {
                  e.stopPropagation();
                  const btn = document.getElementById(`regen-${viewerImage.id}`);
                  btn.textContent = "Generating..."; btn.disabled = true;
                  try {
                    await regeneratePreview(viewerImage.raw_path);
                    await new Promise(r => setTimeout(r, 3000));
                    btn.textContent = "Regenerate";
                  } catch (err) { alert(`Regeneration failed: ${err.message}`); btn.textContent = "Regenerate"; }
                  btn.disabled = false;
                }}>
                Regenerate Preview
              </button>
            </div>
          </div>
        </div>
      )}
    </>
  );
}
