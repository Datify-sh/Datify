import { BranchSwitcher } from "@/components/database/branch-switcher";
import { CreateBranchDialog } from "@/components/database/create-branch-dialog";
import {
  AlertDialog,
  AlertDialogAction,
  AlertDialogCancel,
  AlertDialogContent,
  AlertDialogDescription,
  AlertDialogFooter,
  AlertDialogHeader,
  AlertDialogTitle,
  AlertDialogTrigger,
} from "@/components/ui/alert-dialog";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardContent } from "@/components/ui/card";
import { Progress } from "@/components/ui/progress";
import { Skeleton } from "@/components/ui/skeleton";
import { Spinner } from "@/components/ui/spinner";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { useMetricsStream } from "@/hooks/use-metrics-stream";
import { databasesApi, getErrorMessage, projectsApi } from "@/lib/api";
import type { TerminalType } from "@/pages/databases/detail-context";
import {
  ArrowDownIcon,
  BarChartIcon,
  CodeIcon,
  CommandLineIcon,
  Database01Icon,
  Delete01Icon,
  File01Icon,
  GitBranchIcon,
  Globe02Icon,
  PauseIcon,
  PlayIcon,
  RefreshIcon,
  Settings01Icon,
  SquareLock02Icon,
} from "@hugeicons/core-free-icons";
import { HugeiconsIcon } from "@hugeicons/react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { format } from "date-fns";
import * as React from "react";
import { Link, Outlet, useLocation, useNavigate, useParams } from "react-router-dom";
import { toast } from "sonner";

const statusColors: Record<string, "default" | "secondary" | "destructive" | "outline"> = {
  running: "default",
  stopped: "secondary",
  starting: "outline",
  stopping: "outline",
  error: "destructive",
};

type DatabaseTabKey =
  | "connection"
  | "editor"
  | "branches"
  | "metrics"
  | "terminal"
  | "logs"
  | "config"
  | "settings";

const loadConnectionTab = () => import("./tabs/connection");
const loadEditorTab = () => import("./tabs/editor");
const loadBranchesTab = () => import("./tabs/branches");
const loadMetricsTab = () => import("./tabs/metrics");
const loadTerminalTab = () => import("./tabs/terminal");
const loadLogsTab = () => import("./tabs/logs");
const loadConfigTab = () => import("./tabs/config");
const loadSettingsTab = () => import("./tabs/settings");

const TAB_ROUTE_MAP: Record<DatabaseTabKey, string> = {
  connection: "",
  editor: "editor",
  branches: "branches",
  metrics: "metrics",
  terminal: "terminal",
  logs: "logs",
  config: "config",
  settings: "settings",
};

