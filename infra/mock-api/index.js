/**
 * RawVault Mock API Server
 * ───────────────────────────
 * A lightweight Express server that mimics the RawVault REST API
 * for local frontend development without AWS.
 *
 * Endpoints:
 *   GET  /health                        → Health check (no auth)
 *   GET  /images                        → List images (JWT required)
 *   GET  /images/download?key=<path>    → Presigned URL (JWT required)
 *
 * Auth: Accepts a mock JWT in the Authorization header:
 *       Authorization: Bearer mock-jwt-token
 *       (Or skip auth by setting SKIP_AUTH=true)
 */

import express from "express";
import cors from "cors";
import crypto from "node:crypto";
import path from "node:path";

const app = express();
const PORT = Number(process.env.MOCK_PORT) || 3001;
const SKIP_AUTH = process.env.SKIP_AUTH === "true";

// ── Middleware ─────────────────────────────────
app.use(cors());
app.use(express.json());

// ── Simple JWT mock validator ──────────────────
const VALID_MOCK_TOKEN = "mock-jwt-token";

function authenticate(req, res, next) {
  if (SKIP_AUTH) return next();

  const authHeader = req.headers.authorization;
  if (!authHeader || !authHeader.startsWith("Bearer ")) {
    return res.status(401).json({ error: "Missing or invalid Authorization header" });
  }

  const token = authHeader.split(" ")[1];
  if (token !== VALID_MOCK_TOKEN) {
    return res.status(401).json({ error: "Invalid or expired token" });
  }

  // Attach mock user info
  req.user = { sub: "mock-user-id", email: "dev@rawvault.local" };
  next();
}

// ── Mock Image Catalog ─────────────────────────
// Generate deterministic mock data for local dev
const MOCK_IMAGES = [
  // Fuji RAF files
  {
    id: "galleries/travel_2026/DSCF0001",
    name: "DSCF0001",
    raw_path: "galleries/travel_2026/DSCF0001.RAF",
    preview_path: "galleries/travel_2026/DSCF0001.preview.png",
    size_bytes: 38452736,
    updated_at: "2026-06-01T08:15:00Z",
    gallery: "travel_2026",
  },
  {
    id: "galleries/travel_2026/DSCF0002",
    name: "DSCF0002",
    raw_path: "galleries/travel_2026/DSCF0002.RAF",
    preview_path: "galleries/travel_2026/DSCF0002.preview.png",
    size_bytes: 41028608,
    updated_at: "2026-06-01T08:22:00Z",
    gallery: "travel_2026",
  },
  {
    id: "galleries/travel_2026/DSCF0003",
    name: "DSCF0003",
    raw_path: "galleries/travel_2026/DSCF0003.RAF",
    preview_path: "galleries/travel_2026/DSCF0003.preview.png",
    size_bytes: 35128832,
    updated_at: "2026-06-01T08:35:00Z",
    gallery: "travel_2026",
  },
  // DNG files
  {
    id: "galleries/travel_2026/IMG_0100",
    name: "IMG_0100",
    raw_path: "galleries/travel_2026/IMG_0100.DNG",
    preview_path: "galleries/travel_2026/IMG_0100.preview.png",
    size_bytes: 47892480,
    updated_at: "2026-06-02T14:05:00Z",
    gallery: "travel_2026",
  },
  {
    id: "galleries/travel_2026/IMG_0101",
    name: "IMG_0101",
    raw_path: "galleries/travel_2026/IMG_0101.DNG",
    preview_path: "galleries/travel_2026/IMG_0101.preview.png",
    size_bytes: 52314624,
    updated_at: "2026-06-02T14:12:00Z",
    gallery: "travel_2026",
  },
  {
    id: "galleries/wedding_2026/C100_0001",
    name: "C100_0001",
    raw_path: "galleries/wedding_2026/C100_0001.RAF",
    preview_path: "galleries/wedding_2026/C100_0001.preview.png",
    size_bytes: 33554432,
    updated_at: "2026-05-20T11:00:00Z",
    gallery: "wedding_2026",
  },
  {
    id: "galleries/wedding_2026/C100_0002",
    name: "C100_0002",
    raw_path: "galleries/wedding_2026/C100_0002.RAF",
    preview_path: "galleries/wedding_2026/C100_0002.preview.png",
    size_bytes: 37219328,
    updated_at: "2026-05-20T11:05:00Z",
    gallery: "wedding_2026",
  },
  {
    id: "galleries/studio_2026/STUDIO_001",
    name: "STUDIO_001",
    raw_path: "galleries/studio_2026/STUDIO_001.DNG",
    preview_path: "galleries/studio_2026/STUDIO_001.preview.png",
    size_bytes: 61236736,
    updated_at: "2026-06-03T09:30:00Z",
    gallery: "studio_2026",
  },
];

