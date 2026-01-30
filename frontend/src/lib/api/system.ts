import type {
  PostgresVersionsResponse,
  RedisVersionsResponse,
  SystemInfoResponse,
  ValkeyVersionsResponse,
} from "./types";

import { apiClient } from "./client";

export const systemApi = {
  getInfo: (): Promise<SystemInfoResponse> => {
    return apiClient.get<SystemInfoResponse>("/system");
  },

  getPostgresVersions: (): Promise<PostgresVersionsResponse> => {
    return apiClient.get<PostgresVersionsResponse>("/system/postgres-versions");
  },

  getValkeyVersions: (): Promise<ValkeyVersionsResponse> => {
    return apiClient.get<ValkeyVersionsResponse>("/system/valkey-versions");
  },

  getRedisVersions: (): Promise<RedisVersionsResponse> => {
    return apiClient.get<RedisVersionsResponse>("/system/redis-versions");
  },
};
