import { Button } from "@/components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Field, FieldDescription, FieldLabel } from "@/components/ui/field";
import { Input } from "@/components/ui/input";
import { Slider } from "@/components/ui/slider";
import { Spinner } from "@/components/ui/spinner";
import { Switch } from "@/components/ui/switch";
import { type UpdateDatabaseRequest, databasesApi, getErrorMessage, systemApi } from "@/lib/api";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import * as React from "react";
import { toast } from "sonner";

import type { DatabaseResponse } from "@/lib/api/types";

const SettingsPanel = React.memo(function SettingsPanel({
  database,
}: { database: DatabaseResponse }) {
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
    onError: (err) => toast.error(getErrorMessage(err, "Failed to save settings")),
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

export default SettingsPanel;
