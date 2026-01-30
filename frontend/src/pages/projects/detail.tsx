import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import {
  Dialog,
  DialogClose,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import { Field, FieldDescription, FieldLabel } from "@/components/ui/field";
import { Input } from "@/components/ui/input";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { Skeleton } from "@/components/ui/skeleton";
import { Spinner } from "@/components/ui/spinner";
import { Switch } from "@/components/ui/switch";
import {
  type DatabaseResponse,
  type DatabaseType,
  databasesApi,
  projectsApi,
  systemApi,
} from "@/lib/api";
import { cn } from "@/lib/utils";
import {
  Add01Icon,
  ArrowRight01Icon,
  Copy01Icon,
  Database01Icon,
  Delete01Icon,
  GitBranchIcon,
  Globe02Icon,
  MoreHorizontalIcon,
  PauseIcon,
  PlayIcon,
  Settings01Icon,
  SquareLock02Icon,
} from "@hugeicons/core-free-icons";
import { HugeiconsIcon } from "@hugeicons/react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { format } from "date-fns";
import * as React from "react";
import { Link, useParams } from "react-router-dom";
import { toast } from "sonner";

const statusDots: Record<string, string> = {
  running: "bg-green-500",
  stopped: "bg-neutral-400",
  starting: "bg-yellow-500 animate-pulse",
  stopping: "bg-yellow-500 animate-pulse",
  error: "bg-red-500",
};

function DatabaseRow({
  database,
  isChild = false,
}: { database: DatabaseResponse; isChild?: boolean }) {
  const queryClient = useQueryClient();

  const startMutation = useMutation({
    mutationFn: () => databasesApi.start(database.id),
    onMutate: async () => {
      await queryClient.cancelQueries({ queryKey: ["databases", database.project_id] });
      queryClient.setQueryData(
        ["databases", database.project_id],
        (old: { data: DatabaseResponse[] } | undefined) => {
          if (!old) return old;
          return {
            ...old,
            data: old.data.map((db) =>
              db.id === database.id ? { ...db, status: "starting" } : db,
            ),
          };
        },
      );
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["databases", database.project_id] });
      queryClient.invalidateQueries({ queryKey: ["all-databases-for-stats"] });
      toast.success("Database started");
    },
    onError: () => {
      queryClient.invalidateQueries({ queryKey: ["databases", database.project_id] });
      toast.error("Failed to start database");
    },
  });

  const stopMutation = useMutation({
    mutationFn: () => databasesApi.stop(database.id),
    onMutate: async () => {
      await queryClient.cancelQueries({ queryKey: ["databases", database.project_id] });
      queryClient.setQueryData(
        ["databases", database.project_id],
        (old: { data: DatabaseResponse[] } | undefined) => {
          if (!old) return old;
          return {
            ...old,
            data: old.data.map((db) =>
              db.id === database.id ? { ...db, status: "stopping" } : db,
            ),
          };
        },
      );
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["databases", database.project_id] });
      queryClient.invalidateQueries({ queryKey: ["all-databases-for-stats"] });
      toast.success("Database stopped");
    },
    onError: () => {
      queryClient.invalidateQueries({ queryKey: ["databases", database.project_id] });
      toast.error("Failed to stop database");
    },
  });

  const deleteMutation = useMutation({
    mutationFn: () => databasesApi.delete(database.id),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["databases", database.project_id] });
      queryClient.invalidateQueries({ queryKey: ["projects"] });
      queryClient.invalidateQueries({ queryKey: ["all-databases-for-stats"] });
      toast.success("Database deleted");
    },
    onError: () => toast.error("Failed to delete database"),
  });

  const copyConnectionString = (e: React.MouseEvent) => {
    e.preventDefault();
    e.stopPropagation();
    if (database.connection?.connection_string) {
      navigator.clipboard.writeText(database.connection.connection_string);
      toast.success("Connection string copied");
    }
  };

  const isLoading = startMutation.isPending || stopMutation.isPending;
  const isRunning = database.status === "running";

  return (
    <Link
      to={`/databases/${database.id}`}
      className={cn(
        "group flex items-center justify-between gap-4 rounded-lg border p-3 transition-colors hover:bg-muted/50",
        isChild && "ml-6 border-l-2 border-l-muted-foreground/20",
      )}
    >
      <div className="flex items-center gap-3 min-w-0">
        <div
          className={cn(
            "flex size-8 shrink-0 items-center justify-center rounded-md",
            isChild ? "bg-muted" : "bg-primary/10",
          )}
        >
          <HugeiconsIcon
            icon={isChild ? GitBranchIcon : Database01Icon}
            className={cn("size-4", isChild ? "text-muted-foreground" : "text-primary")}
            strokeWidth={2}
          />
        </div>
        <div className="min-w-0">
          <div className="flex items-center gap-2">
            <span className="font-medium text-sm truncate font-mono">{database.name}</span>
            {database.branch?.is_default && (
              <Badge variant="secondary" className="text-[10px] px-1.5 py-0 h-4">
                main
              </Badge>
            )}
            {isChild && database.branch && (
              <Badge variant="outline" className="text-[10px] px-1.5 py-0 h-4 font-mono">
                {database.branch.name}
              </Badge>
            )}
          </div>
          <div className="flex items-center gap-2 text-xs text-muted-foreground">
            <span className={cn("size-1.5 rounded-full", statusDots[database.status])} />
            <span>{database.status}</span>
            <span>·</span>
            <span className="font-mono">
              {database.database_type === "valkey"
                ? `Valkey ${database.valkey_version}`
                : database.database_type === "redis"
                  ? `Redis ${database.redis_version}`
                  : `PostgreSQL ${database.postgres_version}`}
            </span>
          </div>
        </div>
      </div>

      <div className="flex items-center gap-2 shrink-0">
        <Badge variant="outline" className="text-[10px] px-1.5 py-0 h-5 hidden sm:flex">
          <HugeiconsIcon
            icon={database.public_exposed ? Globe02Icon : SquareLock02Icon}
            className="size-3 mr-1"
            strokeWidth={2}
          />
          {database.public_exposed ? "Public" : "Internal"}
        </Badge>

        <DropdownMenu>
          <DropdownMenuTrigger asChild>
            <Button
              variant="ghost"
              size="icon-sm"
              className="opacity-0 group-hover:opacity-100"
              onClick={(e) => e.preventDefault()}
            >
              <HugeiconsIcon icon={MoreHorizontalIcon} className="size-4" strokeWidth={2} />
            </Button>
          </DropdownMenuTrigger>
          <DropdownMenuContent align="end" onClick={(e) => e.stopPropagation()}>
            {isRunning ? (
              <DropdownMenuItem
                onClick={(e) => {
                  e.preventDefault();
                  stopMutation.mutate();
                }}
                disabled={isLoading}
              >
                <HugeiconsIcon icon={PauseIcon} className="size-3.5" strokeWidth={2} />
                Stop
              </DropdownMenuItem>
            ) : (
              <DropdownMenuItem
                onClick={(e) => {
                  e.preventDefault();
                  startMutation.mutate();
                }}
                disabled={isLoading}
              >
                <HugeiconsIcon icon={PlayIcon} className="size-3.5" strokeWidth={2} />
                Start
              </DropdownMenuItem>
            )}
            {database.connection && (
              <DropdownMenuItem onClick={copyConnectionString}>
                <HugeiconsIcon icon={Copy01Icon} className="size-3.5" strokeWidth={2} />
                Copy connection string
              </DropdownMenuItem>
            )}
            <DropdownMenuSeparator />
            <DropdownMenuItem
              onClick={(e) => {
                e.preventDefault();
                deleteMutation.mutate();
              }}
              className="text-destructive focus:text-destructive"
              disabled={deleteMutation.isPending}
            >
              <HugeiconsIcon icon={Delete01Icon} className="size-3.5" strokeWidth={2} />
              Delete
            </DropdownMenuItem>
          </DropdownMenuContent>
        </DropdownMenu>

        <HugeiconsIcon
          icon={ArrowRight01Icon}
          className="size-4 text-muted-foreground group-hover:text-foreground transition-colors"
          strokeWidth={2}
        />
      </div>
    </Link>
  );
}

