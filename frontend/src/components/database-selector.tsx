import { Database01Icon, Folder01Icon, GitBranchIcon } from "@hugeicons/core-free-icons";
import { HugeiconsIcon } from "@hugeicons/react";
import { useQuery } from "@tanstack/react-query";
import * as React from "react";

import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import {
  Command,
  CommandEmpty,
  CommandGroup,
  CommandInput,
  CommandItem,
  CommandList,
  CommandSeparator,
} from "@/components/ui/command";
import { Popover, PopoverContent, PopoverTrigger } from "@/components/ui/popover";
import { Skeleton } from "@/components/ui/skeleton";
import { databasesApi, projectsApi } from "@/lib/api";
import type { DatabaseResponse } from "@/lib/api/types";
import { cn } from "@/lib/utils";

interface DatabaseSelectorProps {
  selectedProjectId: string | null;
  selectedDatabaseId: string | null;
  onProjectChange: (projectId: string | null) => void;
  onDatabaseChange: (databaseId: string | null) => void;
  requireRunning?: boolean;
  className?: string;
}

const statusColors: Record<string, string> = {
  running: "bg-green-500",
  stopped: "bg-neutral-400",
  starting: "bg-yellow-500 animate-pulse",
  stopping: "bg-yellow-500 animate-pulse",
  error: "bg-red-500",
};

