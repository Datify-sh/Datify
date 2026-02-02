import { apiClient } from "./client";
import type { UpdateUserRequest, UserResponse } from "./types";

export const adminApi = {
  listUsers: (params?: { limit?: number; offset?: number }) => {
    const searchParams = new URLSearchParams();
    if (params?.limit !== undefined) searchParams.set("limit", params.limit.toString());
    if (params?.offset !== undefined) searchParams.set("offset", params.offset.toString());
    const query = searchParams.toString();
    return apiClient.get<UserResponse[]>(`/admin/users${query ? `?${query}` : ""}`);
  },
  updateUser: (id: string, data: UpdateUserRequest) =>
    apiClient.put<UserResponse>(`/admin/users/${id}`, data),
  deleteUser: (id: string) => apiClient.delete<void>(`/admin/users/${id}`),
};
