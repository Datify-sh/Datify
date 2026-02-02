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