export function DatabaseDetailPage() {
  const { id } = useParams<{ id: string }>();
  const location = useLocation();
  const navigate = useNavigate();
  const queryClient = useQueryClient();
  const [terminalType, setTerminalType] = React.useState<TerminalType>("psql");
  const [isCreateBranchOpen, setIsCreateBranchOpen] = React.useState(false);
  const openCreateBranch = React.useCallback(() => setIsCreateBranchOpen(true), []);

  const {
    data: database,
    isLoading,
    error,
  } = useQuery({
    queryKey: ["database", id],
    queryFn: () => (id ? databasesApi.get(id) : Promise.reject()),
    enabled: !!id,
    refetchInterval: 10000,
    staleTime: 5000,
    retry: 3,
    retryDelay: 1000,
  });

  const { data: project } = useQuery({
    queryKey: ["project", database?.project_id],
    queryFn: () => (database?.project_id ? projectsApi.get(database.project_id) : Promise.reject()),
    enabled: !!database?.project_id,
  });

  const { data: parentDatabase } = useQuery({
    queryKey: ["database", database?.branch?.parent_id],
    queryFn: () =>
      database?.branch?.parent_id ? databasesApi.get(database.branch.parent_id) : Promise.reject(),
    enabled: !!database?.branch?.parent_id,
  });

  const { metrics: realtimeMetrics } = useMetricsStream(id ?? "", {
    enabled: !!id && database?.status === "running",
  });

  const startMutation = useMutation({
    mutationFn: () => {
      if (!id) return Promise.reject(new Error("Database ID is required"));
      return databasesApi.start(id);
    },
    onMutate: async () => {
      await queryClient.cancelQueries({ queryKey: ["database", id] });
      queryClient.setQueryData(["database", id], (old: typeof database) => {
        if (!old) return old;
        return { ...old, status: "starting" };
      });
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["database", id] });
      queryClient.invalidateQueries({ queryKey: ["databases", database?.project_id] });
      queryClient.invalidateQueries({ queryKey: ["all-databases-for-stats"] });
      toast.success("Database starting...");
    },
    onError: (err) => {
      queryClient.invalidateQueries({ queryKey: ["database", id] });
      toast.error(getErrorMessage(err, "Failed to start database"));
    },
  });

  const stopMutation = useMutation({
    mutationFn: () => {
      if (!id) return Promise.reject(new Error("Database ID is required"));
      return databasesApi.stop(id);
    },
    onMutate: async () => {
      await queryClient.cancelQueries({ queryKey: ["database", id] });
      queryClient.setQueryData(["database", id], (old: typeof database) => {
        if (!old) return old;
        return { ...old, status: "stopping" };
      });
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["database", id] });
      queryClient.invalidateQueries({ queryKey: ["databases", database?.project_id] });
      queryClient.invalidateQueries({ queryKey: ["all-databases-for-stats"] });
      toast.success("Database stopping...");
    },
    onError: (err) => {
      queryClient.invalidateQueries({ queryKey: ["database", id] });
      toast.error(getErrorMessage(err, "Failed to stop database"));
    },
  });

  const restartMutation = useMutation({
    mutationFn: async () => {
      if (!id) {
        throw new Error("Database ID is required");
      }
      if (database?.status === "running") {
        await databasesApi.stop(id);
      }
      return databasesApi.start(id);
    },
    onMutate: async () => {
      await queryClient.cancelQueries({ queryKey: ["database", id] });
      queryClient.setQueryData(["database", id], (old: typeof database) => {
        if (!old) return old;
        return { ...old, status: old.status === "running" ? "stopping" : "starting" };
      });
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["database", id] });
      queryClient.invalidateQueries({ queryKey: ["databases", database?.project_id] });
      queryClient.invalidateQueries({ queryKey: ["all-databases-for-stats"] });
      toast.success("Database restarting...");
    },
    onError: (err) => {
      queryClient.invalidateQueries({ queryKey: ["database", id] });
      toast.error(getErrorMessage(err, "Failed to restart database"));
    },
  });

  const deleteMutation = useMutation({
    mutationFn: () => {
      if (!id) return Promise.reject(new Error("Database ID is required"));
      return databasesApi.delete(id);
    },
    onSuccess: () => {
      const projectId = database?.project_id;
      queryClient.invalidateQueries({ queryKey: ["databases", projectId] });
      queryClient.invalidateQueries({ queryKey: ["projects"] });
      queryClient.invalidateQueries({ queryKey: ["all-databases-for-stats"] });
      queryClient.invalidateQueries({ queryKey: ["branches"] });
      queryClient.removeQueries({ queryKey: ["database", id] });
      toast.success("Database deleted");
      navigate(`/projects/${projectId}`);
    },
    onError: (err) => toast.error(getErrorMessage(err, "Failed to delete database")),
  });

  const syncMutation = useMutation({
    mutationFn: () => {
      if (!id) return Promise.reject(new Error("Database ID is required"));
      return databasesApi.syncFromParent(id);
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["database", id] });
      if (database?.branch?.parent_id) {
        queryClient.invalidateQueries({ queryKey: ["database", database.branch.parent_id] });
      }
      toast.success("Synced from parent successfully");
    },
    onError: (err) => toast.error(getErrorMessage(err, "Failed to sync from parent")),
  });

  const isRunning = database?.status === "running";
  const isTransitioning = database?.status === "starting" || database?.status === "stopping";
  const isActionLoading =
    startMutation.isPending || stopMutation.isPending || restartMutation.isPending;
  const isKeyValue = database?.database_type === "valkey" || database?.database_type === "redis";
  const isValkey = database?.database_type === "valkey";
  const isRedis = database?.database_type === "redis";
  const hasBranches = database?.branch !== undefined;
  const isChildBranch = database?.branch?.parent_id != null;

  const tabFromPath = React.useMemo<DatabaseTabKey>(() => {
    if (!id) return "connection";
    const basePath = `/databases/${id}`;
    const remainder = location.pathname.startsWith(basePath)
      ? location.pathname.slice(basePath.length)
      : "";
    const segment = remainder.replace(/^\//, "").split("/")[0] || "connection";
    if (segment in TAB_ROUTE_MAP) return segment as DatabaseTabKey;
    return "connection";
  }, [id, location.pathname]);

  const activeTab = !hasBranches && tabFromPath === "branches" ? "connection" : tabFromPath;

  const handleTabChange = React.useCallback(
    (nextTab: string) => {
      if (!id) return;
      if (!(nextTab in TAB_ROUTE_MAP)) return;
      const segment = TAB_ROUTE_MAP[nextTab as DatabaseTabKey];
      const target = segment ? `/databases/${id}/${segment}` : `/databases/${id}`;
      navigate(target);
    },
    [id, navigate],
  );

  React.useEffect(() => {
    if (!hasBranches && tabFromPath === "branches" && id) {
      navigate(`/databases/${id}`, { replace: true });
    }
  }, [hasBranches, tabFromPath, id, navigate]);

  const outletContext = React.useMemo(() => {
    if (!database) return null;
    return {
      database,
      project,
      parentDatabase,
      id: id ?? database.id,
      isRunning: !!isRunning,
      isTransitioning: !!isTransitioning,
      isKeyValue: !!isKeyValue,
      isValkey: !!isValkey,
      isRedis: !!isRedis,
      hasBranches: !!hasBranches,
      isChildBranch: !!isChildBranch,
      terminalType,
      setTerminalType,
      openCreateBranch,
      startDatabase: () => startMutation.mutate(),
      startDatabasePending: startMutation.isPending,
    };
  }, [
    database,
    project,
    parentDatabase,
    id,
    isRunning,
    isTransitioning,
    isKeyValue,
    isValkey,
    isRedis,
    hasBranches,
    isChildBranch,
    terminalType,
    openCreateBranch,
    startMutation,
    startMutation.isPending,
  ]);

  if (isLoading || !database) {
    return (
      <div className="space-y-6">
        <div className="flex items-center justify-between">
          <div className="space-y-2">
            <Skeleton className="h-8 w-48" />
            <Skeleton className="h-4 w-96" />
          </div>
          <Skeleton className="h-10 w-24" />
        </div>
        <div className="grid gap-4 md:grid-cols-2 lg:grid-cols-4">
          <Skeleton className="h-24" />
          <Skeleton className="h-24" />
          <Skeleton className="h-24" />
          <Skeleton className="h-24" />
        </div>
        <Skeleton className="h-64" />
      </div>
    );
  }

  if (error || !database) {
    return (
      <div className="py-16 text-center">
        <p className="text-muted-foreground">
          {error ? "Failed to load database" : "Database not found"}
        </p>
        <Button className="mt-4" variant="outline" asChild>
          <Link to="/projects">Back to projects</Link>
        </Button>
      </div>
    );
  }

  const getStorageUsedBytes = () => {
    if (!realtimeMetrics) return 0;
    if (realtimeMetrics.database_type === "postgres") {
      return realtimeMetrics.storage?.database_size_bytes ?? 0;
    }
    return realtimeMetrics.memory?.used_memory ?? 0;
  };
  const storageUsedBytes = getStorageUsedBytes();
  const storageUsed =
    storageUsedBytes > 0
      ? Math.round(storageUsedBytes / (1024 * 1024))
      : (database.storage_used_mb ?? 0);
  const storagePercent = (storageUsed / database.resources.storage_limit_mb) * 100;

  return (
    <div className="space-y-6">
      <div className="flex items-start justify-between gap-4">
        <div className="min-w-0 flex-1">
          <div className="flex items-center gap-3 flex-wrap">
            <h1 className="text-2xl font-bold tracking-tight font-mono">{database.name}</h1>
            <Badge variant={statusColors[database.status] || "secondary"}>{database.status}</Badge>
            <Badge variant="outline">
              {database.public_exposed ? (
                <>
                  <HugeiconsIcon icon={Globe02Icon} className="size-3" strokeWidth={2} />
                  Public
                </>
              ) : (
                <>
                  <HugeiconsIcon icon={SquareLock02Icon} className="size-3" strokeWidth={2} />
                  Internal
                </>
              )}
            </Badge>
            {hasBranches && id && (
              <BranchSwitcher
                databaseId={id}
                currentBranchName={database.branch.name}
                onCreateBranch={openCreateBranch}
              />
            )}
          </div>
          <p className="text-muted-foreground mt-1">
            <span className="font-mono">
              {database.database_type === "valkey"
                ? `Valkey ${database.valkey_version}`
                : database.database_type === "redis"
                  ? `Redis ${database.redis_version}`
                  : `PostgreSQL ${database.postgres_version}`}
            </span>
            {project && (
              <>
                {" · "}
                <Link to={`/projects/${project.id}`} className="hover:underline">
                  {project.name}
                </Link>
              </>
            )}
            {" · Created "}
            {format(new Date(database.created_at), "MMM d, yyyy 'at' h:mm a")}
          </p>
          {isChildBranch && parentDatabase && (
            <p className="text-muted-foreground mt-1 text-sm flex items-center gap-1">
              <HugeiconsIcon icon={GitBranchIcon} className="size-3.5" strokeWidth={2} />
              Forked from{" "}
              <Link
                to={`/databases/${parentDatabase.id}`}
                className="text-foreground hover:underline"
              >
                {parentDatabase.name}
              </Link>
              {database.branch.forked_at && (
                <span>
                  {" · Last synced "}
                  {format(new Date(database.branch.forked_at), "MMM d, yyyy 'at' h:mm a")}
                </span>
              )}
            </p>
          )}
        </div>

        <div className="flex items-center gap-2 shrink-0">
          {isChildBranch && isRunning && parentDatabase?.status === "running" && (
            <Button
              variant="outline"
              onClick={() => syncMutation.mutate()}
              disabled={syncMutation.isPending}
            >
              {syncMutation.isPending ? (
                <Spinner className="size-4" />
              ) : (
                <HugeiconsIcon icon={ArrowDownIcon} className="size-4" strokeWidth={2} />
              )}
              Sync from Parent
            </Button>
          )}
          {isRunning ? (
            <Button
              variant="outline"
              onClick={() => stopMutation.mutate()}
              disabled={isActionLoading || isTransitioning}
            >
              {stopMutation.isPending ? (
                <Spinner className="size-4" />
              ) : (
                <HugeiconsIcon icon={PauseIcon} className="size-4" strokeWidth={2} />
              )}
              Stop
            </Button>
          ) : (
            <Button
              onClick={() => startMutation.mutate()}
              disabled={isActionLoading || isTransitioning}
            >
              {startMutation.isPending ? (
                <Spinner className="size-4" />
              ) : (
                <HugeiconsIcon icon={PlayIcon} className="size-4" strokeWidth={2} />
              )}
              Start
            </Button>
          )}
          <Button
            variant="outline"
            onClick={() => restartMutation.mutate()}
            disabled={!isRunning || isActionLoading || isTransitioning}
          >
            {restartMutation.isPending ? (
              <Spinner className="size-4" />
            ) : (
              <HugeiconsIcon icon={RefreshIcon} className="size-4" strokeWidth={2} />
            )}
            Restart
          </Button>
          <AlertDialog>
            <AlertDialogTrigger asChild>
              <Button
                variant="outline"
                size="icon"
                className="text-destructive hover:text-destructive"
              >
                <HugeiconsIcon icon={Delete01Icon} className="size-4" strokeWidth={2} />
              </Button>
            </AlertDialogTrigger>
            <AlertDialogContent>
              <AlertDialogHeader>
                <AlertDialogTitle>Delete Database</AlertDialogTitle>
                <AlertDialogDescription>
                  Are you sure you want to delete "{database.name}"? This action cannot be undone
                  and all data will be permanently lost.
                </AlertDialogDescription>
              </AlertDialogHeader>
              <AlertDialogFooter>
                <AlertDialogCancel>Cancel</AlertDialogCancel>
                <AlertDialogAction
                  onClick={() => deleteMutation.mutate()}
                  className="bg-destructive text-destructive-foreground hover:bg-destructive/90"
                >
                  {deleteMutation.isPending ? <Spinner className="size-4" /> : "Delete"}
                </AlertDialogAction>
              </AlertDialogFooter>
            </AlertDialogContent>
          </AlertDialog>
        </div>
      </div>

      <Card>
        <CardContent className="pt-4">
          <div className="flex items-center justify-between mb-2">
            <span className="text-sm font-medium">Storage</span>
            <span className="text-sm text-muted-foreground">
              {storageUsed} MB / {database.resources.storage_limit_mb} MB
            </span>
          </div>
          <Progress value={storagePercent} className="h-2" />
          <div className="flex items-center justify-end mt-2 text-xs text-muted-foreground">
            <span>{storagePercent.toFixed(1)}% used</span>
          </div>
        </CardContent>
      </Card>

      <Tabs value={activeTab} onValueChange={handleTabChange}>
        <TabsList>
          <TabsTrigger value="connection" onMouseEnter={() => void loadConnectionTab()}>
            <HugeiconsIcon icon={Database01Icon} className="size-4" strokeWidth={2} />
            Connection
          </TabsTrigger>
          <TabsTrigger value="editor" onMouseEnter={() => void loadEditorTab()}>
            <HugeiconsIcon icon={CodeIcon} className="size-4" strokeWidth={2} />
            Editor
          </TabsTrigger>
          {hasBranches && (
            <TabsTrigger value="branches" onMouseEnter={() => void loadBranchesTab()}>
              <HugeiconsIcon icon={GitBranchIcon} className="size-4" strokeWidth={2} />
              Branches
            </TabsTrigger>
          )}
          <TabsTrigger value="metrics" onMouseEnter={() => void loadMetricsTab()}>
            <HugeiconsIcon icon={BarChartIcon} className="size-4" strokeWidth={2} />
            Metrics
          </TabsTrigger>
          <TabsTrigger value="terminal" onMouseEnter={() => void loadTerminalTab()}>
            <HugeiconsIcon icon={CommandLineIcon} className="size-4" strokeWidth={2} />
            Terminal
          </TabsTrigger>
          <TabsTrigger value="logs" onMouseEnter={() => void loadLogsTab()}>
            <HugeiconsIcon icon={File01Icon} className="size-4" strokeWidth={2} />
            Logs
          </TabsTrigger>
          <TabsTrigger value="config" onMouseEnter={() => void loadConfigTab()}>
            <HugeiconsIcon icon={Settings01Icon} className="size-4" strokeWidth={2} />
            Config
          </TabsTrigger>
          <TabsTrigger value="settings" onMouseEnter={() => void loadSettingsTab()}>
            <HugeiconsIcon icon={Settings01Icon} className="size-4" strokeWidth={2} />
            Settings
          </TabsTrigger>
        </TabsList>

        <TabsContent value={activeTab} className="mt-6">
          <Outlet context={outletContext} />
        </TabsContent>
      </Tabs>

      {id && (
        <CreateBranchDialog
          open={isCreateBranchOpen}
          onOpenChange={setIsCreateBranchOpen}
          databaseId={id}
          sourceBranchName={database.branch?.name ?? "main"}
          databaseType={database.database_type}
        />
      )}
    </div>
  );
}
