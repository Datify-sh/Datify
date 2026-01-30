import {
  ArrowDownIcon,
  ArrowUpIcon,
  ClockIcon,
  CpuIcon,
  DatabaseIcon,
  HardDriveIcon,
  LinkIcon,
  SmartPhone01Icon,
} from "@hugeicons/core-free-icons";
import { HugeiconsIcon, type IconSvgElement } from "@hugeicons/react";
import * as React from "react";

import { Card, CardContent } from "@/components/ui/card";
import type { DatabaseMetrics } from "@/lib/api/types";

interface PostgresMetricsPanelProps {
  metrics: DatabaseMetrics;
}

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
}

function MetricCard({ title, value, subtitle, icon }: MetricCardProps) {
  return (
    <Card>
      <CardContent className="pt-4">
        <div className="flex items-start justify-between">
          <div className="space-y-1">
            <p className="text-xs text-muted-foreground">{title}</p>
            <p className="text-2xl font-bold tabular-nums">{value}</p>
            {subtitle && <p className="text-xs text-muted-foreground">{subtitle}</p>}
          </div>
          <div className="rounded-md bg-muted p-2">
            <HugeiconsIcon icon={icon} className="size-4 text-muted-foreground" strokeWidth={2} />
          </div>
        </div>
      </CardContent>
    </Card>
  );
}

export const PostgresMetricsPanel = React.memo(function PostgresMetricsPanel({
  metrics,
}: PostgresMetricsPanelProps) {
  return (
    <div className="grid gap-4 sm:grid-cols-2 lg:grid-cols-4">
      <MetricCard
        title="CPU Usage"
        value={`${metrics.resources.cpu_percent.toFixed(1)}%`}
        icon={CpuIcon}
      />
      <MetricCard
        title="Memory Usage"
        value={formatBytes(metrics.resources.memory_used_bytes)}
        subtitle={`${metrics.resources.memory_percent.toFixed(1)}% of ${formatBytes(metrics.resources.memory_limit_bytes)}`}
        icon={SmartPhone01Icon}
      />
      <MetricCard
        title="Storage Used"
        value={formatBytes(metrics.storage.database_size_bytes)}
        subtitle={`${metrics.storage.storage_percent.toFixed(1)}% of limit`}
        icon={HardDriveIcon}
      />
      <MetricCard
        title="Connections"
        value={metrics.connections.active_connections}
        subtitle={`${metrics.connections.idle_connections} idle / ${metrics.connections.max_connections} max`}
        icon={LinkIcon}
      />
      <MetricCard
        title="Total Queries"
        value={formatNumber(metrics.queries.total_queries)}
        subtitle={`${metrics.queries.queries_per_sec.toFixed(1)}/sec`}
        icon={DatabaseIcon}
      />
      <MetricCard
        title="Avg Latency"
        value={formatMs(metrics.queries.avg_latency_ms)}
        subtitle={`Max: ${formatMs(metrics.queries.max_latency_ms)}`}
        icon={ClockIcon}
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
        subtitle={`${metrics.tables.total_tables} tables, ${metrics.tables.total_indexes} indexes`}
        icon={ArrowDownIcon}
      />
    </div>
  );
});
