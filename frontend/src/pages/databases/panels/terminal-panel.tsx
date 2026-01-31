import { TerminalComponent, type TerminalRef } from "@/components/terminal";
import { Button } from "@/components/ui/button";
import { databasesApi } from "@/lib/api";
import { RefreshIcon } from "@hugeicons/core-free-icons";
import { HugeiconsIcon } from "@hugeicons/react";
import * as React from "react";

type TerminalType = "shell" | "psql" | "valkey-cli" | "redis-cli";

const TerminalPanel = React.memo(function TerminalPanel({
  databaseId,
  type,
}: { databaseId: string; type: TerminalType }) {
  const [isConnected, setIsConnected] = React.useState(false);
  const terminalRef = React.useRef<TerminalRef>(null);

  const wsUrl = React.useMemo(() => {
    if (type === "psql") return databasesApi.getPsqlUrl(databaseId);
    if (type === "valkey-cli") return databasesApi.getValkeyCliUrl(databaseId);
    if (type === "redis-cli") return databasesApi.getRedisCliUrl(databaseId);
    return databasesApi.getTerminalUrl(databaseId);
  }, [type, databaseId]);

  const handleReconnect = React.useCallback(() => {
    terminalRef.current?.reconnect();
  }, []);
  const handleConnected = React.useCallback(() => setIsConnected(true), []);
  const handleDisconnected = React.useCallback(() => setIsConnected(false), []);

  return (
    <div className="space-y-4">
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-2">
          <div
            className={`size-2 rounded-full ${isConnected ? "bg-green-500" : "bg-neutral-500"}`}
          />
          <span className="text-sm text-muted-foreground">
            {isConnected ? "Connected" : "Disconnected"}
          </span>
        </div>
        <Button variant="outline" size="sm" onClick={handleReconnect}>
          <HugeiconsIcon icon={RefreshIcon} className="size-4" strokeWidth={2} />
          Reconnect
        </Button>
      </div>

      <div className="h-[500px] rounded-lg border overflow-hidden">
        <TerminalComponent
          ref={terminalRef}
          wsUrl={wsUrl}
          onConnected={handleConnected}
          onDisconnected={handleDisconnected}
        />
      </div>
    </div>
  );
});

export default TerminalPanel;
