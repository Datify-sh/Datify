import { EditorToolbar } from "@/components/editor/editor-toolbar";
import { QueryEditor } from "@/components/editor/query-editor";
import { ResultsPanel } from "@/components/editor/results-panel";
import type { KeyValueResult, KvResult, QueryResult, SqlResult } from "@/components/editor/types";
import { useQueryTabs } from "@/components/editor/use-query-tabs";
import { Button } from "@/components/ui/button";
import { ScrollArea, ScrollBar } from "@/components/ui/scroll-area";
import { Spinner } from "@/components/ui/spinner";
import { Tooltip, TooltipContent, TooltipTrigger } from "@/components/ui/tooltip";
import { getErrorMessage } from "@/lib/api";
import type { DatabaseResponse, DatabaseType } from "@/lib/api/types";
import { cn } from "@/lib/utils";
import { Add01Icon, Cancel01Icon, CodeIcon, Database01Icon } from "@hugeicons/core-free-icons";
import { HugeiconsIcon } from "@hugeicons/react";
import * as React from "react";
import { Group, Panel, Separator } from "react-resizable-panels";

interface EditorPanelProps {
  database: DatabaseResponse;
  isRunning: boolean;
}

/**
 * Render a tabbed query editor and results panel for the given database.
 *
 * Displays a multi-tab query/command editor (SQL for Postgres, command editor for KV stores), a toolbar with run controls, per-tab execution state and results, and a message when the database is not running. Tabs can be added, closed, switched, edited, and executed; results are shown below the editor when available.
 *
 * @param database - Database metadata used to choose editor mode, label UI, and identify the target for query execution
 * @param isRunning - When false, shows a disabled/placeholder state indicating the database must be started to run queries
 * @returns The React element for the editor panel UI
 */
export function EditorPanel({ database, isRunning }: EditorPanelProps) {
  const {
    tabs,
    activeTabId,
    addTab,
    closeTab,
    setActiveTab,
    updateTabContent,
    updateTabResult,
    setTabExecuting,
  } = useQueryTabs(database.database_type);

  const activeTab = tabs.find((t) => t.id === activeTabId);
  const editorLabel = database.database_type === "postgres" ? "SQL Editor" : "Command Editor";

  const handleRunQuery = React.useCallback(async () => {
    if (!activeTab || !isRunning) return;
    if (!activeTab.content.trim()) return;

    const startedAt = performance.now();
    setTabExecuting(activeTabId, true);
    try {
      const result = await executeQuery(database.id, database.database_type, activeTab.content);
      updateTabResult(activeTabId, result, performance.now() - startedAt);
    } catch (error) {
      const message = getErrorMessage(error, "Failed to execute query");
      updateTabResult(
        activeTabId,
        buildErrorResult(activeTab.type, message),
        performance.now() - startedAt,
      );
    } finally {
      setTabExecuting(activeTabId, false);
    }
  }, [activeTab, activeTabId, database, isRunning, setTabExecuting, updateTabResult]);

  if (!isRunning) {
    return (
      <div className="flex flex-col items-center justify-center py-16 text-center">
        <div className="flex size-12 items-center justify-center rounded-full bg-muted">
          <HugeiconsIcon
            icon={Database01Icon}
            className="size-6 text-muted-foreground"
            strokeWidth={2}
          />
        </div>
        <h3 className="mt-4 text-lg font-semibold">Database is not running</h3>
        <p className="mt-1 text-sm text-muted-foreground max-w-sm">
          Start the database to use the {editorLabel} and execute queries.
        </p>
      </div>
    );
  }

  return (
    <div className="flex flex-col h-[calc(100vh-280px)] min-h-[500px] border rounded-lg bg-card">
      {/* Tab Bar */}
      <div className="flex items-center border-b bg-muted/30">
        <ScrollArea className="flex-1 whitespace-nowrap">
          <div className="flex items-center" role="tablist" aria-label="Query tabs">
            {tabs.map((tab) => (
              <div
                role="tab"
                tabIndex={0}
                key={tab.id}
                className={cn(
                  "group flex items-center gap-2 px-3 py-2 text-xs font-medium cursor-pointer border-r transition-colors outline-none focus-visible:ring-2 focus-visible:ring-primary/30",
                  activeTabId === tab.id
                    ? "bg-background text-foreground"
                    : "bg-muted/50 text-muted-foreground hover:bg-muted hover:text-foreground",
                )}
                aria-selected={activeTabId === tab.id}
                title={tab.title}
                onClick={() => setActiveTab(tab.id)}
                onKeyDown={(e) => {
                  if (e.key === "Enter" || e.key === " ") {
                    e.preventDefault();
                    setActiveTab(tab.id);
                  }
                }}
              >
                <HugeiconsIcon
                  icon={tab.type === "sql" ? Database01Icon : CodeIcon}
                  className="size-3.5"
                  strokeWidth={2}
                />
                <span className="max-w-[120px] truncate">{tab.title}</span>
                {tab.isExecuting && <Spinner className="size-3" />}
                {tabs.length > 1 && (
                  <button
                    type="button"
                    onClick={(e) => {
                      e.stopPropagation();
                      closeTab(tab.id);
                    }}
                    className="opacity-0 group-hover:opacity-100 focus-visible:opacity-100 hover:text-destructive transition-opacity"
                    aria-label="Close tab"
                  >
                    <HugeiconsIcon icon={Cancel01Icon} className="size-3" strokeWidth={2} />
                  </button>
                )}
              </div>
            ))}
          </div>
          <ScrollBar orientation="horizontal" />
        </ScrollArea>
        <Tooltip>
          <TooltipTrigger asChild>
            <Button
              variant="ghost"
              size="icon-sm"
              className="h-8 w-8 shrink-0 rounded-none border-l"
              onClick={() => addTab()}
            >
              <HugeiconsIcon icon={Add01Icon} className="size-4" strokeWidth={2} />
            </Button>
          </TooltipTrigger>
          <TooltipContent>New Query Tab</TooltipContent>
        </Tooltip>
      </div>

      {/* Toolbar */}
      <EditorToolbar
        databaseType={database.database_type}
        onRun={handleRunQuery}
        isExecuting={activeTab?.isExecuting ?? false}
        hasContent={!!activeTab?.content?.trim()}
      />

      {/* Editor and Results Split */}
      <div className="flex-1 flex flex-col min-h-0">
        {activeTab?.result ? (
          <Group orientation="vertical" className="flex-1 min-h-0">
            <Panel defaultSize={60} minSize={30} className="min-h-[200px]">
              {activeTab && (
                <QueryEditor
                  key={activeTab.id}
                  content={activeTab.content}
                  onChange={(content) => updateTabContent(activeTab.id, content)}
                  databaseType={database.database_type}
                  onRun={handleRunQuery}
                />
              )}
            </Panel>
            <Separator className="h-2 bg-border/50 hover:bg-border transition-colors cursor-row-resize">
              <div className="mx-auto h-1 w-10 rounded-full bg-muted-foreground/40" />
            </Separator>
            <Panel defaultSize={40} minSize={20} className="min-h-[160px]">
              <ResultsPanel
                result={activeTab.result}
                databaseType={database.database_type}
                executionTime={activeTab.executionTime}
              />
            </Panel>
          </Group>
        ) : (
          <div className="flex-1 min-h-[200px]">
            {activeTab && (
              <QueryEditor
                key={activeTab.id}
                content={activeTab.content}
                onChange={(content) => updateTabContent(activeTab.id, content)}
                databaseType={database.database_type}
                onRun={handleRunQuery}
              />
            )}
          </div>
        )}
      </div>
    </div>
  );
}

