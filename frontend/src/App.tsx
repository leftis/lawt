/**
 * lawt — Main App
 * ─────────────────────
 * Handles routing between login and gallery views.
 */
import { useState, useEffect } from "react";
import Login from "./components/Login";
import Gallery from "./components/Gallery";
import { setAuthToken } from "./api";
import TimelineView from "./timeline/TimelineView";

/**
 * Chooses the UI. The timeline is the default; `VITE_UI=legacy` flips the default, and
 * `?ui=legacy` / `?ui=timeline` switch it for this browser (remembered, because the login
 * redirect drops the query string).
 */
function useTimelineUi(): boolean {
  const KEY = "lawt_ui";
  const defaultIsTimeline = import.meta.env.VITE_UI !== "legacy";
  const requested = new URLSearchParams(window.location.search).get("ui");
  try {
    if (requested === "timeline" || requested === "legacy") localStorage.setItem(KEY, requested);
    const stored = localStorage.getItem(KEY);
    return stored === null ? defaultIsTimeline : stored === "timeline";
  } catch {
    return requested === null ? defaultIsTimeline : requested === "timeline";
  }
}

const USE_TIMELINE = useTimelineUi();

export default function App() {
  const [isAuthenticated, setIsAuthenticated] = useState(false);
  const [checking, setChecking] = useState(true);

  const handleCodeRedirect = async (): Promise<void> => {
    const params = new URLSearchParams(window.location.search);
    const code = params.get("code");
    if (!code) return;

    try {
      const redirectUri = import.meta.env.VITE_COGNITO_LOGIN_REDIRECT || window.location.origin + "/";
      const response = await fetch(
        `${import.meta.env.VITE_API_BASE_URL}/auth/token`,
        {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({ code, redirect_uri: redirectUri }),
        }
      );

      if (response.ok) {
        // API Gateway's Cognito authorizer validates ID tokens, not access tokens.
        const data = (await response.json()) as { id_token?: string; access_token: string };
        setAuthToken(data.id_token ?? data.access_token);
        window.history.replaceState({}, "", window.location.pathname);
        setIsAuthenticated(true);
        window.location.hash = "#gallery";
      }
    } catch (err) {
      console.error("Token exchange error:", err);
    }
  };

  useEffect(() => {
    const checkAuth = () => {
      const hash = window.location.hash;
      const token = localStorage.getItem("lawt_token");
      if (hash === "#login" || !token) {
        setIsAuthenticated(false);
      } else {
        setIsAuthenticated(true);
      }
      setChecking(false);
    };

    // Handle Cognito redirect with code (Login.jsx also handles this)
    const params = new URLSearchParams(window.location.search);
    const code = params.get("code");
    const IS_LOCAL = import.meta.env.VITE_API_BASE_URL?.includes("localhost");
    const COGNITO_ENABLED = !IS_LOCAL && import.meta.env.VITE_COGNITO_USER_POOL_ID;

    if (code && COGNITO_ENABLED) {
      handleCodeRedirect().then(checkAuth);
    } else {
      checkAuth();
    }
  }, []);

  const handleLogin = () => {
    setIsAuthenticated(true);
    window.location.hash = "#gallery";
  };

  const handleLogout = () => {
    setAuthToken(null);
    setIsAuthenticated(false);
    window.location.hash = "#login";
  };

  if (checking) {
    return (
      <div style={{
        display: "flex",
        justifyContent: "center",
        alignItems: "center",
        height: "100vh",
        background: "var(--bg-primary)",
      }}>
        <div className="spinner" />
      </div>
    );
  }
  if (isAuthenticated) return USE_TIMELINE ? <TimelineView onLogout={handleLogout} /> : <Gallery />;
  return (
    <Login onLogin={handleLogin} />
  );
}
