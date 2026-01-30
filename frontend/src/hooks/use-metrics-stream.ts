import { databasesApi } from "@/lib/api";
import type { DatabaseMetrics, MetricsStreamMessage } from "@/lib/api/types";
import * as React from "react";

interface UseMetricsStreamOptions {
  enabled?: boolean;
}

interface UseMetricsStreamResult {
  metrics: DatabaseMetrics | null;
  isConnected: boolean;
  isConnecting: boolean;
  error: string | null;
}

const MAX_RECONNECT_ATTEMPTS = 5;
const BASE_RECONNECT_DELAY = 1000;

export function useMetricsStream(
  databaseId: string,
  options: UseMetricsStreamOptions = {},
): UseMetricsStreamResult {
  const { enabled = true } = options;

  const [metrics, setMetrics] = React.useState<DatabaseMetrics | null>(null);
  const [isConnected, setIsConnected] = React.useState(false);
  const [isConnecting, setIsConnecting] = React.useState(false);
  const [error, setError] = React.useState<string | null>(null);

  const wsRef = React.useRef<WebSocket | null>(null);
  const reconnectTimeoutRef = React.useRef<ReturnType<typeof setTimeout> | null>(null);
  const reconnectAttemptsRef = React.useRef(0);

  React.useEffect(() => {
    if (!enabled || !databaseId) {
      return;
    }

    reconnectAttemptsRef.current = 0;

    const connect = () => {
      setIsConnecting(true);
      setError(null);

      const wsUrl = databasesApi.getMetricsStreamUrl(databaseId);
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
          const data = JSON.parse(event.data) as MetricsStreamMessage;

          if (data.type === "metrics") {
            setMetrics(data.metrics);
          } else if (data.type === "error") {
            setError(data.message);
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
  }, [databaseId, enabled]);

  return {
    metrics,
    isConnected,
    isConnecting,
    error,
  };
}
