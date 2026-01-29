import * as React from "react";
import { useQuery } from "@tanstack/react-query";
import { HugeiconsIcon, type IconSvgElement } from "@hugeicons/react";
import {
  DatabaseIcon,
  HardDriveIcon,
  ClockIcon,
  TableIcon,
  LinkIcon,
  BarChartIcon,
  ArrowUpIcon,
  ArrowDownIcon,
} from "@hugeicons/core-free-icons";
import { LineChart, Line, AreaChart, Area, XAxis, YAxis, CartesianGrid } from "recharts";
import { format } from "date-fns";

import { databasesApi } from "@/lib/api";
import type { TimeRange, DatabaseMetrics, MetricsHistoryPoint } from "@/lib/api/types";
import { useMetricsStream } from "@/hooks/use-metrics-stream";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Tabs, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { ChartContainer, ChartTooltip, ChartTooltipContent } from "@/components/ui/chart";
import { Skeleton } from "@/components/ui/skeleton";
import { Badge } from "@/components/ui/badge";

interface MetricsPanelProps {
  databaseId: string;
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

function formatBytes(bytes: number): string {
  if (bytes === 0) return "0 B";
  const k = 1024;
  const sizes = ["B", "KB", "MB", "GB", "TB"];
  const i = Math.floor(Math.log(bytes) / Math.log(k));
  return `${Number.parseFloat((bytes / k ** i).toFixed(1))} ${sizes[i]}`;
}

function formatNumber(num: number): string {
  if (num >= 1000000) return `${(num / 1000000).toFixed(1)}M`;
  if (num >= 1000) return `${(num / 1000).toFixed(1)}K`;
  return num.toFixed(0);
}

function formatMs(ms: number): string {
  if (ms >= 1000) return `${(ms / 1000).toFixed(2)}s`;
  return `${ms.toFixed(2)}ms`;
}

interface MetricCardProps {
  title: string;
  value: string | number;
  subtitle?: string;
  icon: IconSvgElement;
  trend?: "up" | "down";
  trendValue?: string;
}

function MetricCard({ title, value, subtitle, icon, trend, trendValue }: MetricCardProps) {
  return (
    <Card>
      <CardContent className="pt-4">
        <div className="flex items-start justify-between">
          <div className="space-y-1">
            <p className="text-xs text-muted-foreground">{title}</p>
            <p className="text-2xl font-bold tabular-nums">{value}</p>
            {subtitle && <p className="text-xs text-muted-foreground">{subtitle}</p>}
          </div>
          <div className="flex flex-col items-end gap-1">
            <div className="rounded-md bg-muted p-2">
              <HugeiconsIcon icon={icon} className="size-4 text-muted-foreground" strokeWidth={2} />
            </div>
            {trend && trendValue && (
              <div
                className={`flex items-center gap-0.5 text-xs ${trend === "up" ? "text-green-500" : "text-red-500"}`}
              >
                <HugeiconsIcon
                  icon={trend === "up" ? ArrowUpIcon : ArrowDownIcon}
                  className="size-3"
                  strokeWidth={2}
                />
                {trendValue}
              </div>
            )}
          </div>
        </div>
      </CardContent>
    </Card>
  );
}

const chartConfig = {
  queries: { label: "Queries", color: "hsl(var(--chart-1))" },
  latency: { label: "Latency", color: "hsl(var(--chart-2))" },
  cpu: { label: "CPU", color: "hsl(var(--chart-3))" },
  memory: { label: "Memory", color: "hsl(var(--chart-4))" },
  rowsRead: { label: "Rows Read", color: "hsl(var(--chart-1))" },
  rowsWritten: { label: "Rows Written", color: "hsl(var(--chart-2))" },
  connections: { label: "Connections", color: "hsl(var(--chart-5))" },
};

export const MetricsPanel = React.memo(function MetricsPanel({
  databaseId,
  isRunning,
}: MetricsPanelProps) {
  const [timeRange, setTimeRange] = React.useState<TimeRange>("last_15_min");
  const isRealtime = timeRange === "realtime";

  const handleTimeRangeChange = React.useCallback((v: string) => setTimeRange(v as TimeRange), []);

  const { metrics: realtimeMetrics, isConnected } = useMetricsStream(databaseId, {
    enabled: isRunning && isRealtime,
  });

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

  const metrics: DatabaseMetrics | null = isRealtime
    ? realtimeMetrics
    : (currentMetrics?.metrics ?? null);

  const chartData = React.useMemo(() => {
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
  }, [historyData?.points]);

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

  return (
    <div className="space-y-6">
      {/* Time Range Selector */}
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

      {/* Metric Cards */}
      {isLoading ? (
        <div className="grid gap-4 sm:grid-cols-2 lg:grid-cols-4">
          <Skeleton className="h-24" />
          <Skeleton className="h-24" />
          <Skeleton className="h-24" />
          <Skeleton className="h-24" />
          <Skeleton className="h-24" />
          <Skeleton className="h-24" />
          <Skeleton className="h-24" />
          <Skeleton className="h-24" />
        </div>
      ) : metrics ? (
        <div className="grid gap-4 sm:grid-cols-2 lg:grid-cols-4">
          <MetricCard
            title="Total Queries"
            value={formatNumber(metrics.queries.total_queries)}
            subtitle={`${metrics.queries.queries_per_sec.toFixed(1)}/sec`}
            icon={DatabaseIcon}
          />
          <MetricCard
            title="Rows Read"
            value={formatNumber(metrics.rows.rows_read)}
            subtitle={`${formatNumber(metrics.rows.total_rows)} total rows`}
            icon={ArrowUpIcon}
          />
          <MetricCard
            title="Rows Written"
            value={formatNumber(metrics.rows.rows_written)}
            icon={ArrowDownIcon}
          />
          <MetricCard
            title="Total Tables"
            value={metrics.tables.total_tables}
            subtitle={`${metrics.tables.total_indexes} indexes`}
            icon={TableIcon}
          />
          <MetricCard
            title="Storage Used"
            value={formatBytes(metrics.storage.database_size_bytes)}
            subtitle={`${metrics.storage.storage_percent.toFixed(1)}% of limit`}
            icon={HardDriveIcon}
          />
          <MetricCard
            title="Avg Latency"
            value={formatMs(metrics.queries.avg_latency_ms)}
            subtitle={`Max: ${formatMs(metrics.queries.max_latency_ms)}`}
            icon={ClockIcon}
          />
          <MetricCard
            title="Connections"
            value={metrics.connections.active_connections}
            subtitle={`${metrics.connections.idle_connections} idle / ${metrics.connections.max_connections} max`}
            icon={LinkIcon}
          />
        </div>
      ) : null}

      {/* Charts */}
      {!isRealtime && chartData.length > 0 && (
        <div className="grid gap-6 lg:grid-cols-2">
          {/* Query Latency Chart */}
          <Card>
            <CardHeader className="pb-2">
              <CardTitle className="text-sm font-medium">Query Latency</CardTitle>
            </CardHeader>
            <CardContent>
              <ChartContainer config={chartConfig} className="h-[200px]">
                <LineChart data={chartData}>
                  <CartesianGrid strokeDasharray="3 3" className="stroke-muted" />
                  <XAxis dataKey="time" tick={{ fontSize: 10 }} tickLine={false} axisLine={false} />
                  <YAxis
                    tick={{ fontSize: 10 }}
                    tickLine={false}
                    axisLine={false}
                    tickFormatter={(v) => `${v}ms`}
                  />
                  <ChartTooltip content={<ChartTooltipContent />} />
                  <Line
                    type="monotone"
                    dataKey="latency"
                    stroke="var(--color-latency)"
                    strokeWidth={2}
                    dot={false}
                  />
                </LineChart>
              </ChartContainer>
            </CardContent>
          </Card>

          {/* Queries/sec Chart */}
          <Card>
            <CardHeader className="pb-2">
              <CardTitle className="text-sm font-medium">Queries per Second</CardTitle>
            </CardHeader>
            <CardContent>
              <ChartContainer config={chartConfig} className="h-[200px]">
                <AreaChart data={chartData}>
                  <CartesianGrid strokeDasharray="3 3" className="stroke-muted" />
                  <XAxis dataKey="time" tick={{ fontSize: 10 }} tickLine={false} axisLine={false} />
                  <YAxis tick={{ fontSize: 10 }} tickLine={false} axisLine={false} />
                  <ChartTooltip content={<ChartTooltipContent />} />
                  <Area
                    type="monotone"
                    dataKey="queries"
                    stroke="var(--color-queries)"
                    fill="var(--color-queries)"
                    fillOpacity={0.2}
                    strokeWidth={2}
                  />
                </AreaChart>
              </ChartContainer>
            </CardContent>
          </Card>

          {/* Rows Read/Written Chart */}
          <Card>
            <CardHeader className="pb-2">
              <CardTitle className="text-sm font-medium">Rows Read / Written</CardTitle>
            </CardHeader>
            <CardContent>
              <ChartContainer config={chartConfig} className="h-[200px]">
                <LineChart data={chartData}>
                  <CartesianGrid strokeDasharray="3 3" className="stroke-muted" />
                  <XAxis dataKey="time" tick={{ fontSize: 10 }} tickLine={false} axisLine={false} />
                  <YAxis
                    tick={{ fontSize: 10 }}
                    tickLine={false}
                    axisLine={false}
                    tickFormatter={(v) => formatNumber(v)}
                  />
                  <ChartTooltip content={<ChartTooltipContent />} />
                  <Line
                    type="monotone"
                    dataKey="rowsRead"
                    stroke="var(--color-rowsRead)"
                    strokeWidth={2}
                    dot={false}
                  />
                  <Line
                    type="monotone"
                    dataKey="rowsWritten"
                    stroke="var(--color-rowsWritten)"
                    strokeWidth={2}
                    dot={false}
                  />
                </LineChart>
              </ChartContainer>
            </CardContent>
          </Card>

          {/* Resource Usage Chart */}
          <Card>
            <CardHeader className="pb-2">
              <CardTitle className="text-sm font-medium">Resource Usage</CardTitle>
            </CardHeader>
            <CardContent>
              <ChartContainer config={chartConfig} className="h-[200px]">
                <AreaChart data={chartData}>
                  <CartesianGrid strokeDasharray="3 3" className="stroke-muted" />
                  <XAxis dataKey="time" tick={{ fontSize: 10 }} tickLine={false} axisLine={false} />
                  <YAxis
                    tick={{ fontSize: 10 }}
                    tickLine={false}
                    axisLine={false}
                    tickFormatter={(v) => `${v}%`}
                    domain={[0, 100]}
                  />
                  <ChartTooltip content={<ChartTooltipContent />} />
                  <Area
                    type="monotone"
                    dataKey="cpu"
                    stroke="var(--color-cpu)"
                    fill="var(--color-cpu)"
                    fillOpacity={0.2}
                    strokeWidth={2}
                  />
                  <Area
                    type="monotone"
                    dataKey="memory"
                    stroke="var(--color-memory)"
                    fill="var(--color-memory)"
                    fillOpacity={0.2}
                    strokeWidth={2}
                  />
                </AreaChart>
              </ChartContainer>
            </CardContent>
          </Card>
        </div>
      )}

      {/* No data message for charts */}
      {!isRealtime && chartData.length === 0 && !historyLoading && (
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
    </div>
  );
});
