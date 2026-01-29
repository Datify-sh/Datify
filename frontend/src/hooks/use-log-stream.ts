import * as React from "react";
import type { LogEntryResponse } from "@/lib/api/types";
import { databasesApi } from "@/lib/api";

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

  const clear = React.useCallback(() => {
    setEntries([]);
  }, []);

  React.useEffect(() => {
    if (!enabled || !databaseId) {
      return;
    }

    reconnectAttemptsRef.current = 0;

    const connect = () => {
      setIsConnecting(true);
      setError(null);

      const wsUrl = databasesApi.getLogsStreamUrl(databaseId, tail);
      const ws = new WebSocket(wsUrl);
      wsRef.current = ws;

      ws.onopen = () => {
        setIsConnected(true);
        setIsConnecting(false);
        setError(null);
        reconnectAttemptsRef.current = 0;
      };

      ws.onmessage = (event) => {
        try {
          const data = JSON.parse(event.data);

          if (data.type === "initial" && Array.isArray(data.entries)) {
            setEntries(data.entries);
          } else if (data.type === "log" && data.entry) {
            setEntries((prev) => [...prev, data.entry]);
          } else if (data.log_type && data.message !== undefined) {
            setEntries((prev) => [...prev, data as LogEntryResponse]);
          }
        } catch {
          /* ignore */
        }
      };

      ws.onerror = () => {
        setError("Connection error");
        setIsConnected(false);
        setIsConnecting(false);
      };

      ws.onclose = () => {
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
