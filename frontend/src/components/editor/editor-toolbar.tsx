import { Button } from "@/components/ui/button";
import { Separator } from "@/components/ui/separator";
import { Tooltip, TooltipContent, TooltipTrigger } from "@/components/ui/tooltip";
import type { DatabaseType } from "@/lib/api/types";
import { CommandIcon, Database01Icon, PlayIcon } from "@hugeicons/core-free-icons";
import { HugeiconsIcon } from "@hugeicons/react";

interface EditorToolbarProps {
  databaseType: DatabaseType;
  onRun: () => void;
  isExecuting: boolean;
  hasContent: boolean;
}

export function EditorToolbar({
  databaseType,
  onRun,
  isExecuting,
  hasContent,
}: EditorToolbarProps) {
  const isPostgres = databaseType === "postgres";

  return (
    <div className="flex items-center justify-between px-3 py-2 border-b bg-muted/20">
      <div className="flex items-center gap-2">
        <Tooltip>
          <TooltipTrigger asChild>
            <Button
              size="sm"
              onClick={onRun}
              disabled={isExecuting || !hasContent}
              className="h-7 gap-1.5"
            >
              {isExecuting ? (
                <div className="size-3.5 border-2 border-current border-t-transparent rounded-full animate-spin" />
              ) : (
                <HugeiconsIcon icon={PlayIcon} className="size-3.5" strokeWidth={2} />
              )}
              Run
              <span className="text-[10px] opacity-70 font-normal hidden sm:inline">
                ⌘/Ctrl+Enter
              </span>
            </Button>
          </TooltipTrigger>
          <TooltipContent>{isPostgres ? "Execute SQL query" : "Execute command"}</TooltipContent>
        </Tooltip>

        <Separator orientation="vertical" className="h-5" />

        <div className="flex items-center gap-1.5 text-xs text-muted-foreground">
          <HugeiconsIcon
            icon={isPostgres ? Database01Icon : CommandIcon}
            className="size-3.5"
            strokeWidth={2}
          />
          <span className="capitalize">{databaseType}</span>
          <span className="text-[10px] opacity-60">
            {isPostgres ? "SQL Editor" : "Command Interface"}
          </span>
        </div>
      </div>

      <div className="flex items-center gap-2">
        {isPostgres ? (
          <div className="hidden sm:flex items-center gap-3 text-[10px] text-muted-foreground">
            <span className="flex items-center gap-1">
              <kbd className="px-1.5 py-0.5 bg-muted rounded border text-[9px]">SELECT</kbd>
              <kbd className="px-1.5 py-0.5 bg-muted rounded border text-[9px]">INSERT</kbd>
              <kbd className="px-1.5 py-0.5 bg-muted rounded border text-[9px]">UPDATE</kbd>
              <span>supported</span>
            </span>
          </div>
        ) : (
          <div className="hidden sm:flex items-center gap-3 text-[10px] text-muted-foreground">
            <span className="flex items-center gap-1">
              <kbd className="px-1.5 py-0.5 bg-muted rounded border text-[9px]">GET</kbd>
              <kbd className="px-1.5 py-0.5 bg-muted rounded border text-[9px]">SET</kbd>
              <kbd className="px-1.5 py-0.5 bg-muted rounded border text-[9px]">DEL</kbd>
              <span>supported</span>
            </span>
          </div>
        )}
      </div>
    </div>
  );
}
