import { CreateProjectDialog } from "@/components/create-project-dialog";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Skeleton } from "@/components/ui/skeleton";
import { useAuth } from "@/contexts/auth-context";
import { type DatabaseResponse, databasesApi, projectsApi } from "@/lib/api";
import {
  Add01Icon,
  ArrowRight01Icon,
  Copy01Icon,
  Database01Icon,
  Folder01Icon,
  PlayIcon,
} from "@hugeicons/core-free-icons";
import { HugeiconsIcon } from "@hugeicons/react";
import { useQuery } from "@tanstack/react-query";
import { format } from "date-fns";
import * as React from "react";
import { Link } from "react-router-dom";
import { toast } from "sonner";

export function DashboardPage() {
  const { user } = useAuth();
  const [createOpen, setCreateOpen] = React.useState(false);

  const { data: projectsData, isLoading: projectsLoading } = useQuery({
    queryKey: ["projects"],
    queryFn: () => projectsApi.list({ limit: 100 }),
  });

  const projects = projectsData?.data || [];
  const totalProjects = projectsData?.total || 0;
  const totalDatabases = projects.reduce((acc, p) => acc + (p.database_count || 0), 0);

  const { data: allDatabasesData, isLoading: databasesLoading } = useQuery({
    queryKey: ["all-databases-for-stats"],
    queryFn: async () => {
      const results = await Promise.all(
        projects.map((p) => databasesApi.list(p.id, { limit: 100 })),
      );
      return results.flatMap((r) => r.data);
    },
    enabled: projects.length > 0,
    refetchInterval: 30000,
  });

  const allDatabases = allDatabasesData || [];
  const runningDatabases = allDatabases.filter((db) => db.status === "running");
  const isLoading = projectsLoading;

  const copyConnectionString = (db: DatabaseResponse) => {
    if (db.connection?.connection_string) {
      navigator.clipboard.writeText(db.connection.connection_string);
      toast.success("Connection string copied");
    }
  };

  return (
    <div className="space-y-8">
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-2xl font-semibold tracking-tight">
            Welcome back, {user?.name?.split(" ")[0]}
          </h1>
          <p className="text-muted-foreground">Here's what's happening with your databases</p>
        </div>
        <Button onClick={() => setCreateOpen(true)}>
          <HugeiconsIcon icon={Add01Icon} className="size-4" strokeWidth={2} />
          New Project
        </Button>
      </div>

      <div className="grid gap-4 sm:grid-cols-3">
        <Card>
          <CardHeader className="flex flex-row items-center justify-between pb-2">
            <CardTitle className="text-sm font-medium">Projects</CardTitle>
            <HugeiconsIcon
              icon={Folder01Icon}
              className="size-4 text-muted-foreground"
              strokeWidth={2}
            />
          </CardHeader>
          <CardContent>
            {isLoading ? (
              <Skeleton className="h-7 w-12" />
            ) : (
              <div className="text-2xl font-bold">{totalProjects}</div>
            )}
          </CardContent>
        </Card>

        <Card>
          <CardHeader className="flex flex-row items-center justify-between pb-2">
            <CardTitle className="text-sm font-medium">Databases</CardTitle>
            <HugeiconsIcon
              icon={Database01Icon}
              className="size-4 text-muted-foreground"
              strokeWidth={2}
            />
          </CardHeader>
          <CardContent>
            {isLoading ? (
              <Skeleton className="h-7 w-12" />
            ) : (
              <div className="text-2xl font-bold">{totalDatabases}</div>
            )}
          </CardContent>
        </Card>

        <Card>
          <CardHeader className="flex flex-row items-center justify-between pb-2">
            <CardTitle className="text-sm font-medium">Running</CardTitle>
            <HugeiconsIcon
              icon={PlayIcon}
              className="size-4 text-muted-foreground"
              strokeWidth={2}
            />
          </CardHeader>
          <CardContent>
            {isLoading || databasesLoading ? (
              <Skeleton className="h-7 w-12" />
            ) : (
              <div className="flex items-baseline gap-2">
                <span className="text-2xl font-bold">{runningDatabases.length}</span>
                {totalDatabases > 0 && (
                  <span className="text-sm text-muted-foreground">/ {totalDatabases}</span>
                )}
              </div>
            )}
          </CardContent>
        </Card>
      </div>

      {runningDatabases.length > 0 && (
        <div className="space-y-4">
          <div className="flex items-center gap-2">
            <div className="size-2 rounded-full bg-green-500 animate-pulse" />
            <h2 className="text-lg font-semibold">Running Databases</h2>
          </div>
          <div className="grid gap-3">
            {runningDatabases.slice(0, 5).map((db) => (
              <Link
                key={db.id}
                to={`/databases/${db.id}`}
                className="group flex items-center justify-between rounded-lg border p-4 transition-colors hover:bg-muted/50"
              >
                <div className="flex items-center gap-3">
                  <div className="flex size-9 items-center justify-center rounded-md bg-primary/10">
                    <HugeiconsIcon
                      icon={Database01Icon}
                      className="size-4 text-primary"
                      strokeWidth={2}
                    />
                  </div>
                  <div>
                    <div className="flex items-center gap-2">
                      <span className="font-medium font-mono text-sm">{db.name}</span>
                      <Badge variant="outline" className="text-[10px] h-5">
                        {db.database_type === "valkey"
                          ? `Valkey ${db.valkey_version}`
                          : `PG ${db.postgres_version}`}
                      </Badge>
                    </div>
                    <p className="text-xs text-muted-foreground">
                      {db.connection?.host}:{db.connection?.port}
                    </p>
                  </div>
                </div>
                <div className="flex items-center gap-2">
                  {db.connection && (
                    <Button
                      variant="ghost"
                      size="icon-sm"
                      className="opacity-0 group-hover:opacity-100"
                      onClick={(e) => {
                        e.preventDefault();
                        copyConnectionString(db);
                      }}
                    >
                      <HugeiconsIcon icon={Copy01Icon} className="size-3.5" strokeWidth={2} />
                    </Button>
                  )}
                  <HugeiconsIcon
                    icon={ArrowRight01Icon}
                    className="size-4 text-muted-foreground group-hover:text-foreground transition-colors"
                    strokeWidth={2}
                  />
                </div>
              </Link>
            ))}
          </div>
        </div>
      )}

      <div className="space-y-4">
        <div className="flex items-center justify-between">
          <h2 className="text-lg font-semibold">Recent Projects</h2>
          {projects.length > 0 && (
            <Button variant="ghost" size="sm" asChild>
              <Link to="/projects">
                View all
                <HugeiconsIcon icon={ArrowRight01Icon} className="size-4" strokeWidth={2} />
              </Link>
            </Button>
          )}
        </div>

        {isLoading ? (
          <div className="grid gap-4 sm:grid-cols-2 lg:grid-cols-3">
            {[1, 2, 3].map((i) => (
              <Card key={i}>
                <CardHeader>
                  <Skeleton className="h-5 w-32" />
                  <Skeleton className="h-4 w-48" />
                </CardHeader>
                <CardContent>
                  <Skeleton className="h-4 w-24" />
                </CardContent>
              </Card>
            ))}
          </div>
        ) : projects.length === 0 ? (
          <Card>
            <CardContent className="flex flex-col items-center justify-center py-16">
              <div className="flex size-14 items-center justify-center rounded-full bg-muted">
                <HugeiconsIcon
                  icon={Database01Icon}
                  className="size-7 text-muted-foreground"
                  strokeWidth={1.5}
                />
              </div>
              <h3 className="mt-4 text-lg font-semibold">No projects yet</h3>
              <p className="mt-1 text-sm text-muted-foreground text-center max-w-sm">
                Create your first project to start managing databases
              </p>
              <Button onClick={() => setCreateOpen(true)} className="mt-6">
                <HugeiconsIcon icon={Add01Icon} className="size-4" strokeWidth={2} />
                Create Project
              </Button>
            </CardContent>
          </Card>
        ) : (
          <div className="grid gap-4 sm:grid-cols-2 lg:grid-cols-3">
            {projects.slice(0, 6).map((project) => (
              <Link key={project.id} to={`/projects/${project.id}`}>
                <Card className="h-full transition-colors hover:bg-muted/50">
                  <CardHeader className="pb-2">
                    <div className="flex items-start justify-between">
                      <CardTitle className="text-base font-mono">{project.name}</CardTitle>
                      <Badge variant="secondary" className="text-xs">
                        {project.database_count || 0} DB{(project.database_count || 0) !== 1 && "s"}
                      </Badge>
                    </div>
                    {project.description && (
                      <CardDescription className="line-clamp-2">
                        {project.description}
                      </CardDescription>
                    )}
                  </CardHeader>
                  <CardContent>
                    <p className="text-xs text-muted-foreground">
                      {format(new Date(project.created_at), "MMM d, yyyy")}
                    </p>
                  </CardContent>
                </Card>
              </Link>
            ))}
          </div>
        )}
      </div>

      <CreateProjectDialog open={createOpen} onOpenChange={setCreateOpen} />
    </div>
  );
}
