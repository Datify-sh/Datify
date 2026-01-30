import type { PostgresVersionsResponse, SystemInfoResponse, ValkeyVersionsResponse } from "./types";

export const systemApi = {
  getInfo: async (): Promise<SystemInfoResponse> => {
    const response = await fetch("/system");
    if (!response.ok) {
      throw new Error("Failed to fetch system info");
    }
    return response.json();
  },

  getPostgresVersions: async (): Promise<PostgresVersionsResponse> => {
    const response = await fetch("/system/postgres-versions");
    if (!response.ok) {
      throw new Error("Failed to fetch PostgreSQL versions");
    }
    return response.json();
  },

  getValkeyVersions: async (): Promise<ValkeyVersionsResponse> => {
    const response = await fetch("/system/valkey-versions");
    if (!response.ok) {
      throw new Error("Failed to fetch Valkey versions");
    }
    return response.json();
  },
};
