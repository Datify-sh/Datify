import * as React from "react";
import { useQueryClient } from "@tanstack/react-query";
import { authApi, apiClient } from "@/lib/api";
import type { UserResponse, LoginRequest, RegisterRequest } from "@/lib/api";

interface AuthContextValue {
  user: UserResponse | null;
  isLoading: boolean;
  isAuthenticated: boolean;
  authError: string | null;
  login: (data: LoginRequest) => Promise<void>;
  register: (data: RegisterRequest) => Promise<void>;
  logout: () => Promise<void>;
  clearAuthError: () => void;
}

const AuthContext = React.createContext<AuthContextValue | null>(null);

export function useAuth() {
  const context = React.useContext(AuthContext);
  if (!context) {
    throw new Error("useAuth must be used within an AuthProvider");
  }
  return context;
}

export function AuthProvider({ children }: { children: React.ReactNode }) {
  const queryClient = useQueryClient();
  const [user, setUser] = React.useState<UserResponse | null>(null);
  const [isLoading, setIsLoading] = React.useState(true);
  const [authError, setAuthError] = React.useState<string | null>(null);

  const clearAuthError = React.useCallback(() => setAuthError(null), []);

  React.useEffect(() => {
    const checkAuth = async () => {
      try {
        const userData = await authApi.me();
        setUser(userData);
        apiClient.setAuthenticated(true);
        setAuthError(null);
      } catch (err) {
        setUser(null);
        apiClient.setAuthenticated(false);
        if (err instanceof Error && !err.message.includes("401")) {
          setAuthError("Failed to verify authentication");
        }
      }
      setIsLoading(false);
    };
    checkAuth();
  }, []);

  const logout = React.useCallback(async () => {
    try {
      await authApi.logout();
    } finally {
      setUser(null);
      apiClient.setAuthenticated(false);
      queryClient.clear();
    }
  }, [queryClient]);

  const login = React.useCallback(async (data: LoginRequest) => {
    const response = await authApi.login(data);
    setUser(response.user);
    apiClient.setAuthenticated(true);
  }, []);

  const register = React.useCallback(async (data: RegisterRequest) => {
    const response = await authApi.register(data);
    setUser(response.user);
    apiClient.setAuthenticated(true);
  }, []);

  const value = React.useMemo<AuthContextValue>(
    () => ({
      user,
      isLoading,
      isAuthenticated: !!user,
      authError,
      login,
      register,
      logout,
      clearAuthError,
    }),
    [user, isLoading, authError, login, register, logout, clearAuthError],
  );

  return <AuthContext.Provider value={value}>{children}</AuthContext.Provider>;
}
