/** Shared HTTP plumbing: base URL, auth token, JSON requests. */

export const API_BASE: string = import.meta.env.VITE_API_BASE_URL || "http://localhost:3001/api";

const TOKEN_KEY = "lawt_token";

function readToken(): string | null {
  try {
    return localStorage.getItem(TOKEN_KEY);
  } catch {
    return null;
  }
}

let authToken: string | null = readToken();

export function setAuthToken(token: string | null): void {
  authToken = token;
  try {
    if (token) localStorage.setItem(TOKEN_KEY, token);
    else localStorage.removeItem(TOKEN_KEY);
  } catch {
    // Storage can be unavailable (private mode); the in-memory token still works.
  }
}

export class ApiError extends Error {
  constructor(
    message: string,
    readonly status: number,
  ) {
    super(message);
    this.name = "ApiError";
  }
}

interface ErrorBody {
  error?: string;
}

async function errorMessage(response: Response): Promise<string> {
  const body = (await response.json().catch(() => ({ error: response.statusText }))) as ErrorBody;
  return body.error ?? `API error: ${response.status}`;
}

export interface RequestOptions extends RequestInit {
  /** Aborts the request when the signal fires. */
  signal?: AbortSignal;
}

/** Sends an authenticated request and parses the JSON body as `T`. */
export async function apiRequest<T>(endpoint: string, options: RequestOptions = {}): Promise<T> {
  const headers = new Headers(options.headers);
  if (!headers.has("Content-Type")) headers.set("Content-Type", "application/json");
  if (authToken) headers.set("Authorization", `Bearer ${authToken}`);

  const response = await fetch(`${API_BASE}${endpoint}`, { ...options, headers });

  if (response.status === 401) {
    setAuthToken(null);
    window.location.hash = "#login";
    throw new ApiError("Authentication required", 401);
  }
  if (!response.ok) throw new ApiError(await errorMessage(response), response.status);
  return (await response.json()) as T;
}
