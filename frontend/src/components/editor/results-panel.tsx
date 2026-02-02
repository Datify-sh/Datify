import { Button } from "@/components/ui/button";
import { Collapsible, CollapsibleContent, CollapsibleTrigger } from "@/components/ui/collapsible";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import { ScrollArea, ScrollBar } from "@/components/ui/scroll-area";
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table";
import { Toggle } from "@/components/ui/toggle";
import { Tooltip, TooltipContent, TooltipTrigger } from "@/components/ui/tooltip";
import { cn } from "@/lib/utils";
import {
  Alert01Icon,
  CheckmarkCircle01Icon,
  CodeIcon,
  Copy01Icon,
  Download01Icon,
  File01Icon,
  GridTableIcon,
} from "@hugeicons/core-free-icons";
import { HugeiconsIcon } from "@hugeicons/react";
import * as React from "react";
import type { KeyValueResult, KvResult, QueryResult, SqlResult } from "./types";

interface ResultsPanelProps {
  result: QueryResult;
  databaseType?: import("@/lib/api/types").DatabaseType;
  executionTime?: number;
}

export function ResultsPanel({ result, executionTime }: ResultsPanelProps) {
  const [activeView, setActiveView] = React.useState<"table" | "json">("table");
  const [kvView, setKvView] = React.useState<"pretty" | "raw">("pretty");
  const [wrapLines, setWrapLines] = React.useState(true);
  const hasKvErrors = result.type === "kv" && result.results.some((item) => item.error);
  const hasInfoOutput = result.type === "kv" && result.results.some((item) => isInfoCommand(item));

  const handleCopyKvOutput = React.useCallback(async () => {
    if (result.type !== "kv") return;
    const text = result.results
      .map((item) => {
        const header = `> ${item.command}`;
        if (item.error) {
          return `${header}\nERROR: ${item.error}`;
        }
        return `${header}\n${formatKvText(item.result)}`;
      })
      .join("\n\n");

    await navigator.clipboard.writeText(text);
  }, [result]);

  const handleDownload = React.useCallback(() => {
    if (result.type !== "sql") return;

    const headers = result.columns.map((c) => c.name).join(",");
    const rows = result.rows
      .map((row) =>
        row
          .map((cell) => {
            const str = String(cell ?? "");
            if (str.includes(",") || str.includes('"') || str.includes("\n")) {
              return `"${str.replace(/"/g, '""')}"`;
            }
            return str;
          })
          .join(","),
      )
      .join("\n");

    const csv = `${headers}\n${rows}`;
    const blob = new Blob([csv], { type: "text/csv" });
    const url = URL.createObjectURL(blob);
    const a = document.createElement("a");
    a.href = url;
    a.download = `query-results-${Date.now()}.csv`;
    document.body.appendChild(a);
    a.click();
    document.body.removeChild(a);
    URL.revokeObjectURL(url);
  }, [result]);

  if (result.error) {
    return (
      <div className="h-full flex flex-col">
        <div className="flex items-center gap-2 px-3 py-2 border-b bg-destructive/5">
          <HugeiconsIcon icon={Alert01Icon} className="size-4 text-destructive" strokeWidth={2} />
          <span className="text-sm font-medium text-destructive">Error</span>
        </div>
        <div className="flex-1 p-4 overflow-auto">
          <pre className="text-sm text-destructive whitespace-pre-wrap font-mono">
            {result.error}
          </pre>
        </div>
      </div>
    );
  }

  return (
    <div className="h-full flex flex-col bg-background">
      {/* Results Header */}
      <div className="flex items-center justify-between px-3 py-2 border-b">
        <div className="flex items-center gap-3">
          <div className="flex items-center gap-1.5 text-sm font-medium">
            <HugeiconsIcon
              icon={hasKvErrors ? Alert01Icon : CheckmarkCircle01Icon}
              className={cn("size-4", hasKvErrors ? "text-amber-500" : "text-green-500")}
              strokeWidth={2}
            />
            <span>{hasKvErrors ? "Completed with errors" : "Success"}</span>
          </div>

          {executionTime !== undefined && (
            <span className="text-xs text-muted-foreground">{executionTime.toFixed(2)}ms</span>
          )}

          {result.type === "sql" && (
            <span className="text-xs text-muted-foreground">
              {result.rowCount} {result.rowCount === 1 ? "row" : "rows"}
              {result.truncated && " (truncated)"}
            </span>
          )}

          {result.type === "kv" && (
            <span className="text-xs text-muted-foreground">
              {result.results.length} {result.results.length === 1 ? "command" : "commands"}
            </span>
          )}
        </div>

        <div className="flex items-center gap-1">
          {result.type === "sql" && (
            <>
              <div className="flex items-center bg-muted rounded-md p-0.5">
                <Button
                  variant={activeView === "table" ? "secondary" : "ghost"}
                  size="icon-sm"
                  className="h-6 w-6"
                  onClick={() => setActiveView("table")}
                >
                  <HugeiconsIcon icon={GridTableIcon} className="size-3.5" strokeWidth={2} />
                </Button>
                <Button
                  variant={activeView === "json" ? "secondary" : "ghost"}
                  size="icon-sm"
                  className="h-6 w-6"
                  onClick={() => setActiveView("json")}
                >
                  <HugeiconsIcon icon={CodeIcon} className="size-3.5" strokeWidth={2} />
                </Button>
              </div>

              <DropdownMenu>
                <DropdownMenuTrigger asChild>
                  <Button variant="ghost" size="icon-sm" className="h-7 w-7">
                    <HugeiconsIcon icon={Download01Icon} className="size-4" strokeWidth={2} />
                  </Button>
                </DropdownMenuTrigger>
                <DropdownMenuContent align="end">
                  <DropdownMenuItem onClick={handleDownload}>
                    <HugeiconsIcon icon={File01Icon} className="size-4 mr-2" strokeWidth={2} />
                    Download as CSV
                  </DropdownMenuItem>
                </DropdownMenuContent>
              </DropdownMenu>
            </>
          )}
          {result.type === "kv" && (
            <>
              {hasInfoOutput && (
                <div className="flex items-center bg-muted rounded-md p-0.5">
                  <Button
                    variant={kvView === "pretty" ? "secondary" : "ghost"}
                    size="icon-sm"
                    className="h-6 w-10 text-[10px]"
                    onClick={() => setKvView("pretty")}
                  >
                    Pretty
                  </Button>
                  <Button
                    variant={kvView === "raw" ? "secondary" : "ghost"}
                    size="icon-sm"
                    className="h-6 w-9 text-[10px]"
                    onClick={() => setKvView("raw")}
                  >
                    Raw
                  </Button>
                </div>
              )}
              <Toggle
                size="sm"
                variant="outline"
                pressed={wrapLines}
                onPressedChange={setWrapLines}
              >
                Wrap
              </Toggle>
              <Tooltip>
                <TooltipTrigger asChild>
                  <Button
                    variant="ghost"
                    size="icon-sm"
                    className="h-7 w-7"
                    onClick={handleCopyKvOutput}
                  >
                    <HugeiconsIcon icon={Copy01Icon} className="size-4" strokeWidth={2} />
                  </Button>
                </TooltipTrigger>
                <TooltipContent>Copy output</TooltipContent>
              </Tooltip>
            </>
          )}
        </div>
      </div>

      {/* Results Content */}
      <div className="flex-1 min-h-0 overflow-hidden">
        {result.type === "sql" ? (
          <SqlResults result={result} view={activeView} />
        ) : (
          <KvResults result={result} view={kvView} wrapLines={wrapLines} />
        )}
      </div>
    </div>
  );
}

