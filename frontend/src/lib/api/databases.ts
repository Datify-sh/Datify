import { apiClient } from "./client";
import type {
  BranchResponse,
  ChangePasswordRequest,
  CreateBranchRequest,
  CreateDatabaseRequest,
  DatabaseResponse,
  ExecuteQueryRequest,
  LogsResponse,
  MetricsHistory,
  MetricsResponse,
  PaginatedResponse,
  QueryLogsResponse,
  QueryResult,
  SchemaInfo,
  TablePreview,
  TimeRange,
  UpdateDatabaseRequest,
} from "./types";

export const databasesApi = {
  list: (projectId: string, params?: { limit?: number; offset?: number }) => {
    const searchParams = new URLSearchParams();
    if (params?.limit) searchParams.set("limit", params.limit.toString());
    if (params?.offset) searchParams.set("offset", params.offset.toString());
    const query = searchParams.toString();
    return apiClient.get<PaginatedResponse<DatabaseResponse>>(
      `/projects/${projectId}/databases${query ? `?${query}` : ""}`,
    );
  },

  get: (id: string) => apiClient.get<DatabaseResponse>(`/databases/${id}`),

  create: (projectId: string, data: CreateDatabaseRequest) =>
    apiClient.post<DatabaseResponse>(`/projects/${projectId}/databases`, data),

  update: (id: string, data: UpdateDatabaseRequest) =>
    apiClient.put<DatabaseResponse>(`/databases/${id}`, data),

  delete: (id: string) => apiClient.delete<void>(`/databases/${id}`),

  start: (id: string) => apiClient.post<DatabaseResponse>(`/databases/${id}/start`),

  stop: (id: string) => apiClient.post<DatabaseResponse>(`/databases/${id}/stop`),

  changePassword: (id: string, data: ChangePasswordRequest) =>
    apiClient.post<DatabaseResponse>(`/databases/${id}/change-password`, data),

  logs: (id: string, params?: { tail?: number; since?: number; timestamps?: boolean }) => {
    const searchParams = new URLSearchParams();
    if (params?.tail) searchParams.set("tail", params.tail.toString());
    if (params?.since) searchParams.set("since", params.since.toString());
    if (params?.timestamps) searchParams.set("timestamps", params.timestamps.toString());
    const query = searchParams.toString();
    return apiClient.get<LogsResponse>(`/databases/${id}/logs${query ? `?${query}` : ""}`);
  },

  getLogsStreamUrl: (id: string, tail?: number) => {
    const baseUrl = window.location.origin.replace("http", "ws");
    const params = tail ? `?tail=${tail}` : "";
    return `${baseUrl}/api/v1/databases/${id}/logs/stream${params}`;
  },

  getTerminalUrl: (id: string) => {
    const baseUrl = window.location.origin.replace("http", "ws");
    return `${baseUrl}/api/v1/databases/${id}/terminal`;
  },

  getPsqlUrl: (id: string) => {
    const baseUrl = window.location.origin.replace("http", "ws");
    return `${baseUrl}/api/v1/databases/${id}/psql`;
  },

  getValkeyCliUrl: (id: string) => {
    const baseUrl = window.location.origin.replace("http", "ws");
    return `${baseUrl}/api/v1/databases/${id}/valkey-cli`;
  },

  metrics: (id: string) => apiClient.get<MetricsResponse>(`/databases/${id}/metrics`),

  metricsHistory: (id: string, range?: TimeRange) => {
    const params = range ? `?range=${range}` : "";
    return apiClient.get<MetricsHistory>(`/databases/${id}/metrics/history${params}`);
  },

  queryLogs: (id: string, params?: { limit?: number; sort_by?: string }) => {
    const searchParams = new URLSearchParams();
    if (params?.limit) searchParams.set("limit", params.limit.toString());
    if (params?.sort_by) searchParams.set("sort_by", params.sort_by);
    const query = searchParams.toString();
    return apiClient.get<QueryLogsResponse>(`/databases/${id}/queries${query ? `?${query}` : ""}`);
  },

  getMetricsStreamUrl: (id: string) => {
    const baseUrl = window.location.origin.replace("http", "ws");
    return `${baseUrl}/api/v1/databases/${id}/metrics/stream`;
  },

  listBranches: (id: string) => apiClient.get<BranchResponse[]>(`/databases/${id}/branches`),

  createBranch: (id: string, data: CreateBranchRequest) =>
    apiClient.post<DatabaseResponse>(`/databases/${id}/branches`, data),

  syncFromParent: (id: string) =>
    apiClient.post<DatabaseResponse>(`/databases/${id}/sync-from-parent`),

  // SQL Editor endpoints
  getSchema: (id: string) => apiClient.get<SchemaInfo>(`/databases/${id}/schema`),

  executeQuery: (id: string, request: ExecuteQueryRequest) =>
    apiClient.post<QueryResult>(`/databases/${id}/query`, request),

  previewTable: (
    id: string,
    schema: string,
    table: string,
    params?: { limit?: number; offset?: number },
  ) => {
    const searchParams = new URLSearchParams();
    if (params?.limit) searchParams.set("limit", params.limit.toString());
    if (params?.offset) searchParams.set("offset", params.offset.toString());
    const query = searchParams.toString();
    return apiClient.get<TablePreview>(
      `/databases/${id}/tables/${encodeURIComponent(schema)}/${encodeURIComponent(table)}/preview${query ? `?${query}` : ""}`,
    );
  },
};