/**
 * Execute the provided SQL query or newline-separated commands against the specified database and return a structured result.
 *
 * @param databaseId - The target database identifier.
 * @param databaseType - The database engine type (e.g., `"postgres"` or a key-value/command type).
 * @param content - SQL text or newline-separated commands; for command-style databases, lines starting with `#`, `//`, or `--` are ignored.
 * @returns A QueryResult: a `SqlResult` when `databaseType` is `"postgres"` (includes `columns`, `rows`, `rowCount`, `truncated`, and `error`), or a `KeyValueResult` for command/kv databases (includes per-command `results` and `error`).
 */
async function executeQuery(
  databaseId: string,
  databaseType: DatabaseType,
  content: string,
): Promise<QueryResult> {
  const apiClient = (await import("@/lib/api/client")).apiClient;

  if (databaseType === "postgres") {
    const response = await apiClient.post<{
      columns?: { name: string; type: string }[];
      rows?: unknown[][];
      row_count?: number;
      truncated?: boolean;
    }>(`/databases/${databaseId}/query`, {
      sql: content,
      limit: 1000,
    });
    return {
      type: "sql",
      columns: response.columns || [],
      rows: response.rows || [],
      rowCount: response.row_count || 0,
      truncated: response.truncated || false,
      error: null,
    };
  }
  // For Redis/Valkey, execute as a command
  const lines = content
    .split("\n")
    .map((line) => line.trim())
    .filter(
      (line) => line && !line.startsWith("#") && !line.startsWith("//") && !line.startsWith("--"),
    );
  const results: KeyValueResult[] = [];

  for (const line of lines) {
    try {
      const response = await apiClient.post<{ result?: unknown }>(`/databases/${databaseId}/kv`, {
        command: line,
      });
      results.push({
        command: line,
        result: response.result || "OK",
        error: null,
      });
    } catch (err) {
      results.push({
        command: line,
        result: null,
        error: err instanceof Error ? err.message : "Unknown error",
      });
    }
  }

  return {
    type: "kv",
    results,
    error: null,
  };
}

/**
 * Create a QueryResult representing an error for the specified result type.
 *
 * @param type - The result type to construct (`"sql"` or `"kv"`).
 * @param message - The error message to include in the result.
 * @returns A QueryResult with empty data fields for the chosen type and `error` set to `message`.
 */
function buildErrorResult(type: "sql" | "kv", message: string): QueryResult {
  if (type === "sql") {
    return {
      type: "sql",
      columns: [],
      rows: [],
      rowCount: 0,
      truncated: false,
      error: message,
    };
  }

  return {
    type: "kv",
    results: [],
    error: message,
  };
}

export type { QueryResult, SqlResult, KeyValueResult, KvResult };