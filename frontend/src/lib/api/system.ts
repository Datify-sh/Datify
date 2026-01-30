import type {
  PostgresVersionsResponse,
  RedisVersionsResponse,
  SystemInfoResponse,
  ValkeyVersionsResponse,
} from "./types";

import { fetchWithAuth } from "./client";

export const systemApi = {
  getInfo: async (): Promise<SystemInfoResponse> => {
    const response = await fetchWithAuth("/api/v1/system");
    if (!response.ok) {
      throw new Error("Failed to fetch system info");
    }
    return response.json();
  },

  getPostgresVersions: async (): Promise<PostgresVersionsResponse> => {
    const response = await fetchWithAuth("/api/v1/system/postgres-versions");
    if (!response.ok) {
      throw new Error("Failed to fetch PostgreSQL versions");
    }
    return response.json();
  },

  getValkeyVersions: async (): Promise<ValkeyVersionsResponse> => {
    const response = await fetchWithAuth("/api/v1/system/valkey-versions");
    if (!response.ok) {
      throw new Error("Failed to fetch Valkey versions");
    }
    return response.json();
  },

  getRedisVersions: async (): Promise<RedisVersionsResponse> => {
    const response = await fetchWithAuth("/api/v1/system/redis-versions");
    if (!response.ok) {
      throw new Error("Failed to fetch Redis versions");
    }
    return response.json();
  },
};
