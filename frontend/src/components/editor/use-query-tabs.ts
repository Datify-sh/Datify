import type { DatabaseType } from "@/lib/api/types";
import * as React from "react";
import type { QueryResult, QueryTab } from "./types";

const getDefaultQuery = (databaseType: DatabaseType): string => {
  if (databaseType === "postgres") {
    return `-- Write your SQL query here
SELECT * FROM information_schema.tables 
WHERE table_schema = 'public'
LIMIT 10;`;
  }
  if (databaseType === "valkey") {
    return `# Valkey commands
INFO server
PING`;
  }
  return `# Redis commands
INFO server
PING`;
};

const createNewTab = (databaseType: DatabaseType, index: number): QueryTab => ({
  id: `tab-${Date.now()}-${index}`,
  title: `Query ${index}`,
  content: getDefaultQuery(databaseType),
  type: databaseType === "postgres" ? "sql" : "kv",
  isExecuting: false,
});

/**
 * Manages a list of query editor tabs and provides actions to add, close, switch, and update tabs.
 *
 * @param databaseType - Database type used to initialize new tabs' default content and tab type (e.g., `"postgres"` for SQL tabs, otherwise KV-style tabs).
 * @returns An object with the current tab state and mutator functions:
 *  - `tabs`: the array of `QueryTab` objects.
 *  - `activeTabId`: the id of the currently active tab.
 *  - `addTab()`: creates a new tab, makes it active, and returns the new tab's id.
 *  - `closeTab(tabId)`: removes the specified tab (no-op if only one tab remains) and updates the active tab if the closed tab was active.
 *  - `setActiveTab(tabId)`: sets the active tab by id.
 *  - `updateTabContent(tabId, content)`: replaces a tab's content and, if `content` is non-empty, updates the tab title derived from the content and tab type.
 *  - `updateTabResult(tabId, result, executionTime?)`: sets a tab's result and optional execution time, and marks the tab as not executing.
 *  - `setTabExecuting(tabId, isExecuting)`: sets the executing flag for the specified tab.
 */
export function useQueryTabs(databaseType: DatabaseType) {
  const [tabs, setTabs] = React.useState<QueryTab[]>(() => [createNewTab(databaseType, 1)]);
  const [activeTabId, setActiveTabId] = React.useState<string>(tabs[0].id);

  const addTab = React.useCallback(() => {
    const newTab = createNewTab(databaseType, tabs.length + 1);
    setTabs((prev) => [...prev, newTab]);
    setActiveTabId(newTab.id);
    return newTab.id;
  }, [databaseType, tabs.length]);

  const closeTab = React.useCallback(
    (tabId: string) => {
      setTabs((prev) => {
        if (prev.length <= 1) return prev;
        const newTabs = prev.filter((t) => t.id !== tabId);
        if (activeTabId === tabId) {
          const index = prev.findIndex((t) => t.id === tabId);
          const newActiveTab = newTabs[Math.min(index, newTabs.length - 1)];
          setActiveTabId(newActiveTab.id);
        }
        return newTabs;
      });
    },
    [activeTabId],
  );

  const setActiveTab = React.useCallback((tabId: string) => {
    setActiveTabId(tabId);
  }, []);

  const updateTabContent = React.useCallback((tabId: string, content: string) => {
    setTabs((prev) =>
      prev.map((tab) =>
        tab.id === tabId
          ? {
              ...tab,
              content,
              title: content.trim() ? getQueryTitle(content, tab.type) : tab.title,
            }
          : tab,
      ),
    );
  }, []);

  const updateTabResult = React.useCallback(
    (tabId: string, result: QueryResult, executionTime?: number) => {
      setTabs((prev) =>
        prev.map((tab) =>
          tab.id === tabId ? { ...tab, result, executionTime, isExecuting: false } : tab,
        ),
      );
    },
    [],
  );

  const setTabExecuting = React.useCallback((tabId: string, isExecuting: boolean) => {
    setTabs((prev) => prev.map((tab) => (tab.id === tabId ? { ...tab, isExecuting } : tab)));
  }, []);

  return {
    tabs,
    activeTabId,
    addTab,
    closeTab,
    setActiveTab,
    updateTabContent,
    updateTabResult,
    setTabExecuting,
  };
}

/**
 * Derives a human-readable tab title from query content for SQL or KV editors.
 *
 * For SQL content, the title will prefer the operation and table name (e.g., "SELECT users"), fall back to the first word of the query, or "Query" if empty. For KV content, the title is the first command token uppercased (e.g., "GET") or "Command" if empty.
 *
 * @param content - The query or command text to derive the title from
 * @param type - The editor type that determines parsing rules: `"sql"` or `"kv"`
 * @returns The derived title string appropriate for a tab label
 */
function getQueryTitle(content: string, type: "sql" | "kv"): string {
  const trimmed = content.trim();
  if (!trimmed) return "Query";

  if (type === "sql") {
    // Try to extract table name from SELECT, INSERT, UPDATE, DELETE
    const match =
      trimmed.match(/SELECT\s+.+?\s+FROM\s+(\w+)/i) ||
      trimmed.match(/INSERT\s+INTO\s+(\w+)/i) ||
      trimmed.match(/UPDATE\s+(\w+)/i) ||
      trimmed.match(/DELETE\s+FROM\s+(\w+)/i) ||
      trimmed.match(/CREATE\s+TABLE\s+(\w+)/i) ||
      trimmed.match(/ALTER\s+TABLE\s+(\w+)/i) ||
      trimmed.match(/DROP\s+TABLE\s+(\w+)/i);

    if (match) {
      const operation = trimmed.match(/^(\w+)/)?.[0] || "Query";
      return `${operation} ${match[1]}`;
    }

    return trimmed.match(/^(\w+)/)?.[0] || "Query";
  }

  // For KV, use the first command
  const firstLine = trimmed.split("\n")[0].trim();
  const command = firstLine.split(/\s+/)[0].toUpperCase();
  return command || "Command";
}