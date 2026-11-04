/**
 * Header Component
 * ─────────────────
 * Sticky header with logo, gallery filter, view controls, and logout
 */
import { Logo } from "./Logo";

export default function Header({
  user,
  onLogout,
  galleries,
  selectedGallery,
  onGalleryChange,
  viewMode,
  onViewModeChange,
}) {
  return (
    <header className="header">
      <div className="header-inner">
        <a href="#" className="logo" onClick={(e) => e.preventDefault()}>
          <Logo size={28} />
          lawt
        </a>

        <div className="header-actions">
          {/* Gallery filter */}
          <select
            className="filter-select"
            value={selectedGallery}
            onChange={(e) => onGalleryChange(e.target.value)}
          >
            {galleries.map((g) => (
              <option key={g} value={g}>
                {g === "all" ? "All Galleries" : g}
              </option>
            ))}
          </select>

          {/* View mode toggle */}
          <div className="view-toggle">
            <button
              className={viewMode === "grid" ? "active" : ""}
              onClick={() => onViewModeChange("grid")}
              title="Grid view"
            >
              ⊞
            </button>
            <button
              className={viewMode === "list" ? "active" : ""}
              onClick={() => onViewModeChange("list")}
              title="List view"
            >
              ☰
            </button>
          </div>

          {/* User badge */}
          <div className="user-badge">
            <span className="user-dot" />
            {user?.email || "dev@lawt.local"}
          </div>

          {/* Logout */}
          <button className="btn-icon" onClick={onLogout} title="Logout">
            ↗
          </button>
        </div>
      </div>
    </header>
  );
}
