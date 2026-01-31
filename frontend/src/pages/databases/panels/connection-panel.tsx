import { Button } from "@/components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { systemApi } from "@/lib/api";
import { Copy01Icon, Database01Icon, ViewIcon, ViewOffIcon } from "@hugeicons/core-free-icons";
import { HugeiconsIcon } from "@hugeicons/react";
import { useQuery } from "@tanstack/react-query";
import * as React from "react";
import { toast } from "sonner";

import type { DatabaseResponse } from "@/lib/api/types";

const ConnectionPanel = React.memo(function ConnectionPanel({
  database,
}: { database: DatabaseResponse }) {
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

export default ConnectionPanel;
