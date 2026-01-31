import { Button } from "@/components/ui/button";
import { Card, CardContent } from "@/components/ui/card";
import { Spinner } from "@/components/ui/spinner";
import { useDatabaseDetailContext } from "@/pages/databases/detail-context";
import TerminalPanel from "@/pages/databases/panels/terminal-panel";
import { CommandLineIcon, PlayIcon } from "@hugeicons/core-free-icons";
import { HugeiconsIcon } from "@hugeicons/react";

export function DatabaseTerminalTab() {
  const {
    database,
    isRunning,
    isValkey,
    isRedis,
    isKeyValue,
    terminalType,
    setTerminalType,
    startDatabase,
    startDatabasePending,
  } = useDatabaseDetailContext();

  if (!isRunning) {
    return (
      <Card>
        <CardContent className="flex flex-col items-center justify-center py-12">
          <HugeiconsIcon
            icon={CommandLineIcon}
            className="size-12 text-muted-foreground"
            strokeWidth={1.5}
          />
          <p className="mt-4 text-muted-foreground">Start the database to access the terminal</p>
          <Button className="mt-4" onClick={startDatabase} disabled={startDatabasePending}>
            {startDatabasePending ? (
              <Spinner className="size-4" />
            ) : (
              <HugeiconsIcon icon={PlayIcon} className="size-4" strokeWidth={2} />
            )}
            Start Database
          </Button>
        </CardContent>
      </Card>
    );
  }

  return (
    <div className="space-y-4">
      <div className="flex items-center gap-2">
        {isValkey && (
          <Button
            variant={terminalType === "valkey-cli" ? "default" : "outline"}
            size="sm"
            onClick={() => setTerminalType("valkey-cli")}
          >
            Valkey CLI
          </Button>
        )}
        {isRedis && (
          <Button
            variant={terminalType === "redis-cli" ? "default" : "outline"}
            size="sm"
            onClick={() => setTerminalType("redis-cli")}
          >
            Redis CLI
          </Button>
        )}
        {!isKeyValue && (
          <Button
            variant={terminalType === "psql" ? "default" : "outline"}
            size="sm"
            onClick={() => setTerminalType("psql")}
          >
            PSQL
          </Button>
        )}
        <Button
          variant={terminalType === "shell" ? "default" : "outline"}
          size="sm"
          onClick={() => setTerminalType("shell")}
        >
          Shell
        </Button>
      </div>
      <TerminalPanel
        databaseId={database.id}
        type={
          isValkey && terminalType === "psql"
            ? "valkey-cli"
            : isRedis && terminalType === "psql"
              ? "redis-cli"
              : terminalType
        }
      />
    </div>
  );
}
