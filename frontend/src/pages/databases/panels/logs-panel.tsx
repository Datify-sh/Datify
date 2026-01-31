import { Button } from "@/components/ui/button";
import { Spinner } from "@/components/ui/spinner";
import { useLogStream } from "@/hooks/use-log-stream";
import { useVirtualizer } from "@tanstack/react-virtual";
import { format } from "date-fns";
import * as React from "react";

const LogsPanel = React.memo(function LogsPanel({ databaseId }: { databaseId: string }) {
  const [streaming, setStreaming] = React.useState(true);
  const scrollContainerRef = React.useRef<HTMLDivElement>(null);

  const { entries, isConnected, isConnecting, clear } = useLogStream(databaseId, {
    tail: 200,
    enabled: streaming,
  });

  const virtualizer = useVirtualizer({
    count: entries.length,
    getScrollElement: () => scrollContainerRef.current,
    estimateSize: () => 24,
    overscan: 10,
  });

  const prevEntriesLengthRef = React.useRef(entries.length);
  React.useEffect(() => {
    if (streaming && entries.length > prevEntriesLengthRef.current && scrollContainerRef.current) {
      virtualizer.scrollToIndex(entries.length - 1, { align: "end" });
    }
    prevEntriesLengthRef.current = entries.length;
  }, [entries.length, streaming, virtualizer]);

  const toggleStreaming = React.useCallback(() => setStreaming((s) => !s), []);

  return (
    <div className="space-y-4">
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-2">
          <div className="flex items-center gap-2">
            <div
              className={`size-2 rounded-full ${isConnected ? "bg-green-500" : isConnecting ? "bg-yellow-500 animate-pulse" : "bg-neutral-500"}`}
            />
            <span className="text-sm text-muted-foreground">
              {isConnected ? "Live" : isConnecting ? "Connecting..." : "Disconnected"}
            </span>
          </div>
          <Button variant={streaming ? "default" : "outline"} size="sm" onClick={toggleStreaming}>
            {streaming ? "Pause" : "Resume"}
          </Button>
          <Button variant="outline" size="sm" onClick={clear}>
            Clear
          </Button>
        </div>
        <span className="text-sm text-muted-foreground">{entries.length} entries</span>
      </div>

      <div
        ref={scrollContainerRef}
        className="h-[500px] overflow-auto rounded-lg border bg-[#0a0a0a] font-mono text-sm"
      >
        {isConnecting && entries.length === 0 ? (
          <div className="flex h-full items-center justify-center">
            <Spinner className="size-6" />
          </div>
        ) : entries.length === 0 ? (
          <div className="flex h-full items-center justify-center text-muted-foreground">
            No logs available
          </div>
        ) : (
          <div className="relative w-full" style={{ height: `${virtualizer.getTotalSize()}px` }}>
            {virtualizer.getVirtualItems().map((virtualRow) => {
              const entry = entries[virtualRow.index];
              return (
                <div
                  key={virtualRow.key}
                  ref={virtualizer.measureElement}
                  data-index={virtualRow.index}
                  className="absolute left-0 w-full flex gap-3 text-xs leading-relaxed hover:bg-white/5 px-4 py-0.5"
                  style={{
                    transform: `translateY(${virtualRow.start}px)`,
                  }}
                >
                  {entry.timestamp && (
                    <span className="text-neutral-500 shrink-0 tabular-nums">
                      {format(new Date(entry.timestamp), "HH:mm:ss")}
                    </span>
                  )}
                  <span className={entry.stream === "stderr" ? "text-red-400" : "text-neutral-300"}>
                    {entry.message}
                  </span>
                </div>
              );
            })}
          </div>
        )}
      </div>
    </div>
  );
});

export default LogsPanel;
