import { BranchPanel } from "@/components/database/branch-panel";
import { BranchSwitcher } from "@/components/database/branch-switcher";
import { CreateBranchDialog } from "@/components/database/create-branch-dialog";
import { MetricsPanel } from "@/components/database/metrics-panel";
import { QueryLogsPanel } from "@/components/database/query-logs-panel";
import { TerminalComponent, type TerminalRef } from "@/components/terminal";
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
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Field, FieldDescription, FieldLabel } from "@/components/ui/field";
import { Input } from "@/components/ui/input";
import { Progress } from "@/components/ui/progress";
import { Skeleton } from "@/components/ui/skeleton";
import { Slider } from "@/components/ui/slider";
import { Spinner } from "@/components/ui/spinner";
import { Switch } from "@/components/ui/switch";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { useLogStream } from "@/hooks/use-log-stream";
import { useMetricsStream } from "@/hooks/use-metrics-stream";
import { type UpdateDatabaseRequest, databasesApi, projectsApi, systemApi } from "@/lib/api";
import {
  ArrowDownIcon,
  BarChartIcon,
  CommandLineIcon,
  Copy01Icon,
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
  ViewIcon,
  ViewOffIcon,
} from "@hugeicons/core-free-icons";
import { HugeiconsIcon } from "@hugeicons/react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useVirtualizer } from "@tanstack/react-virtual";
import { format } from "date-fns";
import * as React from "react";
import { Link, useNavigate, useParams } from "react-router-dom";
import { toast } from "sonner";

