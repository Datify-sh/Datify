import { databasesApi } from "@/lib/api";
import type { LogEntryResponse } from "@/lib/api/types";
import * as React from "react";

interface UseLogStreamOptions {
  tail?: number;
  enabled?: boolean;
}

interface UseLogStreamResult {
  entries: LogEntryResponse[];
  isConnected: boolean;
  isConnecting: boolean;
  error: string | null;
  clear: () => void;
}

const MAX_RECONNECT_ATTEMPTS = 5;
const BASE_RECONNECT_DELAY = 1000;
const MAX_LOG_ENTRIES = 500;

export function useLogStream(
  databaseId: string,
  options: UseLogStreamOptions = {},
): UseLogStreamResult {
  const { tail = 200, enabled = true } = options;

  const [entries, setEntries] = React.useState<LogEntryResponse[]>([]);
  const [isConnected, setIsConnected] = React.useState(false);
  const [isConnecting, setIsConnecting] = React.useState(false);
  const [error, setError] = React.useState<string | null>(null);

  const wsRef = React.useRef<WebSocket | null>(null);
  const reconnectTimeoutRef = React.useRef<ReturnType<typeof setTimeout> | null>(null);
  const reconnectAttemptsRef = React.useRef(0);
  const isMountedRef = React.useRef(true);

  const clear = React.useCallback(() => {
    setEntries([]);
  }, []);

  React.useEffect(() => {
    if (!enabled || !databaseId) {
      return;
    }

    isMountedRef.current = true;
    reconnectAttemptsRef.current = 0;

    const connect = () => {
      if (!isMountedRef.current) return;
      setIsConnecting(true);
      setError(null);

      const wsUrl = databasesApi.getLogsStreamUrl(databaseId, tail);
      const ws = new WebSocket(wsUrl);
      wsRef.current = ws;

      ws.onopen = () => {
        if (!isMountedRef.current) return;
        setIsConnected(true);
        setIsConnecting(false);
        setError(null);
        reconnectAttemptsRef.current = 0;
      };

      ws.onmessage = (event) => {
        if (!isMountedRef.current) return;
        try {
          const data = JSON.parse(event.data);

          if (data.type === "initial" && Array.isArray(data.entries)) {
            const entries = data.entries.slice(-MAX_LOG_ENTRIES);
            setEntries(entries);
          } else if (data.type === "log" && data.entry) {
            setEntries((prev) => {
              const newEntries = [...prev, data.entry];
              return newEntries.length > MAX_LOG_ENTRIES
                ? newEntries.slice(-MAX_LOG_ENTRIES)
                : newEntries;
            });
          } else if (data.log_type && data.message !== undefined) {
            setEntries((prev) => {
              const newEntries = [...prev, data as LogEntryResponse];
              return newEntries.length > MAX_LOG_ENTRIES
                ? newEntries.slice(-MAX_LOG_ENTRIES)
                : newEntries;
            });
          }
        } catch {
          /* ignore */
        }
      };

      ws.onerror = () => {
        if (!isMountedRef.current) return;
        setError("Connection error");
        setIsConnected(false);
        setIsConnecting(false);
      };

      ws.onclose = () => {
        if (!isMountedRef.current) return;
        setIsConnected(false);
        setIsConnecting(false);

        if (enabled && reconnectAttemptsRef.current < MAX_RECONNECT_ATTEMPTS) {
          const delay = BASE_RECONNECT_DELAY * 2 ** reconnectAttemptsRef.current;
          reconnectAttemptsRef.current++;
          reconnectTimeoutRef.current = setTimeout(connect, delay);
        } else if (reconnectAttemptsRef.current >= MAX_RECONNECT_ATTEMPTS) {
          setError("Connection failed after multiple attempts");
        }
      };
    };

    connect();

    return () => {
      isMountedRef.current = false;
      if (reconnectTimeoutRef.current) {
        clearTimeout(reconnectTimeoutRef.current);
      }
      if (wsRef.current) {
        wsRef.current.close();
        wsRef.current = null;
      }
    };
  }, [databaseId, tail, enabled]);

  return {
    entries,
    isConnected,
    isConnecting,
    error,
    clear,
  };
}
