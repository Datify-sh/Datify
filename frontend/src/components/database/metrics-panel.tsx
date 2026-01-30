import { BarChartIcon } from "@hugeicons/core-free-icons";
import { HugeiconsIcon } from "@hugeicons/react";
import { useQuery } from "@tanstack/react-query";
import { format } from "date-fns";
import * as React from "react";
import { Area, AreaChart, CartesianGrid, Line, LineChart, XAxis, YAxis } from "recharts";

import { Badge } from "@/components/ui/badge";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { ChartContainer, ChartTooltip, ChartTooltipContent } from "@/components/ui/chart";
import { Skeleton } from "@/components/ui/skeleton";
import { Tabs, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { useMetricsStream } from "@/hooks/use-metrics-stream";
import { databasesApi } from "@/lib/api";
import type {
  DatabaseMetrics,
  DatabaseType,
  KeyValueMetrics,
  MetricsHistoryPoint,
  TimeRange,
  UnifiedMetrics,
} from "@/lib/api/types";

import { KeyValueMetricsPanel } from "./keyvalue-metrics-panel";
import { PostgresMetricsPanel } from "./postgres-metrics-panel";

interface MetricsPanelProps {
  databaseId: string;
  databaseType: DatabaseType;
  isRunning: boolean;
}

const TIME_RANGES: { value: TimeRange; label: string }[] = [
  { value: "realtime", label: "Realtime" },
  { value: "last_5_min", label: "5 min" },
  { value: "last_15_min", label: "15 min" },
  { value: "last_30_min", label: "30 min" },
  { value: "last_1_hour", label: "1 hour" },
  { value: "last_24_hours", label: "24 hours" },
];

function formatNumber(num: number): string {
  if (num >= 1000000) return `${(num / 1000000).toFixed(1)}M`;
  if (num >= 1000) return `${(num / 1000).toFixed(1)}K`;
  return num.toFixed(0);
}

const chartConfig = {
  cpu: { label: "CPU %", color: "#f97316" },
  memory: { label: "Memory %", color: "#8b5cf6" },
  queries: { label: "Queries/sec", color: "#3b82f6" },
  latency: { label: "Latency (ms)", color: "#ef4444" },
  rowsRead: { label: "Rows Read", color: "#22c55e" },
  rowsWritten: { label: "Rows Written", color: "#eab308" },
  connections: { label: "Connections", color: "#06b6d4" },
};

function isPostgresMetrics(
  metrics: UnifiedMetrics,
): metrics is { database_type: "postgres" } & DatabaseMetrics {
  return metrics.database_type === "postgres";
}

function isKeyValueMetrics(
  metrics: UnifiedMetrics,
): metrics is ({ database_type: "redis" } | { database_type: "valkey" }) & KeyValueMetrics {
  return metrics.database_type === "redis" || metrics.database_type === "valkey";
}

const MAX_REALTIME_POINTS = 60;

export const MetricsPanel = React.memo(function MetricsPanel({
  databaseId,
  databaseType,
  isRunning,
}: MetricsPanelProps) {
  const [timeRange, setTimeRange] = React.useState<TimeRange>("last_15_min");
  const isRealtime = timeRange === "realtime";
  const [realtimeBuffer, setRealtimeBuffer] = React.useState<
    Array<{
      time: string;
      timestamp: string;
      queries: number;
      latency: number;
      cpu: number;
      memory: number;
      rowsRead: number;
      rowsWritten: number;
      connections: number;
    }>
  >([]);

  const handleTimeRangeChange = React.useCallback((v: string) => {
    setTimeRange(v as TimeRange);
    if (v === "realtime") {
      setRealtimeBuffer([]);
    }
  }, []);

  const { metrics: realtimeMetrics, isConnected } = useMetricsStream(databaseId, {
    enabled: isRunning && isRealtime,
  });

  React.useEffect(() => {
    if (realtimeMetrics && isRealtime) {
      const now = new Date();
      const point = {
        time: format(now, "HH:mm:ss"),
        timestamp: now.toISOString(),
        queries:
          realtimeMetrics.database_type === "postgres"
            ? (realtimeMetrics.queries?.queries_per_sec ?? 0)
            : 0,
        latency:
          realtimeMetrics.database_type === "postgres"
            ? (realtimeMetrics.queries?.avg_latency_ms ?? 0)
            : 0,
        cpu:
          realtimeMetrics.database_type === "postgres"
            ? (realtimeMetrics.resources?.cpu_percent ?? 0)
            : (realtimeMetrics.resources?.cpu_percent ?? 0),
        memory:
          realtimeMetrics.database_type === "postgres"
            ? (realtimeMetrics.resources?.memory_percent ?? 0)
            : (realtimeMetrics.resources?.memory_percent ?? 0),
        rowsRead:
          realtimeMetrics.database_type === "postgres" ? (realtimeMetrics.rows?.rows_read ?? 0) : 0,
        rowsWritten:
          realtimeMetrics.database_type === "postgres"
            ? (realtimeMetrics.rows?.rows_written ?? 0)
            : 0,
        connections:
          realtimeMetrics.database_type === "postgres"
            ? (realtimeMetrics.connections?.active_connections ?? 0)
            : (realtimeMetrics.clients?.connected_clients ?? 0),
      };
      setRealtimeBuffer((prev) => {
        const newBuffer = [...prev, point];
        if (newBuffer.length > MAX_REALTIME_POINTS) {
          return newBuffer.slice(-MAX_REALTIME_POINTS);
        }
        return newBuffer;
      });
    }
  }, [realtimeMetrics, isRealtime]);

  React.useEffect(() => {
    if (!isRealtime) {
      setRealtimeBuffer([]);
    }
  }, [isRealtime]);

  const { data: historyData, isLoading: historyLoading } = useQuery({
    queryKey: ["metricsHistory", databaseId, timeRange],
    queryFn: () => databasesApi.metricsHistory(databaseId, timeRange),
    enabled: isRunning && !isRealtime,
    refetchInterval: timeRange === "last_5_min" ? 10000 : 30000,
    staleTime: timeRange === "last_5_min" ? 5000 : 15000,
  });

  const { data: currentMetrics, isLoading: currentLoading } = useQuery({
    queryKey: ["metrics", databaseId],
    queryFn: () => databasesApi.metrics(databaseId),
    enabled: isRunning && !isRealtime,
    refetchInterval: 10000,
    staleTime: 5000,
  });

  const metrics: UnifiedMetrics | null = isRealtime
    ? realtimeMetrics
    : (currentMetrics?.metrics ?? null);

  const chartData = React.useMemo(() => {
    if (isRealtime) {
      return realtimeBuffer;
    }
    const historyPoints: MetricsHistoryPoint[] = historyData?.points ?? [];
    return historyPoints.map((point) => ({
      time: format(new Date(point.timestamp), "HH:mm:ss"),
      timestamp: point.timestamp,
      queries: point.queries_per_sec,
      latency: point.avg_latency_ms,
      cpu: point.cpu_percent,
      memory: point.memory_percent,
      rowsRead: point.rows_read,
      rowsWritten: point.rows_written,
      connections: point.active_connections,
    }));
  }, [historyData?.points, isRealtime, realtimeBuffer]);

  if (!isRunning) {
    return (
      <Card>
        <CardContent className="flex flex-col items-center justify-center py-12">
          <HugeiconsIcon
            icon={BarChartIcon}
            className="size-12 text-muted-foreground"
            strokeWidth={1.5}
          />
          <p className="mt-4 text-muted-foreground">Start the database to view metrics</p>
        </CardContent>
      </Card>
    );
  }

  const isLoading = isRealtime ? !metrics && !isConnected : currentLoading || historyLoading;
  const isPostgres = databaseType === "postgres";

  return (
    <div className="space-y-6">
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-2">
          <span className="text-sm text-muted-foreground">Time Range:</span>
          <Tabs value={timeRange} onValueChange={handleTimeRangeChange}>
            <TabsList variant="line">
              {TIME_RANGES.map(({ value, label }) => (
                <TabsTrigger key={value} value={value}>
                  {label}
                </TabsTrigger>
              ))}
            </TabsList>
          </Tabs>
        </div>
        {isRealtime && (
          <Badge variant={isConnected ? "default" : "secondary"}>
            {isConnected ? "Live" : "Connecting..."}
          </Badge>
        )}
      </div>

      {isLoading ? (
        <div className="grid gap-4 sm:grid-cols-2 lg:grid-cols-4">
          {[...Array(8)].map((_, i) => (
            <Skeleton key={i} className="h-24" />
          ))}
        </div>
      ) : metrics ? (
        <>
          {isPostgresMetrics(metrics) ? (
            <PostgresMetricsPanel metrics={metrics} />
          ) : isKeyValueMetrics(metrics) ? (
            <KeyValueMetricsPanel metrics={metrics} databaseType={metrics.database_type} />
          ) : null}
        </>
      ) : null}

      {chartData.length > 0 && (
        <div className="grid gap-6 lg:grid-cols-2">
          <Card>
            <CardHeader className="pb-2">
              <CardTitle className="text-sm font-medium">CPU Usage</CardTitle>
            </CardHeader>
            <CardContent>
              <ChartContainer config={chartConfig} className="h-[200px] w-full">
                <AreaChart data={chartData} margin={{ top: 10, right: 10, left: 0, bottom: 0 }}>
                  <defs>
                    <linearGradient id="cpuGradient" x1="0" y1="0" x2="0" y2="1">
                      <stop offset="5%" stopColor="#f97316" stopOpacity={0.3} />
                      <stop offset="95%" stopColor="#f97316" stopOpacity={0} />
                    </linearGradient>
                  </defs>
                  <CartesianGrid strokeDasharray="3 3" className="stroke-muted" />
                  <XAxis dataKey="time" tick={{ fontSize: 10 }} tickLine={false} axisLine={false} />
                  <YAxis
                    tick={{ fontSize: 10 }}
                    tickLine={false}
                    axisLine={false}
                    tickFormatter={(v) => `${v}%`}
                    domain={[0, 100]}
                    width={40}
                  />
                  <ChartTooltip content={<ChartTooltipContent />} />
                  <Area
                    type="monotone"
                    dataKey="cpu"
                    stroke="#f97316"
                    fill="url(#cpuGradient)"
                    strokeWidth={2}
                  />
                </AreaChart>
              </ChartContainer>
            </CardContent>
          </Card>

          <Card>
            <CardHeader className="pb-2">
              <CardTitle className="text-sm font-medium">Memory Usage</CardTitle>
            </CardHeader>
            <CardContent>
              <ChartContainer config={chartConfig} className="h-[200px] w-full">
                <AreaChart data={chartData} margin={{ top: 10, right: 10, left: 0, bottom: 0 }}>
                  <defs>
                    <linearGradient id="memoryGradient" x1="0" y1="0" x2="0" y2="1">
                      <stop offset="5%" stopColor="#8b5cf6" stopOpacity={0.3} />
                      <stop offset="95%" stopColor="#8b5cf6" stopOpacity={0} />
                    </linearGradient>
                  </defs>
                  <CartesianGrid strokeDasharray="3 3" className="stroke-muted" />
                  <XAxis dataKey="time" tick={{ fontSize: 10 }} tickLine={false} axisLine={false} />
                  <YAxis
                    tick={{ fontSize: 10 }}
                    tickLine={false}
                    axisLine={false}
                    tickFormatter={(v) => `${v}%`}
                    domain={[0, 100]}
                    width={40}
                  />
                  <ChartTooltip content={<ChartTooltipContent />} />
                  <Area
                    type="monotone"
                    dataKey="memory"
                    stroke="#8b5cf6"
                    fill="url(#memoryGradient)"
                    strokeWidth={2}
                  />
                </AreaChart>
              </ChartContainer>
            </CardContent>
          </Card>

          <Card>
            <CardHeader className="pb-2">
              <CardTitle className="text-sm font-medium">
                {isPostgres ? "Active Connections" : "Connected Clients"}
              </CardTitle>
            </CardHeader>
            <CardContent>
              <ChartContainer config={chartConfig} className="h-[200px] w-full">
                <AreaChart data={chartData} margin={{ top: 10, right: 10, left: 0, bottom: 0 }}>
                  <defs>
                    <linearGradient id="connectionsGradient" x1="0" y1="0" x2="0" y2="1">
                      <stop offset="5%" stopColor="#06b6d4" stopOpacity={0.3} />
                      <stop offset="95%" stopColor="#06b6d4" stopOpacity={0} />
                    </linearGradient>
                  </defs>
                  <CartesianGrid strokeDasharray="3 3" className="stroke-muted" />
                  <XAxis dataKey="time" tick={{ fontSize: 10 }} tickLine={false} axisLine={false} />
                  <YAxis tick={{ fontSize: 10 }} tickLine={false} axisLine={false} width={40} />
                  <ChartTooltip content={<ChartTooltipContent />} />
                  <Area
                    type="monotone"
                    dataKey="connections"
                    stroke="#06b6d4"
                    fill="url(#connectionsGradient)"
                    strokeWidth={2}
                  />
                </AreaChart>
              </ChartContainer>
            </CardContent>
          </Card>

          {isPostgres && (
            <>
              <Card>
                <CardHeader className="pb-2">
                  <CardTitle className="text-sm font-medium">Query Latency</CardTitle>
                </CardHeader>
                <CardContent>
                  <ChartContainer config={chartConfig} className="h-[200px] w-full">
                    <LineChart data={chartData} margin={{ top: 10, right: 10, left: 0, bottom: 0 }}>
                      <CartesianGrid strokeDasharray="3 3" className="stroke-muted" />
                      <XAxis
                        dataKey="time"
                        tick={{ fontSize: 10 }}
                        tickLine={false}
                        axisLine={false}
                      />
                      <YAxis
                        tick={{ fontSize: 10 }}
                        tickLine={false}
                        axisLine={false}
                        tickFormatter={(v) => `${v}ms`}
                        width={50}
                      />
                      <ChartTooltip content={<ChartTooltipContent />} />
                      <Line
                        type="monotone"
                        dataKey="latency"
                        stroke="#ef4444"
                        strokeWidth={2}
                        dot={false}
                      />
                    </LineChart>
                  </ChartContainer>
                </CardContent>
              </Card>

              <Card>
                <CardHeader className="pb-2">
                  <CardTitle className="text-sm font-medium">Queries per Second</CardTitle>
                </CardHeader>
                <CardContent>
                  <ChartContainer config={chartConfig} className="h-[200px] w-full">
                    <AreaChart data={chartData} margin={{ top: 10, right: 10, left: 0, bottom: 0 }}>
                      <defs>
                        <linearGradient id="queriesGradient" x1="0" y1="0" x2="0" y2="1">
                          <stop offset="5%" stopColor="#3b82f6" stopOpacity={0.3} />
                          <stop offset="95%" stopColor="#3b82f6" stopOpacity={0} />
                        </linearGradient>
                      </defs>
                      <CartesianGrid strokeDasharray="3 3" className="stroke-muted" />
                      <XAxis
                        dataKey="time"
                        tick={{ fontSize: 10 }}
                        tickLine={false}
                        axisLine={false}
                      />
                      <YAxis tick={{ fontSize: 10 }} tickLine={false} axisLine={false} width={40} />
                      <ChartTooltip content={<ChartTooltipContent />} />
                      <Area
                        type="monotone"
                        dataKey="queries"
                        stroke="#3b82f6"
                        fill="url(#queriesGradient)"
                        strokeWidth={2}
                      />
                    </AreaChart>
                  </ChartContainer>
                </CardContent>
              </Card>

              <Card className="lg:col-span-2">
                <CardHeader className="pb-2">
                  <CardTitle className="text-sm font-medium">Rows Read / Written</CardTitle>
                </CardHeader>
                <CardContent>
                  <ChartContainer config={chartConfig} className="h-[200px] w-full">
                    <LineChart data={chartData} margin={{ top: 10, right: 10, left: 0, bottom: 0 }}>
                      <CartesianGrid strokeDasharray="3 3" className="stroke-muted" />
                      <XAxis
                        dataKey="time"
                        tick={{ fontSize: 10 }}
                        tickLine={false}
                        axisLine={false}
                      />
                      <YAxis
                        tick={{ fontSize: 10 }}
                        tickLine={false}
                        axisLine={false}
                        tickFormatter={(v) => formatNumber(v)}
                        width={50}
                      />
                      <ChartTooltip content={<ChartTooltipContent />} />
                      <Line
                        type="monotone"
                        dataKey="rowsRead"
                        stroke="#22c55e"
                        strokeWidth={2}
                        dot={false}
                        name="Rows Read"
                      />
                      <Line
                        type="monotone"
                        dataKey="rowsWritten"
                        stroke="#eab308"
                        strokeWidth={2}
                        dot={false}
                        name="Rows Written"
                      />
                    </LineChart>
                  </ChartContainer>
                </CardContent>
              </Card>
            </>
          )}
        </div>
      )}

      {chartData.length === 0 && !historyLoading && !isRealtime && (
        <Card>
          <CardContent className="flex flex-col items-center justify-center py-8">
            <p className="text-sm text-muted-foreground">
              No historical data available for this time range yet.
            </p>
            <p className="text-xs text-muted-foreground mt-1">
              Metrics are collected every 15 seconds.
            </p>
          </CardContent>
        </Card>
      )}
      {chartData.length === 0 && isRealtime && isConnected && (
        <Card>
          <CardContent className="flex flex-col items-center justify-center py-8">
            <p className="text-sm text-muted-foreground">Collecting realtime data...</p>
            <p className="text-xs text-muted-foreground mt-1">
              Charts will appear as metrics are received.
            </p>
          </CardContent>
        </Card>
      )}
    </div>
  );
});