const statusColors: Record<string, "default" | "secondary" | "destructive" | "outline"> = {
  running: "default",
  stopped: "secondary",
  starting: "outline",
  stopping: "outline",
  error: "destructive",
};

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
                  className="absolute left-0 w-full flex gap-3 text-xs leading-relaxed hover:bg-white/5 px-4 py-0.5"
                  style={{
                    height: `${virtualRow.size}px`,
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

const TerminalPanel = React.memo(function TerminalPanel({
  databaseId,
  type,
}: { databaseId: string; type: "shell" | "psql" | "valkey-cli" | "redis-cli" }) {
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

const ConnectionPanel = React.memo(function ConnectionPanel({
  database,
}: { database: NonNullable<Awaited<ReturnType<typeof databasesApi.get>>> }) {
  const connection = database.connection;
  const [showPassword, setShowPassword] = React.useState(false);

  const { data: systemInfo } = useQuery({
    queryKey: ["system-info"],
    queryFn: () => systemApi.getInfo(),
    staleTime: 5 * 60 * 1000,
  });

  const copyToClipboard = React.useCallback((text: string, label: string) => {
    navigator.clipboard.writeText(text);
    toast.success(`${label} copied`);
  }, []);

  const togglePassword = React.useCallback(() => setShowPassword((s) => !s), []);

  if (!connection) {
    return (
      <Card>
        <CardContent className="flex flex-col items-center justify-center py-12">
          <HugeiconsIcon
            icon={Database01Icon}
            className="size-12 text-muted-foreground"
            strokeWidth={1.5}
          />
          <p className="mt-4 text-muted-foreground">Start the database to see connection details</p>
        </CardContent>
      </Card>
    );
  }

  const publicHost = systemInfo?.public_host;
  const displayHost =
    database.public_exposed && publicHost && publicHost !== "localhost"
      ? publicHost
      : connection.host;
  const displayConnectionString =
    database.public_exposed && publicHost && publicHost !== "localhost"
      ? connection.connection_string.replace(connection.host, publicHost)
      : connection.connection_string;

  return (
    <div className="space-y-6">
      <Card>
        <CardHeader>
          <CardTitle className="text-base">Connection String</CardTitle>
          <CardDescription>Use this to connect from your application</CardDescription>
        </CardHeader>
        <CardContent>
          <div className="flex gap-2">
            <Input value={displayConnectionString} readOnly className="font-mono text-xs" />
            <Button
              variant="outline"
              size="icon"
              onClick={() => copyToClipboard(displayConnectionString, "Connection string")}
            >
              <HugeiconsIcon icon={Copy01Icon} className="size-4" strokeWidth={2} />
            </Button>
          </div>
        </CardContent>
      </Card>

      <div className="grid gap-4 sm:grid-cols-2 lg:grid-cols-3">
        {[
          { label: "Host", value: displayHost },
          { label: "Port", value: String(connection.port) },
          { label: "Database", value: connection.database },
          { label: "Username", value: connection.username },
        ].map((item) => (
          <Card key={item.label}>
            <CardContent className="pt-4">
              <div className="flex items-start justify-between gap-2">
                <div className="min-w-0 flex-1">
                  <p className="text-xs text-muted-foreground">{item.label}</p>
                  <code className="text-sm break-all">{item.value}</code>
                </div>
                <Button
                  variant="ghost"
                  size="icon-sm"
                  className="shrink-0"
                  onClick={() => copyToClipboard(item.value, item.label)}
                >
                  <HugeiconsIcon icon={Copy01Icon} className="size-3.5" strokeWidth={2} />
                </Button>
              </div>
            </CardContent>
          </Card>
        ))}
        <Card>
          <CardContent className="pt-4">
            <div className="flex items-start justify-between gap-2">
              <div className="min-w-0 flex-1">
                <p className="text-xs text-muted-foreground">Password</p>
                <code className="text-sm break-all">
                  {showPassword ? connection.password : "••••••••"}
                </code>
              </div>
              <div className="flex gap-1 shrink-0">
                <Button variant="ghost" size="icon-sm" onClick={togglePassword}>
                  <HugeiconsIcon
                    icon={showPassword ? ViewOffIcon : ViewIcon}
                    className="size-3.5"
                    strokeWidth={2}
                  />
                </Button>
                <Button
                  variant="ghost"
                  size="icon-sm"
                  onClick={() => copyToClipboard(connection.password, "Password")}
                >
                  <HugeiconsIcon icon={Copy01Icon} className="size-3.5" strokeWidth={2} />
                </Button>
              </div>
            </div>
          </CardContent>
        </Card>
      </div>
    </div>
  );
});

const SettingsPanel = React.memo(function SettingsPanel({
  database,
}: { database: NonNullable<Awaited<ReturnType<typeof databasesApi.get>>> }) {
  const queryClient = useQueryClient();

  const dbName = database.name;
  const dbCpuLimit = database.resources.cpu_limit;
  const dbMemoryLimit = database.resources.memory_limit_mb;
  const dbStorageLimit = database.resources.storage_limit_mb;
  const dbPublicExposed = database.public_exposed ?? false;

  const [settings, setSettings] = React.useState({
    name: dbName,
    cpu_limit: dbCpuLimit,
    memory_limit_mb: dbMemoryLimit,
    storage_limit_mb: dbStorageLimit,
    public_exposed: dbPublicExposed,
  });

  const { data: systemInfo } = useQuery({
    queryKey: ["system-info"],
    queryFn: () => systemApi.getInfo(),
    staleTime: 5 * 60 * 1000,
    refetchOnWindowFocus: false,
  });

  const maxCpu = systemInfo?.cpu_cores ?? 4;
  const maxMemory = systemInfo?.total_memory_mb ?? 4096;

  React.useEffect(() => {
    setSettings({
      name: dbName,
      cpu_limit: dbCpuLimit,
      memory_limit_mb: dbMemoryLimit,
      storage_limit_mb: dbStorageLimit,
      public_exposed: dbPublicExposed,
    });
  }, [dbName, dbCpuLimit, dbMemoryLimit, dbStorageLimit, dbPublicExposed]);

  const updateMutation = useMutation({
    mutationFn: (data: UpdateDatabaseRequest) => databasesApi.update(database.id, data),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["database", database.id] });
      toast.success("Settings saved");
    },
    onError: () => toast.error("Failed to save settings"),
  });

  const handleSave = React.useCallback(() => {
    updateMutation.mutate({
      name: settings.name !== dbName ? settings.name : null,
      cpu_limit: settings.cpu_limit !== dbCpuLimit ? settings.cpu_limit : null,
      memory_limit_mb: settings.memory_limit_mb !== dbMemoryLimit ? settings.memory_limit_mb : null,
      storage_limit_mb:
        settings.storage_limit_mb !== dbStorageLimit ? settings.storage_limit_mb : null,
      public_exposed: settings.public_exposed !== dbPublicExposed ? settings.public_exposed : null,
    });
  }, [
    updateMutation,
    settings,
    dbName,
    dbCpuLimit,
    dbMemoryLimit,
    dbStorageLimit,
    dbPublicExposed,
  ]);

  const hasChanges =
    settings.name !== dbName ||
    settings.cpu_limit !== dbCpuLimit ||
    settings.memory_limit_mb !== dbMemoryLimit ||
    settings.storage_limit_mb !== dbStorageLimit ||
    settings.public_exposed !== dbPublicExposed;

  const isRunning = database.status === "running";

  const formatMemory = (mb: number) => {
    if (mb >= 1024) return `${(mb / 1024).toFixed(1)} GB`;
    return `${mb} MB`;
  };

  return (
    <div className="space-y-6">
      <Card>
        <CardHeader>
          <CardTitle className="text-base">General</CardTitle>
        </CardHeader>
        <CardContent className="space-y-4">
          {isRunning && (
            <div className="rounded-md bg-muted px-3 py-2 text-sm text-muted-foreground">
              Stop the database to change name or access settings
            </div>
          )}
          <Field>
            <FieldLabel>Database Name</FieldLabel>
            <Input
              value={settings.name}
              onChange={(e) => setSettings({ ...settings, name: e.target.value })}
              disabled={isRunning}
            />
          </Field>

          <Field>
            <div className="flex items-center justify-between">
              <div>
                <FieldLabel>Public Access</FieldLabel>
                <FieldDescription>Allow connections from outside the network</FieldDescription>
              </div>
              <Switch
                checked={settings.public_exposed}
                onCheckedChange={(checked) => setSettings({ ...settings, public_exposed: checked })}
                disabled={isRunning}
              />
            </div>
          </Field>
        </CardContent>
      </Card>

      <Card>
        <CardHeader>
          <CardTitle className="text-base">Resources</CardTitle>
          <CardDescription>Resource limits for this database container</CardDescription>
        </CardHeader>
        <CardContent className="space-y-6">
          {isRunning && (
            <div className="rounded-md bg-muted px-3 py-2 text-sm text-muted-foreground">
              Stop the database to change resource limits
            </div>
          )}
          <Field>
            <div className="flex items-center justify-between">
              <FieldLabel>CPU Cores</FieldLabel>
              <span className="text-sm font-medium">{settings.cpu_limit}</span>
            </div>
            <Slider
              value={[settings.cpu_limit]}
              onValueChange={(value) =>
                setSettings({ ...settings, cpu_limit: Array.isArray(value) ? value[0] : value })
              }
              min={0.5}
              max={maxCpu}
              step={0.5}
              disabled={isRunning}
            />
            <div className="flex justify-between text-xs text-muted-foreground">
              <span>0.5</span>
              <span>{maxCpu}</span>
            </div>
          </Field>

          <Field>
            <div className="flex items-center justify-between">
              <FieldLabel>Memory</FieldLabel>
              <span className="text-sm font-medium">{formatMemory(settings.memory_limit_mb)}</span>
            </div>
            <Slider
              value={[settings.memory_limit_mb]}
              onValueChange={(value) =>
                setSettings({
                  ...settings,
                  memory_limit_mb: Array.isArray(value) ? value[0] : value,
                })
              }
              min={256}
              max={maxMemory}
              step={256}
              disabled={isRunning}
            />
            <div className="flex justify-between text-xs text-muted-foreground">
              <span>256 MB</span>
              <span>{formatMemory(maxMemory)}</span>
            </div>
          </Field>

          <Field>
            <div className="flex items-center justify-between">
              <FieldLabel>Storage</FieldLabel>
              <span className="text-sm font-medium">{formatMemory(settings.storage_limit_mb)}</span>
            </div>
            <Slider
              value={[settings.storage_limit_mb]}
              onValueChange={(value) =>
                setSettings({
                  ...settings,
                  storage_limit_mb: Array.isArray(value) ? value[0] : value,
                })
              }
              min={512}
              max={102400}
              step={512}
              disabled={isRunning}
            />
            <div className="flex justify-between text-xs text-muted-foreground">
              <span>512 MB</span>
              <span>100 GB</span>
            </div>
          </Field>
        </CardContent>
      </Card>

      <div className="flex justify-end">
        <Button onClick={handleSave} disabled={!hasChanges || updateMutation.isPending}>
          {updateMutation.isPending && <Spinner className="size-4" />}
          Save Changes
        </Button>
      </div>
    </div>
  );
});

export function DatabaseDetailPage() {
  const { id } = useParams<{ id: string }>();
  const navigate = useNavigate();
  const queryClient = useQueryClient();
  const [terminalType, setTerminalType] = React.useState<
    "shell" | "psql" | "valkey-cli" | "redis-cli"
  >("psql");
  const [isCreateBranchOpen, setIsCreateBranchOpen] = React.useState(false);

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
    onError: () => {
      queryClient.invalidateQueries({ queryKey: ["database", id] });
      toast.error("Failed to start database");
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
    onError: () => {
      queryClient.invalidateQueries({ queryKey: ["database", id] });
      toast.error("Failed to stop database");
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
    onError: () => toast.error("Failed to delete database"),
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
    onError: () => toast.error("Failed to sync from parent"),
  });

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

  const isRunning = database.status === "running";
  const isTransitioning = database.status === "starting" || database.status === "stopping";
  const isActionLoading = startMutation.isPending || stopMutation.isPending;

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

  const isKeyValue = database.database_type === "valkey" || database.database_type === "redis";
  const isValkey = database.database_type === "valkey";
  const isRedis = database.database_type === "redis";
  const hasBranches = database.branch !== undefined;
  const isChildBranch = database.branch?.parent_id != null;

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
                onCreateBranch={() => setIsCreateBranchOpen(true)}
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

      <Tabs defaultValue="connection">
        <TabsList>
          <TabsTrigger value="connection">
            <HugeiconsIcon icon={Database01Icon} className="size-4" strokeWidth={2} />
            Connection
          </TabsTrigger>
          {hasBranches && (
            <TabsTrigger value="branches">
              <HugeiconsIcon icon={GitBranchIcon} className="size-4" strokeWidth={2} />
              Branches
            </TabsTrigger>
          )}
          <TabsTrigger value="metrics">
            <HugeiconsIcon icon={BarChartIcon} className="size-4" strokeWidth={2} />
            Metrics
          </TabsTrigger>
          <TabsTrigger value="terminal">
            <HugeiconsIcon icon={CommandLineIcon} className="size-4" strokeWidth={2} />
            Terminal
          </TabsTrigger>
          <TabsTrigger value="logs">
            <HugeiconsIcon icon={File01Icon} className="size-4" strokeWidth={2} />
            Logs
          </TabsTrigger>
          <TabsTrigger value="settings">
            <HugeiconsIcon icon={Settings01Icon} className="size-4" strokeWidth={2} />
            Settings
          </TabsTrigger>
        </TabsList>

        <TabsContent value="connection" className="mt-6">
          <ConnectionPanel database={database} />
        </TabsContent>

        {hasBranches && id && (
          <TabsContent value="branches" className="mt-6">
            <BranchPanel
              databaseId={id}
              currentBranchId={id}
              onCreateBranch={() => setIsCreateBranchOpen(true)}
            />
          </TabsContent>
        )}

        <TabsContent value="metrics" className="mt-6">
          <div className="space-y-6">
            <MetricsPanel
              databaseId={database.id}
              databaseType={database.database_type}
              isRunning={isRunning}
            />
            {!isKeyValue && <QueryLogsPanel databaseId={database.id} isRunning={isRunning} />}
          </div>
        </TabsContent>

        <TabsContent value="terminal" className="mt-6">
          {isRunning ? (
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
          ) : (
            <Card>
              <CardContent className="flex flex-col items-center justify-center py-12">
                <HugeiconsIcon
                  icon={CommandLineIcon}
                  className="size-12 text-muted-foreground"
                  strokeWidth={1.5}
                />
                <p className="mt-4 text-muted-foreground">
                  Start the database to access the terminal
                </p>
                <Button className="mt-4" onClick={() => startMutation.mutate()}>
                  <HugeiconsIcon icon={PlayIcon} className="size-4" strokeWidth={2} />
                  Start Database
                </Button>
              </CardContent>
            </Card>
          )}
        </TabsContent>

        <TabsContent value="logs" className="mt-6">
          <LogsPanel databaseId={database.id} />
        </TabsContent>

        <TabsContent value="settings" className="mt-6">
          <SettingsPanel database={database} />
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