export const DatabaseSelector = React.memo(function DatabaseSelector({
  selectedProjectId,
  selectedDatabaseId,
  onProjectChange,
  onDatabaseChange,
  requireRunning = false,
  className,
}: DatabaseSelectorProps) {
  const [projectOpen, setProjectOpen] = React.useState(false);
  const [databaseOpen, setDatabaseOpen] = React.useState(false);

  // Fetch projects
  const { data: projectsData, isLoading: projectsLoading } = useQuery({
    queryKey: ["projects"],
    queryFn: () => projectsApi.list({ limit: 100 }),
    staleTime: 60000,
  });

  const projects = projectsData?.data || [];

  // Fetch databases for selected project
  const { data: databasesData, isLoading: databasesLoading } = useQuery({
    queryKey: ["databases", selectedProjectId],
    queryFn: () => {
      if (!selectedProjectId) throw new Error("No project selected");
      return databasesApi.list(selectedProjectId, { limit: 100 });
    },
    enabled: !!selectedProjectId,
    staleTime: 30000,
    refetchInterval: 10000,
  });

  const databases = databasesData?.data || [];
  const selectedProject = projects.find((p) => p.id === selectedProjectId);
  const selectedDatabase = databases.find((d) => d.id === selectedDatabaseId);

  // Group databases by main/branches
  const { mainDatabases, branchesByParent } = React.useMemo(() => {
    const main = databases.filter((db) => !db.branch?.parent_id);
    const branches = new Map<string, DatabaseResponse[]>();
    for (const db of databases) {
      if (db.branch?.parent_id) {
        const existing = branches.get(db.branch.parent_id) || [];
        branches.set(db.branch.parent_id, [...existing, db]);
      }
    }
    return { mainDatabases: main, branchesByParent: branches };
  }, [databases]);

  const handleProjectSelect = React.useCallback(
    (projectId: string) => {
      onProjectChange(projectId);
      onDatabaseChange(null);
      setProjectOpen(false);
    },
    [onProjectChange, onDatabaseChange],
  );

  const handleDatabaseSelect = React.useCallback(
    (databaseId: string) => {
      onDatabaseChange(databaseId);
      setDatabaseOpen(false);
    },
    [onDatabaseChange],
  );

  return (
    <div className={cn("flex items-center gap-3", className)}>
      {/* Project Selector */}
      <Popover open={projectOpen} onOpenChange={setProjectOpen}>
        <PopoverTrigger asChild>
          <Button variant="outline" className="w-[220px] justify-between">
            {projectsLoading ? (
              <Skeleton className="h-4 w-24" />
            ) : selectedProject ? (
              <span className="flex items-center gap-2 truncate">
                <HugeiconsIcon icon={Folder01Icon} className="size-4 shrink-0" strokeWidth={2} />
                <span className="truncate">{selectedProject.name}</span>
              </span>
            ) : (
              <span className="text-muted-foreground">Select project...</span>
            )}
          </Button>
        </PopoverTrigger>
        <PopoverContent className="w-[280px] p-0" align="start">
          <Command>
            <CommandInput placeholder="Search projects..." />
            <CommandList>
              <CommandEmpty>No projects found.</CommandEmpty>
              <CommandGroup heading="Projects">
                {projects.map((project) => (
                  <CommandItem
                    key={project.id}
                    value={project.name}
                    onSelect={() => handleProjectSelect(project.id)}
                    data-checked={project.id === selectedProjectId}
                  >
                    <HugeiconsIcon
                      icon={Folder01Icon}
                      className="size-4 text-muted-foreground"
                      strokeWidth={2}
                    />
                    <span className="flex-1 truncate">{project.name}</span>
                    <Badge variant="outline" className="text-[10px]">
                      {project.database_count}
                    </Badge>
                  </CommandItem>
                ))}
              </CommandGroup>
            </CommandList>
          </Command>
        </PopoverContent>
      </Popover>

      <span className="text-muted-foreground">/</span>

      {/* Database Selector */}
      <Popover open={databaseOpen} onOpenChange={setDatabaseOpen}>
        <PopoverTrigger asChild>
          <Button
            variant="outline"
            className="w-[260px] justify-between"
            disabled={!selectedProjectId}
          >
            {databasesLoading ? (
              <Skeleton className="h-4 w-24" />
            ) : selectedDatabase ? (
              <span className="flex items-center gap-2 truncate">
                <span
                  className={cn(
                    "size-2 rounded-full shrink-0",
                    statusColors[selectedDatabase.status],
                  )}
                />
                {selectedDatabase.branch?.parent_id && (
                  <HugeiconsIcon
                    icon={GitBranchIcon}
                    className="size-3.5 text-muted-foreground shrink-0"
                    strokeWidth={2}
                  />
                )}
                <span className="truncate font-mono text-sm">
                  {selectedDatabase.branch?.name || selectedDatabase.name}
                </span>
              </span>
            ) : (
              <span className="text-muted-foreground">
                {selectedProjectId ? "Select database..." : "Select project first"}
              </span>
            )}
          </Button>
        </PopoverTrigger>
        <PopoverContent className="w-[320px] p-0" align="start">
          <Command>
            <CommandInput placeholder="Search databases..." />
            <CommandList>
              <CommandEmpty>No databases found.</CommandEmpty>
              {mainDatabases.map((db) => {
                const branches = branchesByParent.get(db.id) || [];
                const isDisabled = requireRunning && db.status !== "running";

                return (
                  <React.Fragment key={db.id}>
                    <CommandGroup heading={db.branch?.name || db.name}>
                      <CommandItem
                        value={db.branch?.name || db.name}
                        onSelect={() => !isDisabled && handleDatabaseSelect(db.id)}
                        data-checked={db.id === selectedDatabaseId}
                        disabled={isDisabled}
                        className={cn(isDisabled && "opacity-50")}
                      >
                        <span className={cn("size-2 rounded-full", statusColors[db.status])} />
                        <HugeiconsIcon
                          icon={Database01Icon}
                          className="size-4 text-muted-foreground"
                          strokeWidth={2}
                        />
                        <span className="flex-1 truncate font-mono text-sm">
                          {db.branch?.name || db.name}
                        </span>
                        <Badge variant="outline" className="text-[10px]">
                          main
                        </Badge>
                        {isDisabled && (
                          <Badge variant="secondary" className="text-[10px]">
                            stopped
                          </Badge>
                        )}
                      </CommandItem>
                      {branches.map((branch) => {
                        const branchDisabled = requireRunning && branch.status !== "running";
                        return (
                          <CommandItem
                            key={branch.id}
                            value={branch.branch?.name || branch.name}
                            onSelect={() => !branchDisabled && handleDatabaseSelect(branch.id)}
                            data-checked={branch.id === selectedDatabaseId}
                            disabled={branchDisabled}
                            className={cn("pl-6", branchDisabled && "opacity-50")}
                          >
                            <span
                              className={cn("size-2 rounded-full", statusColors[branch.status])}
                            />
                            <HugeiconsIcon
                              icon={GitBranchIcon}
                              className="size-3.5 text-muted-foreground"
                              strokeWidth={2}
                            />
                            <span className="flex-1 truncate font-mono text-sm">
                              {branch.branch?.name || branch.name}
                            </span>
                            {branchDisabled && (
                              <Badge variant="secondary" className="text-[10px]">
                                stopped
                              </Badge>
                            )}
                          </CommandItem>
                        );
                      })}
                    </CommandGroup>
                    <CommandSeparator />
                  </React.Fragment>
                );
              })}
            </CommandList>
          </Command>
        </PopoverContent>
      </Popover>
    </div>
  );
});