// ── Helper: Format file size ───────────────────
function formatSize(bytes) {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1048576) return `${(bytes / 1024).toFixed(1)} KB`;
  if (bytes < 1073741824) return `${(bytes / 1048576).toFixed(1)} MB`;
  return `${(bytes / 1073741824).toFixed(2)} GB`;
}

// ── Helper: Generate mock presigned URL ───────
function generateMockPresignedUrl(key) {
  const expiry = Date.now() + 15 * 60 * 1000; // 15 min
  const signature = crypto.randomBytes(16).toString("hex");
  return (
    `https://rawvault-storage.s3.amazonaws.com/${key}` +
    `?AWSAccessKeyId=mock-access-key` +
    `&Expires=${Math.floor(expiry / 1000)}` +
    `&Signature=${signature}`
  );
}

// ── Routes ─────────────────────────────────────

// Health check (no auth)
app.get("/health", (_req, res) => {
  res.json({
    status: "ok",
    service: "rawvault-mock-api",
    auth: SKIP_AUTH ? "disabled" : "enabled",
    timestamp: new Date().toISOString(),
  });
});

// GET /images — list all images (with optional gallery filter)
app.get("/api/images", authenticate, (req, res) => {
  let images = MOCK_IMAGES;

  // ?galleries=1 → names only (mirrors the real list-images Lambda)
  if (req.query.galleries) {
    return res.json([...new Set(MOCK_IMAGES.map((i) => i.gallery))].sort());
  }

  // Filter by gallery if query param provided
  const gallery = req.query.gallery;
  if (gallery) {
    images = images.filter((img) => img.gallery === gallery);
  }

  // Sort by date (newest first)
  images = [...images].sort(
    (a, b) => new Date(b.updated_at).getTime() - new Date(a.updated_at).getTime()
  );

  // Attach formatted size for convenience
  const result = images.map((img) => ({
    ...img,
    size_formatted: formatSize(img.size_bytes),
  }));

  res.json(result);
});

// GET /images/download?key=...
app.get("/api/images/download", authenticate, (req, res) => {
  const key = req.query.key;

  if (!key) {
    return res.status(400).json({ error: "Missing 'key' query parameter" });
  }

  // Validate the key is a known image
  const image = MOCK_IMAGES.find(
    (img) => img.raw_path === key || img.preview_path === key
  );

  if (!image) {
    return res.status(404).json({ error: `Image not found: ${key}` });
  }

  res.json({
    download_url: generateMockPresignedUrl(image.raw_path),
    expires_in: 900, // 15 minutes
    file_name: path.basename(image.raw_path),
    file_size: image.size_bytes,
  });
});

// ── Serve mock preview images ──────────────────
// Generate simple placeholder images on the fly for local dev
const { createCanvas } = (() => {
  // Use a simple approach: return colored placeholder data URLs
  return {
    createCanvas: () => null,
  };
})();

// Serve a simple placeholder image
function createPlaceholderImage(width, height, color) {
  const canvas = new (require("canvas") || {});
  // For now, return a simple colored PNG as base64
  const svg = `<svg xmlns="http://www.w3.org/2000/svg" width="${width}" height="${height}">
    <rect width="100%" height="100%" fill="${color}"/>
    <text x="50%" y="50%" dominant-baseline="middle" text-anchor="middle"
          font-family="monospace" font-size="14" fill="white">Preview</text>
  </svg>`;
  return svg;
}

// ── Start Server ───────────────────────────────
app.listen(PORT, "0.0.0.0", () => {
  console.log(`\n  RawVault Mock API Server`);
  console.log(`  ──────────────────────`);
  console.log(`  Running at:    http://localhost:${PORT}`);
  console.log(`  API at:        http://localhost:${PORT}/api`);
  console.log(`  Auth:          ${SKIP_AUTH ? "DISABLED (dev mode)" : "REQUIRED (Bearer mock-jwt-token)"}`);
  console.log(`  Mock images:   ${MOCK_IMAGES.length}`);
  console.log(`  Galleries:     [...new Set(MOCK_IMAGES.map(i => i.gallery)).join(", ")]`);
  console.log(`  ──────────────────────\n`);
});
