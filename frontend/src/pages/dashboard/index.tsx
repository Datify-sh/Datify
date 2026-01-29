import * as React from "react";
import { Link } from "react-router-dom";
import { useQuery } from "@tanstack/react-query";
import { HugeiconsIcon } from "@hugeicons/react";
import {
  Folder01Icon,
  Database01Icon,
  ArrowRight01Icon,
  Add01Icon,
  PlayIcon,
  CheckmarkCircle01Icon,
} from "@hugeicons/core-free-icons";
import { projectsApi, databasesApi } from "@/lib/api";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Skeleton } from "@/components/ui/skeleton";
import { Badge } from "@/components/ui/badge";
import { CreateProjectDialog } from "@/components/create-project-dialog";
import { useAuth } from "@/contexts/auth-context";
import { format } from "date-fns";

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
    refetchInterval: 10000,
  });

  const runningDatabases = allDatabasesData?.filter((db) => db.status === "running").length ?? 0;
  const isLoading = projectsLoading;

  return (
    <div className="space-y-8">
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-3xl font-bold tracking-tight">Databases</h1>
          <p className="text-muted-foreground mt-1">
            Welcome back, {user?.name?.split(" ")[0]}
          </p>
        </div>
        <Button onClick={() => setCreateOpen(true)}>
          <HugeiconsIcon icon={Add01Icon} className="size-4" strokeWidth={2} />
          New Project
        </Button>
      </div>

      <div className="grid gap-4 sm:grid-cols-2 lg:grid-cols-4">
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
              <div className="text-2xl font-bold">{runningDatabases}</div>
            )}
          </CardContent>
        </Card>

        <Card>
          <CardHeader className="flex flex-row items-center justify-between pb-2">
            <CardTitle className="text-sm font-medium">Status</CardTitle>
            <HugeiconsIcon
              icon={CheckmarkCircle01Icon}
              className="size-4 text-muted-foreground"
              strokeWidth={2}
            />
          </CardHeader>
          <CardContent>
            <div className="flex items-center gap-2">
              <div className="size-2 rounded-full bg-green-500" />
              <span className="text-sm font-medium">Operational</span>
            </div>
          </CardContent>
        </Card>
      </div>

      <div className="grid gap-4 sm:grid-cols-2">
        <Card
          className="cursor-pointer transition-colors hover:bg-muted/50"
          onClick={() => setCreateOpen(true)}
        >
          <CardHeader>
            <div className="flex items-center gap-3">
              <div className="flex size-10 items-center justify-center rounded-lg bg-primary/10">
                <HugeiconsIcon icon={Add01Icon} className="size-5 text-primary" strokeWidth={2} />
              </div>
              <div>
                <CardTitle className="text-base">Create Project</CardTitle>
                <CardDescription>Start a new database project</CardDescription>
              </div>
            </div>
          </CardHeader>
        </Card>

        <Card className="cursor-pointer transition-colors hover:bg-muted/50">
          <Link to="/projects" className="block">
            <CardHeader>
              <div className="flex items-center gap-3">
                <div className="flex size-10 items-center justify-center rounded-lg bg-primary/10">
                  <HugeiconsIcon
                    icon={Folder01Icon}
                    className="size-5 text-primary"
                    strokeWidth={2}
                  />
                </div>
                <div>
                  <CardTitle className="text-base">View Projects</CardTitle>
                  <CardDescription>Browse all your projects</CardDescription>
                </div>
              </div>
            </CardHeader>
          </Link>
        </Card>
      </div>

      <div className="space-y-4">
        <div className="flex items-center justify-between">
          <h2 className="text-xl font-semibold">Recent Projects</h2>
          <Button variant="ghost" size="sm" asChild>
            <Link to="/projects">
              View all
              <HugeiconsIcon icon={ArrowRight01Icon} className="size-4" strokeWidth={2} />
            </Link>
          </Button>
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
            <CardContent className="flex flex-col items-center justify-center py-12">
              <div className="flex size-12 items-center justify-center rounded-full bg-muted">
                <HugeiconsIcon
                  icon={Folder01Icon}
                  className="size-6 text-muted-foreground"
                  strokeWidth={2}
                />
              </div>
              <h3 className="mt-4 text-lg font-semibold">No projects yet</h3>
              <p className="mt-1 text-sm text-muted-foreground">
                Create your first project to get started
              </p>
              <Button onClick={() => setCreateOpen(true)} className="mt-4">
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
                  <CardHeader>
                    <div className="flex items-start justify-between">
                      <CardTitle className="text-base font-mono">{project.name}</CardTitle>
                      <Badge variant="secondary">{project.database_count || 0} DBs</Badge>
                    </div>
                    {project.description && (
                      <CardDescription className="line-clamp-2">
                        {project.description}
                      </CardDescription>
                    )}
                  </CardHeader>
                  <CardContent>
                    <p className="text-xs text-muted-foreground">
                      Created {format(new Date(project.created_at), "MMM d, yyyy")}
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
