import { apiClient } from "./client";
import type {
  LoginRequest,
  LoginResponse,
  RegisterRequest,
  RefreshRequest,
  AuthTokens,
  UserResponse,
} from "./types";

export const authApi = {
  login: (data: LoginRequest) => apiClient.post<LoginResponse>("/auth/login", data),

  register: (data: RegisterRequest) => apiClient.post<LoginResponse>("/auth/register", data),

  refresh: (data: RefreshRequest) => apiClient.post<AuthTokens>("/auth/refresh", data),

  me: () => apiClient.get<UserResponse>("/me"),

  logout: () => apiClient.post<void>("/auth/logout", {}),

  logoutAll: () => apiClient.post<void>("/auth/logout-all", {}),
};