function DatabaseGroup({
  main,
  branches,
}: {
  main: DatabaseResponse;
  branches: DatabaseResponse[];
}) {
  return (
    <div className="space-y-2">
      <DatabaseRow database={main} />
      {branches.map((branch) => (
        <DatabaseRow key={branch.id} database={branch} isChild />
      ))}
    </div>
  );
}

export function ProjectDetailPage() {
  const { id } = useParams<{ id: string }>();
  const queryClient = useQueryClient();
  const [isCreateOpen, setIsCreateOpen] = React.useState(false);
  const [publicExposed, setPublicExposed] = React.useState(false);
  const [useCustomPassword, setUseCustomPassword] = React.useState(false);
  const [databaseType, setDatabaseType] = React.useState<DatabaseType>("postgres");

  const { data: project, isLoading: projectLoading } = useQuery({
    queryKey: ["project", id],
    queryFn: () => (id ? projectsApi.get(id) : Promise.reject()),
    enabled: !!id,
  });

  const { data: databasesData, isLoading: databasesLoading } = useQuery({
    queryKey: ["databases", id],
    queryFn: () => (id ? databasesApi.list(id) : Promise.reject()),
    enabled: !!id,
    refetchInterval: 5000, // Keep database statuses fresh
  });

  const { data: postgresVersionsData } = useQuery({
    queryKey: ["postgres-versions"],
    queryFn: () => systemApi.getPostgresVersions(),
    staleTime: 1000 * 60 * 60,
  });

  const { data: valkeyVersionsData } = useQuery({
    queryKey: ["valkey-versions"],
    queryFn: () => systemApi.getValkeyVersions(),
    staleTime: 1000 * 60 * 60,
  });

  const { data: redisVersionsData } = useQuery({
    queryKey: ["redis-versions"],
    queryFn: () => systemApi.getRedisVersions(),
    staleTime: 1000 * 60 * 60,
  });

  const postgresVersions = postgresVersionsData?.versions || [];
  const defaultPostgresVersion = postgresVersionsData?.default_version || "16";
  const valkeyVersions = valkeyVersionsData?.versions || [];
  const defaultValkeyVersion = valkeyVersionsData?.default_version || "8.0";
  const redisVersions = redisVersionsData?.versions || [];
  const defaultRedisVersion = redisVersionsData?.default_version || "7.4";

  const createMutation = useMutation({
    mutationFn: (data: {
      name: string;
      database_type?: DatabaseType;
      postgres_version?: string;
      valkey_version?: string;
      redis_version?: string;
      password?: string;
      public_exposed?: boolean;
    }) => {
      if (!id) return Promise.reject(new Error("Project ID is required"));
      return databasesApi.create(id, data);
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["databases", id] });
      queryClient.invalidateQueries({ queryKey: ["project", id] });
      queryClient.invalidateQueries({ queryKey: ["projects"] });
      queryClient.invalidateQueries({ queryKey: ["all-databases-for-stats"] });
      setIsCreateOpen(false);
      setPublicExposed(false);
      setUseCustomPassword(false);
      setDatabaseType("postgres");
      toast.success("Database created");
    },
    onError: () => toast.error("Failed to create database"),
  });

  const handleCreateDatabase = (e: React.FormEvent<HTMLFormElement>) => {
    e.preventDefault();
    const formData = new FormData(e.currentTarget);
    const password = useCustomPassword ? (formData.get("password") as string) : undefined;
    createMutation.mutate({
      name: formData.get("name") as string,
      database_type: databaseType,
      postgres_version:
        databaseType === "postgres" ? (formData.get("postgres_version") as string) : undefined,
      valkey_version:
        databaseType === "valkey" ? (formData.get("valkey_version") as string) : undefined,
      redis_version:
        databaseType === "redis" ? (formData.get("redis_version") as string) : undefined,
      password: password || undefined,
      public_exposed: publicExposed,
    });
  };

  const databases = databasesData?.data || [];
  const runningCount = databases.filter((d) => d.status === "running").length;

  const groupedDatabases = React.useMemo(() => {
    const mainDatabases = databases.filter((d) => d.branch?.is_default || !d.branch?.parent_id);
    const branchDatabases = databases.filter((d) => d.branch?.parent_id && !d.branch?.is_default);

    return mainDatabases.map((main) => ({
      main,
      branches: branchDatabases
        .filter((b) => b.branch?.parent_id === main.id)
        .sort((a, b) => new Date(a.created_at).getTime() - new Date(b.created_at).getTime()),
    }));
  }, [databases]);

  if (projectLoading) {
    return (
      <div className="space-y-6">
        <Skeleton className="h-8 w-48" />
        <Skeleton className="h-4 w-96" />
        <div className="grid gap-4 sm:grid-cols-3">
          <Skeleton className="h-24" />
          <Skeleton className="h-24" />
          <Skeleton className="h-24" />
        </div>
      </div>
    );
  }

  if (!project) {
    return (
      <div className="py-16 text-center">
        <p className="text-muted-foreground">Project not found</p>
        <Button className="mt-4" variant="outline" asChild>
          <Link to="/projects">Back to projects</Link>
        </Button>
      </div>
    );
  }

  const totalBranches = databases.filter(
    (d) => d.branch?.parent_id && !d.branch?.is_default,
  ).length;

  return (
    <div className="space-y-6">
      <div className="flex items-start justify-between">
        <div>
          <h1 className="text-2xl font-bold tracking-tight font-mono">{project.name}</h1>
          <p className="text-muted-foreground mt-1">
            {project.description ||
              `Created ${format(new Date(project.created_at), "MMM d, yyyy 'at' h:mm a")}`}
          </p>
        </div>
        <div className="flex items-center gap-2">
          <Button variant="outline" size="sm">
            <HugeiconsIcon icon={Settings01Icon} className="size-4" strokeWidth={2} />
            Settings
          </Button>
          <Button size="sm" onClick={() => setIsCreateOpen(true)}>
            <HugeiconsIcon icon={Add01Icon} className="size-4" strokeWidth={2} />
            New Database
          </Button>
        </div>
      </div>

      <div className="grid gap-4 sm:grid-cols-3">
        <Card>
          <CardHeader className="pb-2">
            <CardTitle className="text-sm font-medium text-muted-foreground">Databases</CardTitle>
          </CardHeader>
          <CardContent>
            <div className="text-2xl font-bold">{groupedDatabases.length}</div>
            {totalBranches > 0 && (
              <p className="text-xs text-muted-foreground mt-1">
                + {totalBranches} branch{totalBranches !== 1 && "es"}
              </p>
            )}
          </CardContent>
        </Card>
        <Card>
          <CardHeader className="pb-2">
            <CardTitle className="text-sm font-medium text-muted-foreground">Running</CardTitle>
          </CardHeader>
          <CardContent>
            <div className="text-2xl font-bold text-green-600">{runningCount}</div>
          </CardContent>
        </Card>
        <Card>
          <CardHeader className="pb-2">
            <CardTitle className="text-sm font-medium text-muted-foreground">Created</CardTitle>
          </CardHeader>
          <CardContent>
            <div className="text-lg font-semibold">
              {format(new Date(project.created_at), "MMM d, yyyy")}
            </div>
          </CardContent>
        </Card>
      </div>

      <div className="space-y-4">
        <div className="flex items-center justify-between">
          <div>
            <h2 className="text-lg font-semibold">Databases</h2>
            <p className="text-sm text-muted-foreground">Manage your database instances</p>
          </div>
          <Button variant="outline" size="sm" onClick={() => setIsCreateOpen(true)}>
            <HugeiconsIcon icon={Add01Icon} className="size-4" strokeWidth={2} />
            Add Database
          </Button>
        </div>

        {databasesLoading ? (
          <div className="space-y-3">
            {[1, 2].map((i) => (
              <Skeleton key={i} className="h-16" />
            ))}
          </div>
        ) : databases.length === 0 ? (
          <Card>
            <CardContent className="flex flex-col items-center justify-center py-12">
              <div className="flex size-12 items-center justify-center rounded-full bg-muted">
                <HugeiconsIcon
                  icon={Database01Icon}
                  className="size-6 text-muted-foreground"
                  strokeWidth={2}
                />
              </div>
              <h3 className="mt-4 text-lg font-semibold">No databases yet</h3>
              <p className="mt-1 text-sm text-muted-foreground text-center max-w-sm">
                Create your first database to get started. You can create branches for development
                and testing.
              </p>
              <Button onClick={() => setIsCreateOpen(true)} className="mt-4">
                <HugeiconsIcon icon={Add01Icon} className="size-4" strokeWidth={2} />
                Create Database
              </Button>
            </CardContent>
          </Card>
        ) : (
          <div className="space-y-4">
            {groupedDatabases.map(({ main, branches }) => (
              <DatabaseGroup key={main.id} main={main} branches={branches} />
            ))}
          </div>
        )}
      </div>

      <Dialog open={isCreateOpen} onOpenChange={setIsCreateOpen}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Create Database</DialogTitle>
            <DialogDescription>Create a new database instance in this project</DialogDescription>
          </DialogHeader>
          <form onSubmit={handleCreateDatabase}>
            <div className="space-y-4 py-4">
              <Field>
                <FieldLabel>Database Name</FieldLabel>
                <Input name="name" placeholder="my-database" required autoFocus />
                <FieldDescription>A unique name for this database instance</FieldDescription>
              </Field>
              <Field>
                <FieldLabel>Database Type</FieldLabel>
                <Select
                  value={databaseType}
                  onValueChange={(v) => setDatabaseType(v as DatabaseType)}
                >
                  <SelectTrigger>
                    <SelectValue />
                  </SelectTrigger>
                  <SelectContent>
                    <SelectItem value="postgres">PostgreSQL</SelectItem>
                    <SelectItem value="valkey">Valkey</SelectItem>
                    <SelectItem value="redis">Redis</SelectItem>
                  </SelectContent>
                </Select>
              </Field>
              {databaseType === "postgres" && (
                <Field>
                  <FieldLabel>PostgreSQL Version</FieldLabel>
                  <Select name="postgres_version" defaultValue={defaultPostgresVersion}>
                    <SelectTrigger>
                      <SelectValue />
                    </SelectTrigger>
                    <SelectContent>
                      {postgresVersions.map((v) => (
                        <SelectItem key={v.version} value={v.version}>
                          PostgreSQL {v.version}
                          {v.is_latest ? " (Latest)" : ""}
                        </SelectItem>
                      ))}
                    </SelectContent>
                  </Select>
                </Field>
              )}
              {databaseType === "valkey" && (
                <Field>
                  <FieldLabel>Valkey Version</FieldLabel>
                  <Select name="valkey_version" defaultValue={defaultValkeyVersion}>
                    <SelectTrigger>
                      <SelectValue />
                    </SelectTrigger>
                    <SelectContent>
                      {valkeyVersions.map((v) => (
                        <SelectItem key={v.version} value={v.version}>
                          Valkey {v.version}
                          {v.is_latest ? " (Latest)" : ""}
                        </SelectItem>
                      ))}
                    </SelectContent>
                  </Select>
                </Field>
              )}
              {databaseType === "redis" && (
                <Field>
                  <FieldLabel>Redis Version</FieldLabel>
                  <Select name="redis_version" defaultValue={defaultRedisVersion}>
                    <SelectTrigger>
                      <SelectValue />
                    </SelectTrigger>
                    <SelectContent>
                      {redisVersions.map((v) => (
                        <SelectItem key={v.version} value={v.version}>
                          Redis {v.version}
                          {v.is_latest ? " (Latest)" : ""}
                        </SelectItem>
                      ))}
                    </SelectContent>
                  </Select>
                </Field>
              )}
              <Field>
                <div className="flex items-center justify-between">
                  <div>
                    <FieldLabel>Custom Password</FieldLabel>
                    <FieldDescription>
                      Set your own password instead of generating one
                    </FieldDescription>
                  </div>
                  <Switch checked={useCustomPassword} onCheckedChange={setUseCustomPassword} />
                </div>
              </Field>
              {useCustomPassword && (
                <Field>
                  <FieldLabel>Password</FieldLabel>
                  <Input
                    name="password"
                    type="password"
                    placeholder="Enter password"
                    required
                    minLength={8}
                  />
                  <FieldDescription>Minimum 8 characters</FieldDescription>
                </Field>
              )}
              <Field>
                <div className="flex items-center justify-between">
                  <div>
                    <FieldLabel>Public Access</FieldLabel>
                    <FieldDescription>Allow connections from outside the network</FieldDescription>
                  </div>
                  <Switch checked={publicExposed} onCheckedChange={setPublicExposed} />
                </div>
              </Field>
            </div>
            <DialogFooter>
              <DialogClose asChild>
                <Button type="button" variant="outline">
                  Cancel
                </Button>
              </DialogClose>
              <Button type="submit" disabled={createMutation.isPending}>
                {createMutation.isPending && <Spinner className="size-4" />}
                Create Database
              </Button>
            </DialogFooter>
          </form>
        </DialogContent>
      </Dialog>
    </div>
  );
}
