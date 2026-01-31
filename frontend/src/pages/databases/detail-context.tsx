import type * as React from "react";
import { useOutletContext } from "react-router-dom";

import type { DatabaseResponse, ProjectResponse } from "@/lib/api/types";

export type TerminalType = "shell" | "psql" | "valkey-cli" | "redis-cli";

export type DatabaseDetailContext = {
  database: DatabaseResponse;
  project?: ProjectResponse;
  parentDatabase?: DatabaseResponse;
  id: string;
  isRunning: boolean;
  isTransitioning: boolean;
  isKeyValue: boolean;
  isValkey: boolean;
  isRedis: boolean;
  hasBranches: boolean;
  isChildBranch: boolean;
  terminalType: TerminalType;
  setTerminalType: React.Dispatch<React.SetStateAction<TerminalType>>;
  openCreateBranch: () => void;
  startDatabase: () => void;
  startDatabasePending: boolean;
};

export function useDatabaseDetailContext() {
  return useOutletContext<DatabaseDetailContext>();
}
