import type { ApiError } from "./types";

const API_BASE = "/api/v1";

class ApiClient {
  private refreshPromise: Promise<boolean> | null = null;
  private isAuthenticated = false;

  setAuthenticated(authenticated: boolean) {
    this.isAuthenticated = authenticated;
  }

  getIsAuthenticated() {
    return this.isAuthenticated;
  }

  private async refreshAccessToken(): Promise<boolean> {
    if (this.refreshPromise) {
      return this.refreshPromise;
    }

    this.refreshPromise = (async () => {
      try {
        const response = await fetch(`${API_BASE}/auth/refresh`, {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          credentials: "include",
          body: JSON.stringify({}),
        });

        if (!response.ok) {
          this.isAuthenticated = false;
          return false;
        }

        return true;
      } catch {
        return false;
      } finally {
        this.refreshPromise = null;
      }
    })();

    return this.refreshPromise;
  }

  private async request<T>(
    endpoint: string,
    options: RequestInit = {},
    isRetry = false,
  ): Promise<T> {
    const headers: HeadersInit = {
      "Content-Type": "application/json",
      ...options.headers,
    };

    const response = await fetch(`${API_BASE}${endpoint}`, {
      ...options,
      headers,
      credentials: "include",
    });

    const isAuthEndpoint =
      endpoint === "/auth/login" || endpoint === "/auth/register" || endpoint === "/auth/refresh";
    if (response.status === 401 && !isRetry && !isAuthEndpoint) {
      const refreshed = await this.refreshAccessToken();
      if (refreshed) {
        return this.request<T>(endpoint, options, true);
      }
    }

    if (!response.ok) {
      const error: ApiError = {
        message: response.statusText,
        status: response.status,
      };
      try {
        const data = await response.json();
        if (data.error && typeof data.error === "object") {
          error.message = data.error.message || response.statusText;
          error.code = data.error.code;
          error.details = data.error.details;
        } else {
          error.message = data.message || data.error || response.statusText;
        }
      } catch {
        // Response body is not JSON
      }
      throw error;
    }

    if (response.status === 204) {
      return undefined as T;
    }

    return response.json();
  }

  get<T>(endpoint: string) {
    return this.request<T>(endpoint, { method: "GET" });
  }

  post<T>(endpoint: string, data?: unknown) {
    return this.request<T>(endpoint, {
      method: "POST",
      body: data ? JSON.stringify(data) : undefined,
    });
  }

  put<T>(endpoint: string, data?: unknown) {
    return this.request<T>(endpoint, {
      method: "PUT",
      body: data ? JSON.stringify(data) : undefined,
    });
  }

  delete<T>(endpoint: string) {
    return this.request<T>(endpoint, { method: "DELETE" });
  }
}

export const apiClient = new ApiClient();
