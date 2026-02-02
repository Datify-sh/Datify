export interface SqlResult {
  type: "sql";
  columns: { name: string; type: string }[];
  rows: unknown[][];
  rowCount: number;
  truncated: boolean;
  error: string | null;
}

export interface KeyValueResult {
  command: string;
  result: unknown;
  error: string | null;
}

export interface KvResult {
  type: "kv";
  results: KeyValueResult[];
  error: string | null;
}

export type QueryResult = SqlResult | KvResult;

export interface QueryTab {
  id: string;
  title: string;
  content: string;
  type: "sql" | "kv";
  isExecuting: boolean;
  result?: QueryResult;
  executionTime?: number;
}
