import { ClockIcon, File01Icon } from "@hugeicons/core-free-icons";
import { HugeiconsIcon } from "@hugeicons/react";
import { useQuery } from "@tanstack/react-query";
import * as React from "react";

import { Button } from "@/components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Skeleton } from "@/components/ui/skeleton";
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table";
import { databasesApi } from "@/lib/api";
import type { QueryLogEntry } from "@/lib/api/types";

interface QueryLogsPanelProps {
  databaseId: string;
  isRunning: boolean;
}

type SortBy = "total_time" | "avg_time" | "calls";

function formatMs(ms: number): string {
  if (ms >= 1000) return `${(ms / 1000).toFixed(2)}s`;
  if (ms >= 1) return `${ms.toFixed(2)}ms`;
  return `${(ms * 1000).toFixed(0)}μs`;
}

function formatNumber(num: number): string {
  if (num >= 1000000) return `${(num / 1000000).toFixed(1)}M`;
  if (num >= 1000) return `${(num / 1000).toFixed(1)}K`;
  return num.toString();
}

function truncateQuery(query: string, maxLength = 100): string {
  if (query.length <= maxLength) return query;
  return `${query.slice(0, maxLength)}...`;
}

export function QueryLogsPanel({ databaseId, isRunning }: QueryLogsPanelProps) {
  const [sortBy, setSortBy] = React.useState<SortBy>("total_time");
  const [limit, setLimit] = React.useState(50);

  const { data, isLoading, error, isPlaceholderData } = useQuery({
    queryKey: ["queryLogs", databaseId, sortBy, limit],
    queryFn: () => databasesApi.queryLogs(databaseId, { sort_by: sortBy, limit }),
    enabled: isRunning,
    refetchInterval: 30000,
    placeholderData: (previousData) => previousData,
  });

  if (!isRunning) {
    return (
      <Card>
        <CardContent className="flex flex-col items-center justify-center py-12">
          <HugeiconsIcon
            icon={File01Icon}
            className="size-12 text-muted-foreground"
            strokeWidth={1.5}
          />
          <p className="mt-4 text-muted-foreground">Start the database to view query logs</p>
        </CardContent>
      </Card>
    );
  }

  if (isLoading) {
    return (
      <Card>
        <CardHeader>
          <CardTitle className="text-base">Query Logs</CardTitle>
          <CardDescription>Slowest and most frequent queries</CardDescription>
        </CardHeader>
        <CardContent>
          <div className="space-y-2">
            <Skeleton className="h-12" />
            <Skeleton className="h-12" />
            <Skeleton className="h-12" />
            <Skeleton className="h-12" />
            <Skeleton className="h-12" />
          </div>
        </CardContent>
      </Card>
    );
  }

  if (error) {
    return (
      <Card>
        <CardContent className="flex flex-col items-center justify-center py-12">
          <p className="text-sm text-muted-foreground">Failed to load query logs</p>
          <p className="text-xs text-muted-foreground mt-1">
            pg_stat_statements extension may not be enabled
          </p>
        </CardContent>
      </Card>
    );
  }

  const entries = data?.entries ?? [];
  const totalQueries = data?.total_queries ?? 0;

  return (
    <Card>
      <CardHeader>
        <div className="flex items-center justify-between">
          <div>
            <CardTitle className="text-base">Query Logs</CardTitle>
            <CardDescription>
              {totalQueries > 0
                ? `${totalQueries} unique queries tracked`
                : "No queries tracked yet"}
            </CardDescription>
          </div>
          <div className="flex items-center gap-2">
            <span className="text-xs text-muted-foreground">Sort by:</span>
            <div className="flex gap-1">
              <Button
                variant={sortBy === "total_time" ? "default" : "outline"}
                size="sm"
                onClick={() => setSortBy("total_time")}
              >
                Total Time
              </Button>
              <Button
                variant={sortBy === "avg_time" ? "default" : "outline"}
                size="sm"
                onClick={() => setSortBy("avg_time")}
              >
                Avg Time
              </Button>
              <Button
                variant={sortBy === "calls" ? "default" : "outline"}
                size="sm"
                onClick={() => setSortBy("calls")}
              >
                Calls
              </Button>
            </div>
          </div>
        </div>
      </CardHeader>
      <CardContent>
        {entries.length === 0 ? (
          <div className="flex flex-col items-center justify-center py-8">
            <HugeiconsIcon
              icon={ClockIcon}
              className="size-8 text-muted-foreground"
              strokeWidth={1.5}
            />
            <p className="mt-2 text-sm text-muted-foreground">No queries recorded yet</p>
            <p className="text-xs text-muted-foreground mt-1">
              Run some queries to see statistics here
            </p>
          </div>
        ) : (
          <>
            <Table>
              <TableHeader>
                <TableRow>
                  <TableHead className="w-[40%]">Query</TableHead>
                  <TableHead className="text-right">Calls</TableHead>
                  <TableHead className="text-right">Avg Time</TableHead>
                  <TableHead className="text-right">Total Time</TableHead>
                  <TableHead className="text-right">Rows</TableHead>
                </TableRow>
              </TableHeader>
              <TableBody>
                {entries.map((entry) => (
                  <QueryRow
                    key={`${entry.query.slice(0, 50)}-${entry.calls}-${entry.total_time_ms}`}
                    entry={entry}
                  />
                ))}
              </TableBody>
            </Table>
            {entries.length >= limit && (
              <div className="flex justify-center pt-4">
                <Button
                  variant="outline"
                  size="sm"
                  onClick={() => setLimit(limit + 50)}
                  disabled={isPlaceholderData}
                >
                  {isPlaceholderData ? "Loading..." : "Load More"}
                </Button>
              </div>
            )}
          </>
        )}
      </CardContent>
    </Card>
  );
}

function QueryRow({ entry }: { entry: QueryLogEntry }) {
  const [expanded, setExpanded] = React.useState(false);

  return (
    <TableRow className="cursor-pointer" onClick={() => setExpanded(!expanded)}>
      <TableCell className="font-mono text-xs">
        <div className={expanded ? "whitespace-pre-wrap" : "truncate max-w-md"}>
          {expanded ? entry.query : truncateQuery(entry.query, 80)}
        </div>
      </TableCell>
      <TableCell className="text-right tabular-nums">{formatNumber(entry.calls)}</TableCell>
      <TableCell className="text-right tabular-nums">{formatMs(entry.avg_time_ms)}</TableCell>
      <TableCell className="text-right tabular-nums">{formatMs(entry.total_time_ms)}</TableCell>
      <TableCell className="text-right tabular-nums">
        {formatNumber(entry.rows)}
        <span className="text-muted-foreground text-xs ml-1">
          ({entry.rows_per_call.toFixed(1)}/call)
        </span>
      </TableCell>
    </TableRow>
  );
}
