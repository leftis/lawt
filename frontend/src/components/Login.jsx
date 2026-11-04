/**
 * Login Component
 * ─────────────────
 * Supports:
 * - Mock login (local dev) — any credentials, or "Continue as Guest"
 * - Cognito (production) — AWS Amplify / Cognito Hosted UI auth flow
 */
import { useState, useEffect } from "react";
import { Logo } from "./Logo";
import { setAuthToken, exchangeCode } from "../api";

const IS_LOCAL = import.meta.env.VITE_API_BASE_URL?.includes("localhost");
const COGNITO_ENABLED = !IS_LOCAL && import.meta.env.VITE_COGNITO_USER_POOL_ID;

export default function Login({ onLogin }) {
  const [email, setEmail] = useState("");
  const [password, setPassword] = useState("");
  const [error, setError] = useState("");
  const [loading, setLoading] = useState(false);

  useEffect(() => {
    // Check for existing token
    const token = localStorage.getItem("lawt_token");
    if (token && !IS_LOCAL) {
      setAuthToken(token);
      onLogin();
      return;
    }

    // Check if Cognito redirected back with a code
    if (COGNITO_ENABLED && !IS_LOCAL) {
      const params = new URLSearchParams(window.location.search);
      const code = params.get("code");
      const errorParam = params.get("error");

      if (errorParam) {
        setError(`Login failed: ${errorParam}`);
        // Clean up URL
        window.history.replaceState({}, "", window.location.pathname);
        return;
      }

      if (code) {
        setLoading(true);
        setError("");
        // Exchange code for token
        const redirectUri =
          import.meta.env.VITE_COGNITO_LOGIN_REDIRECT || window.location.origin + "/";
        exchangeCode(code, redirectUri)
          .then((data) => {
            setAuthToken(data.id_token || data.access_token); // the API authorizer validates ID tokens
            window.history.replaceState({}, "", window.location.pathname);
            onLogin();
          })
          .catch((err) => {
            setError(err.message);
            window.history.replaceState({}, "", window.location.pathname);
            setLoading(false);
          });
      }
    }
  }, [onLogin, COGNITO_ENABLED, IS_LOCAL]);

  const handleLogin = async (e) => {
    e.preventDefault();
    setError("");
    setLoading(true);

    if (IS_LOCAL) {
      // Mock login for local development
      await new Promise((r) => setTimeout(r, 500)); // Simulate network
      if (email) {
        setAuthToken("mock-jwt-token");
        onLogin();
      } else {
        setError("Please enter any email to continue");
      }
    } else if (COGNITO_ENABLED) {
      // Production: Cognito Hosted UI
      try {
        const cognitoHostedUI = buildCognitoHostedURL();
        window.location.href = cognitoHostedUI;
      } catch (err) {
        setError(err.message);
        setLoading(false);
      }
    } else {
      setError("Cognito not configured. Set VITE_COGNITO_* env vars.");
      setLoading(false);
    }
  };

  function buildCognitoHostedURL() {
    const cognitoDomain = import.meta.env.VITE_COGNITO_HOSTED_UI_DOMAIN;
    const clientId = import.meta.env.VITE_COGNITO_USER_POOL_CLIENT_ID;
    const loginRedirect =
      import.meta.env.VITE_COGNITO_LOGIN_REDIRECT || window.location.origin + "/";

    return (
      `https://${cognitoDomain}/login` +
      `?response_type=code` +
      `&client_id=${clientId}` +
      `&redirect_uri=${encodeURIComponent(loginRedirect)}`
    );
  }

  function handleLogout() {
    setAuthToken(null);
    const clientId = import.meta.env.VITE_COGNITO_USER_POOL_CLIENT_ID;
    const logoutRedirect = window.location.origin + "/";
    const cognitoDomain = import.meta.env.VITE_COGNITO_HOSTED_UI_DOMAIN;

    if (COGNITO_ENABLED && clientId) {
      const url = `https://${cognitoDomain}/logout` +
        `?client_id=${clientId}` +
        `&logout_uri=${encodeURIComponent(logoutRedirect)}`;
      window.location.href = url;
    } else {
      window.location.hash = "#login";
      window.location.reload();
    }
  }

  return (
    <div className="login-page">
      <div className="login-card">
        <div className="login-header">
          <div className="login-logo"><Logo size={56} /></div>
          <h1>lawt</h1>
          <p>{IS_LOCAL ? "Local Development Mode" : "RAW Image Preview System"}</p>
        </div>

        {!IS_LOCAL && COGNITO_ENABLED && (
          <div className="login-cognito">
            <button type="button" className="btn btn-primary" onClick={handleLogin}>
              {loading ? "Signing in..." : "Sign in with AWS Cognito"}
            </button>
            <p className="login-hint">You will be redirected to the secure AWS login page</p>
          </div>
        )}

        {IS_LOCAL && (
          <form className="login-form" onSubmit={handleLogin}>
            <input
              type="email"
              className="input"
              placeholder="Email address"
              value={email}
              onChange={(e) => setEmail(e.target.value)}
              autoComplete="email"
            />
            <input
              type="password"
              className="input"
              placeholder="Password"
              value={password}
              onChange={(e) => setPassword(e.target.value)}
              autoComplete="current-password"
            />

            <div className="login-error">{error}</div>

            <button type="submit" className="btn btn-primary" disabled={loading}>
              {loading ? "Signing in..." : "Continue"}
            </button>

            <button
              type="button"
              className="btn btn-secondary"
              onClick={() => {
                setAuthToken("mock-jwt-token");
                onLogin();
              }}
            >
              Continue as Guest
            </button>
          </form>
        )}

        {error && !IS_LOCAL && COGNITO_ENABLED && (
          <div className="login-error" style={{ marginTop: "12px" }}>{error}</div>
        )}

        <div className="login-footer">
          {IS_LOCAL
            ? "Mock auth — skip password for development"
            : COGNITO_ENABLED
              ? "Authorized users only"
              : "Configure Cognito to enable auth"}
        </div>
      </div>
    </div>
  );
}
