import { apiClient } from "./client";
import type { AuditLogFilter, PaginatedAuditLogsResponse } from "./types";

export const auditLogsApi = {
  list: (params?: { page?: number; page_size?: number } & AuditLogFilter) => {
    const searchParams = new URLSearchParams();
    if (params?.page) searchParams.set("page", params.page.toString());
    if (params?.page_size) searchParams.set("page_size", params.page_size.toString());
    if (params?.action) searchParams.set("action", params.action);
    if (params?.entity_type) searchParams.set("entity_type", params.entity_type);
    if (params?.status) searchParams.set("status", params.status);
    if (params?.start_date) searchParams.set("start_date", params.start_date);
    if (params?.end_date) searchParams.set("end_date", params.end_date);
    const query = searchParams.toString();
    return apiClient.get<PaginatedAuditLogsResponse>(`/audit-logs${query ? `?${query}` : ""}`);
  },
};
