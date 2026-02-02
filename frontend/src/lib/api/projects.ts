import { apiClient } from "./client";
import type {
  CreateProjectRequest,
  PaginatedResponse,
  ProjectResponse,
  ProjectWithStats,
  UpdateProjectRequest,
} from "./types";

export const projectsApi = {
  list: (params?: { page?: number; pageSize?: number }) => {
    const searchParams = new URLSearchParams();
    if (params?.page !== undefined) searchParams.set("page", params.page.toString());
    if (params?.pageSize !== undefined) searchParams.set("page_size", params.pageSize.toString());
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