function SqlResults({ result, view }: { result: SqlResult; view: "table" | "json" }) {
  if (result.rows.length === 0) {
    return (
      <div className="h-full flex items-center justify-center text-muted-foreground">
        <div className="text-center">
          <HugeiconsIcon
            icon={GridTableIcon}
            className="size-8 mx-auto mb-2 opacity-50"
            strokeWidth={1.5}
          />
          <p className="text-sm">Query executed successfully</p>
          <p className="text-xs">No rows returned</p>
        </div>
      </div>
    );
  }

  if (view === "json") {
    const jsonData = result.rows.map((row) => {
      const obj: Record<string, unknown> = {};
      result.columns.forEach((col, i) => {
        obj[col.name] = row[i];
      });
      return obj;
    });

    return (
      <ScrollArea className="h-full">
        <pre className="p-4 text-xs font-mono">{JSON.stringify(jsonData, null, 2)}</pre>
        <ScrollBar orientation="horizontal" />
      </ScrollArea>
    );
  }

  return (
    <ScrollArea className="h-full">
      <Table>
        <TableHeader className="sticky top-0 bg-background z-10">
          <TableRow className="hover:bg-transparent">
            {result.columns.map((col) => (
              <TableHead key={col.name} className="whitespace-nowrap font-mono text-xs bg-muted/50">
                <div className="flex flex-col">
                  <span>{col.name}</span>
                  <span className="text-[10px] text-muted-foreground font-normal">{col.type}</span>
                </div>
              </TableHead>
            ))}
          </TableRow>
        </TableHeader>
        <TableBody>
          {result.rows.map((row, rowIndex) => (
            <TableRow key={rowIndex} className="text-xs">
              {row.map((cell, cellIndex) => (
                <TableCell
                  key={cellIndex}
                  className={cn(
                    "whitespace-nowrap font-mono max-w-[200px] truncate",
                    cell === null && "text-muted-foreground italic",
                  )}
                  title={cell !== null ? String(cell) : "NULL"}
                >
                  {cell === null
                    ? "NULL"
                    : typeof cell === "object"
                      ? JSON.stringify(cell)
                      : String(cell)}
                </TableCell>
              ))}
            </TableRow>
          ))}
        </TableBody>
      </Table>
      <ScrollBar orientation="horizontal" />
    </ScrollArea>
  );
}

