import {
  CommandIcon,
  CpuIcon,
  Database01Icon,
  HardDriveIcon,
  LinkIcon,
  SmartPhone01Icon,
  Target01Icon,
} from "@hugeicons/core-free-icons";
import { HugeiconsIcon, type IconSvgElement } from "@hugeicons/react";
import * as React from "react";

import { Card, CardContent } from "@/components/ui/card";
import type { KeyValueMetrics } from "@/lib/api/types";

interface KeyValueMetricsPanelProps {
  metrics: KeyValueMetrics;
  databaseType: "redis" | "valkey";
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

export const KeyValueMetricsPanel = React.memo(function KeyValueMetricsPanel({
  metrics,
}: KeyValueMetricsPanelProps) {
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
        title="Used Memory"
        value={formatBytes(metrics.memory.used_memory)}
        subtitle={`Peak: ${formatBytes(metrics.memory.used_memory_peak)}`}
        icon={HardDriveIcon}
      />
      <MetricCard
        title="Connected Clients"
        value={metrics.clients.connected_clients}
        subtitle={`${metrics.clients.blocked_clients} blocked / ${metrics.clients.max_clients} max`}
        icon={LinkIcon}
      />
      <MetricCard
        title="Total Keys"
        value={formatNumber(metrics.keys.total_keys)}
        subtitle={`${formatNumber(metrics.keys.keys_with_expiry)} with expiry`}
        icon={Database01Icon}
      />
      <MetricCard
        title="Commands"
        value={formatNumber(metrics.commands.total_commands)}
        subtitle={`${metrics.commands.ops_per_sec.toFixed(1)} ops/sec`}
        icon={CommandIcon}
      />
      <MetricCard
        title="Hit Rate"
        value={`${metrics.commands.hit_rate.toFixed(1)}%`}
        subtitle={`${formatNumber(metrics.commands.keyspace_hits)} hits / ${formatNumber(metrics.commands.keyspace_misses)} misses`}
        icon={Target01Icon}
      />
      <MetricCard
        title="Evicted Keys"
        value={formatNumber(metrics.keys.evicted_keys)}
        subtitle={`${formatNumber(metrics.keys.expired_keys)} expired`}
        icon={Database01Icon}
      />
    </div>
  );
});
