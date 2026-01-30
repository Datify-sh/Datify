import { apiClient } from "./client";
import type {
  CreateProjectRequest,
  PaginatedResponse,
  ProjectResponse,
  ProjectWithStats,
  UpdateProjectRequest,
} from "./types";

export const projectsApi = {
  list: (params?: { limit?: number; offset?: number }) => {
    const searchParams = new URLSearchParams();
    if (params?.limit) searchParams.set("limit", params.limit.toString());
    if (params?.offset) searchParams.set("offset", params.offset.toString());
    const query = searchParams.toString();
    return apiClient.get<PaginatedResponse<ProjectWithStats>>(
      `/projects${query ? `?${query}` : ""}`,
    );
  },

  get: (id: string) => apiClient.get<ProjectWithStats>(`/projects/${id}`),

  create: (data: CreateProjectRequest) => apiClient.post<ProjectResponse>("/projects", data),

  update: (id: string, data: UpdateProjectRequest) =>
    apiClient.put<ProjectResponse>(`/projects/${id}`, data),

  delete: (id: string) => apiClient.delete<void>(`/projects/${id}`),
};