function KvResults({
  result,
  view,
  wrapLines,
}: {
  result: KvResult;
  view: "pretty" | "raw";
  wrapLines: boolean;
}) {
  if (result.results.length === 0) {
    return (
      <div className="h-full flex items-center justify-center text-muted-foreground">
        <div className="text-center">
          <HugeiconsIcon
            icon={CodeIcon}
            className="size-8 mx-auto mb-2 opacity-50"
            strokeWidth={1.5}
          />
          <p className="text-sm">No commands executed</p>
          <p className="text-xs">Enter a command and run it to see output</p>
        </div>
      </div>
    );
  }

  return (
    <ScrollArea className="h-full">
      <div className="p-4 space-y-3">
        {result.results.map((item, index) => (
          <div
            key={index}
            className={cn(
              "rounded-lg border p-3",
              item.error ? "border-destructive/30 bg-destructive/5" : "bg-muted/30",
            )}
          >
            <div className="flex items-center justify-between gap-3 mb-2">
              <code className="text-xs font-mono bg-background px-2 py-0.5 rounded border truncate">
                {item.command}
              </code>
              {item.error && <span className="text-xs text-destructive font-medium">Error</span>}
            </div>

            {item.error ? (
              <p className="text-sm text-destructive">{item.error}</p>
            ) : (
              <KvOutput item={item} view={view} wrapLines={wrapLines} />
            )}
          </div>
        ))}
      </div>
      <ScrollBar orientation="horizontal" />
    </ScrollArea>
  );
}

function KvOutput({
  item,
  view,
  wrapLines,
}: {
  item: KeyValueResult;
  view: "pretty" | "raw";
  wrapLines: boolean;
}) {
  const rawText = formatKvText(item.result);
  const isInfo = isInfoCommand(item);

  if (view === "pretty" && isInfo) {
    const sections = parseInfoOutput(rawText);
    return (
      <div className="space-y-2">
        {sections.map((section) => (
          <Collapsible key={section.title} defaultOpen>
            <CollapsibleTrigger className="group flex w-full items-center justify-between rounded-md bg-background px-2 py-1 text-[11px] font-semibold text-muted-foreground">
              <span>{section.title}</span>
              <span className="text-[10px] group-data-[state=open]:opacity-60">
                {section.entries.length} keys
              </span>
            </CollapsibleTrigger>
            <CollapsibleContent className="mt-2 rounded-md border bg-background p-2">
              <div className="grid gap-y-1 text-[11px] font-mono">
                {section.entries.map((entry) => (
                  <div
                    key={`${section.title}-${entry.key}`}
                    className="grid grid-cols-[minmax(140px,220px)_1fr] gap-x-3"
                  >
                    <span className="text-muted-foreground">{entry.key}</span>
                    <span className={cn("text-foreground", wrapLines ? "break-words" : "truncate")}>
                      {entry.value}
                    </span>
                  </div>
                ))}
                {section.extras.map((line) => (
                  <div key={`${section.title}-${line}`} className="text-muted-foreground">
                    {line}
                  </div>
                ))}
              </div>
            </CollapsibleContent>
          </Collapsible>
        ))}
      </div>
    );
  }

  return (
    <pre
      className={cn(
        "text-xs font-mono bg-background p-2 rounded border overflow-auto",
        wrapLines ? "whitespace-pre-wrap break-words" : "whitespace-pre",
      )}
    >
      {rawText}
    </pre>
  );
}

function formatKvText(result: unknown): string {
  if (result === null || result === undefined) return "";
  if (typeof result === "string") return result;
  return JSON.stringify(result, null, 2);
}

function isInfoCommand(item: KeyValueResult): boolean {
  return item.command.trim().toUpperCase().startsWith("INFO");
}

type InfoSection = {
  title: string;
  entries: { key: string; value: string }[];
  extras: string[];
};

function parseInfoOutput(raw: string): InfoSection[] {
  const sections: InfoSection[] = [];
  let current: InfoSection = { title: "General", entries: [], extras: [] };

  for (const line of raw.split(/\r?\n/)) {
    const trimmed = line.trim();
    if (!trimmed) continue;

    if (trimmed.startsWith("#")) {
      if (current.entries.length || current.extras.length) {
        sections.push(current);
      }
      current = {
        title: trimmed.replace(/^#+\s*/, "") || "General",
        entries: [],
        extras: [],
      };
      continue;
    }

    const separatorIndex = trimmed.indexOf(":");
    if (separatorIndex > 0) {
      const key = trimmed.slice(0, separatorIndex).trim();
      const value = trimmed.slice(separatorIndex + 1).trim();
      current.entries.push({ key, value });
    } else {
      current.extras.push(trimmed);
    }
  }

  if (current.entries.length || current.extras.length) {
    sections.push(current);
  }

  return sections;
}
