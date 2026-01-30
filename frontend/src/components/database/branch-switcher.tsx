import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import {
  Command,
  CommandDialog,
  CommandEmpty,
  CommandGroup,
  CommandInput,
  CommandItem,
  CommandList,
  CommandSeparator,
} from "@/components/ui/command";
import { Tooltip, TooltipContent, TooltipTrigger } from "@/components/ui/tooltip";
import { type BranchResponse, databasesApi } from "@/lib/api";
import { cn } from "@/lib/utils";
import {
  Add01Icon,
  ArrowRight01Icon,
  CheckmarkCircle02Icon,
  Clock01Icon,
  GitBranchIcon,
} from "@hugeicons/core-free-icons";
import { HugeiconsIcon } from "@hugeicons/react";
import { useQuery } from "@tanstack/react-query";
import { formatDistanceToNow } from "date-fns";
import * as React from "react";
import { useNavigate } from "react-router-dom";

interface BranchSwitcherProps {
  databaseId: string;
  currentBranchName: string;
  onCreateBranch?: () => void;
}

const statusConfig: Record<string, { color: string; label: string }> = {
  running: { color: "bg-green-500", label: "Running" },
  stopped: { color: "bg-neutral-400", label: "Stopped" },
  starting: { color: "bg-yellow-500 animate-pulse", label: "Starting" },
  stopping: { color: "bg-yellow-500 animate-pulse", label: "Stopping" },
  error: { color: "bg-red-500", label: "Error" },
};

export function BranchSwitcher({
  databaseId,
  currentBranchName,
  onCreateBranch,
}: BranchSwitcherProps) {
  const navigate = useNavigate();
  const [open, setOpen] = React.useState(false);

  const { data: branches, isLoading } = useQuery({
    queryKey: ["branches", databaseId],
    queryFn: () => databasesApi.listBranches(databaseId),
  });

  React.useEffect(() => {
    const down = (e: KeyboardEvent) => {
      if (e.key === "b" && (e.metaKey || e.ctrlKey)) {
        e.preventDefault();
        setOpen((open) => !open);
      }
    };
    document.addEventListener("keydown", down);
    return () => document.removeEventListener("keydown", down);
  }, []);

  const handleSwitchBranch = (branch: BranchResponse) => {
    if (branch.id !== databaseId) {
      navigate(`/databases/${branch.id}`);
    }
    setOpen(false);
  };

  const currentBranch = branches?.find((b) => b.id === databaseId);
  const otherBranches = branches?.filter((b) => b.id !== databaseId) ?? [];
  const defaultBranch = branches?.find((b) => b.is_default);
  const childBranches = otherBranches.filter((b) => !b.is_default);

  return (
    <>
      <Tooltip>
        <TooltipTrigger asChild>
          <Button
            variant="outline"
            size="sm"
            onClick={() => setOpen(true)}
            disabled={isLoading}
            className="gap-2 font-mono text-xs"
          >
            <HugeiconsIcon icon={GitBranchIcon} className="size-4" strokeWidth={2} />
            <span className="max-w-[120px] truncate">{currentBranchName}</span>
            {currentBranch && (
              <span
                className={cn(
                  "size-2 rounded-full",
                  statusConfig[currentBranch.status]?.color ?? "bg-neutral-400",
                )}
              />
            )}
          </Button>
        </TooltipTrigger>
        <TooltipContent side="bottom">
          <div className="flex items-center gap-2">
            <span>Switch branch</span>
            <kbd className="bg-background/20 px-1.5 py-0.5 rounded text-[10px]">⌘B</kbd>
          </div>
        </TooltipContent>
      </Tooltip>

      <CommandDialog
        open={open}
        onOpenChange={setOpen}
        title="Switch Branch"
        description="Select a branch to switch to"
      >
        <Command className="rounded-lg border-none">
          <CommandInput placeholder="Search branches..." />
          <CommandList>
            <CommandEmpty>No branches found.</CommandEmpty>

            {currentBranch && (
              <CommandGroup heading="Current Branch">
                <BranchItem
                  branch={currentBranch}
                  isCurrent={true}
                  onSelect={() => setOpen(false)}
                />
              </CommandGroup>
            )}

            {defaultBranch && defaultBranch.id !== databaseId && (
              <CommandGroup heading="Main Branch">
                <BranchItem
                  branch={defaultBranch}
                  isCurrent={false}
                  onSelect={() => handleSwitchBranch(defaultBranch)}
                />
              </CommandGroup>
            )}

            {childBranches.length > 0 && (
              <CommandGroup heading="Other Branches">
                {childBranches.map((branch) => (
                  <BranchItem
                    key={branch.id}
                    branch={branch}
                    isCurrent={false}
                    onSelect={() => handleSwitchBranch(branch)}
                  />
                ))}
              </CommandGroup>
            )}

            {onCreateBranch && (
              <>
                <CommandSeparator />
                <CommandGroup>
                  <CommandItem
                    onSelect={() => {
                      setOpen(false);
                      onCreateBranch();
                    }}
                    className="text-primary"
                  >
                    <HugeiconsIcon icon={Add01Icon} className="size-4" strokeWidth={2} />
                    <span>Create new branch</span>
                    <span className="ml-auto text-xs text-muted-foreground">
                      from {currentBranchName}
                    </span>
                  </CommandItem>
                </CommandGroup>
              </>
            )}
          </CommandList>
        </Command>
      </CommandDialog>
    </>
  );
}

function BranchItem({
  branch,
  isCurrent,
  onSelect,
}: {
  branch: BranchResponse;
  isCurrent: boolean;
  onSelect: () => void;
}) {
  const status = statusConfig[branch.status] ?? { color: "bg-neutral-400", label: "Unknown" };

  return (
    <CommandItem
      onSelect={onSelect}
      className="flex items-center justify-between"
      data-checked={isCurrent}
    >
      <div className="flex items-center gap-3">
        <div className="flex items-center gap-2">
          <HugeiconsIcon
            icon={GitBranchIcon}
            className="size-4 text-muted-foreground"
            strokeWidth={2}
          />
          <span className="font-mono text-sm">{branch.name}</span>
        </div>
        {branch.is_default && (
          <Badge variant="secondary" className="text-[10px] px-1.5 py-0 h-4">
            main
          </Badge>
        )}
      </div>
      <div className="flex items-center gap-3">
        {branch.forked_at && !branch.is_default && (
          <Tooltip>
            <TooltipTrigger className="flex items-center gap-1 text-xs text-muted-foreground">
              <HugeiconsIcon icon={Clock01Icon} className="size-3" strokeWidth={2} />
              {formatDistanceToNow(new Date(branch.forked_at), { addSuffix: true })}
            </TooltipTrigger>
            <TooltipContent>
              Last synced {formatDistanceToNow(new Date(branch.forked_at), { addSuffix: true })}
            </TooltipContent>
          </Tooltip>
        )}
        <div className="flex items-center gap-2">
          <span className={cn("size-2 rounded-full", status.color)} />
          <span className="text-xs text-muted-foreground w-16">{status.label}</span>
        </div>
        {isCurrent ? (
          <HugeiconsIcon
            icon={CheckmarkCircle02Icon}
            className="size-4 text-primary"
            strokeWidth={2}
          />
        ) : (
          <HugeiconsIcon
            icon={ArrowRight01Icon}
            className="size-4 text-muted-foreground opacity-0 group-data-selected:opacity-100"
            strokeWidth={2}
          />
        )}
      </div>
    </CommandItem>
  );
}
