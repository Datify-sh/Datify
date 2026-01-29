import * as React from "react";
import { Link } from "react-router-dom";
import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import { HugeiconsIcon } from "@hugeicons/react";
import {
  Folder01Icon,
  Database01Icon,
  Add01Icon,
  Search01Icon,
  MoreHorizontalIcon,
  Delete01Icon,
  Edit01Icon,
} from "@hugeicons/core-free-icons";
import { projectsApi } from "@/lib/api";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Badge } from "@/components/ui/badge";
import { Skeleton } from "@/components/ui/skeleton";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import {
  AlertDialog,
  AlertDialogAction,
  AlertDialogCancel,
  AlertDialogContent,
  AlertDialogDescription,
  AlertDialogFooter,
  AlertDialogHeader,
  AlertDialogTitle,
} from "@/components/ui/alert-dialog";
import { CreateProjectDialog } from "@/components/create-project-dialog";
import { toast } from "sonner";
import { format } from "date-fns";

export function ProjectsListPage() {
  const [search, setSearch] = React.useState("");
  const [deleteProject, setDeleteProject] = React.useState<{ id: string; name: string } | null>(
    null,
  );
  const [createOpen, setCreateOpen] = React.useState(false);
  const queryClient = useQueryClient();

  const { data: projectsData, isLoading } = useQuery({
    queryKey: ["projects"],
    queryFn: () => projectsApi.list({ limit: 50 }),
    staleTime: 1000 * 30,
  });

  const deleteMutation = useMutation({
    mutationFn: (id: string) => projectsApi.delete(id),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["projects"] });
      queryClient.invalidateQueries({ queryKey: ["all-databases-for-stats"] });
      toast.success("Project deleted");
      setDeleteProject(null);
    },
    onError: () => toast.error("Failed to delete project"),
  });

  const searchLower = search.toLowerCase();
  const filteredProjects = React.useMemo(() => {
    const projects = projectsData?.data || [];
    return projects.filter((p) => p.name.toLowerCase().includes(searchLower));
  }, [projectsData?.data, searchLower]);

  return (
    <div className="space-y-6">
      {/* Header */}
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-2xl font-bold tracking-tight">Projects</h1>
          <p className="text-muted-foreground">Manage your database projects</p>
        </div>
        <Button onClick={() => setCreateOpen(true)}>
          <HugeiconsIcon icon={Add01Icon} className="size-4" strokeWidth={2} />
          New Project
        </Button>
      </div>

      {/* Search */}
      <div className="relative max-w-sm">
        <HugeiconsIcon
          icon={Search01Icon}
          className="absolute left-3 top-1/2 size-4 -translate-y-1/2 text-muted-foreground"
          strokeWidth={2}
        />
        <Input
          placeholder="Search projects..."
          value={search}
          onChange={(e) => setSearch(e.target.value)}
          className="pl-10"
        />
      </div>

      {/* Projects grid */}
      {isLoading ? (
        <div className="grid gap-4 sm:grid-cols-2 lg:grid-cols-3">
          {[1, 2, 3, 4, 5, 6].map((i) => (
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
      ) : filteredProjects.length === 0 ? (
        <Card>
          <CardContent className="flex flex-col items-center justify-center py-16">
            <div className="flex size-12 items-center justify-center rounded-full bg-muted">
              <HugeiconsIcon
                icon={Folder01Icon}
                className="size-6 text-muted-foreground"
                strokeWidth={2}
              />
            </div>
            <h3 className="mt-4 text-lg font-semibold">
              {search ? "No projects found" : "No projects yet"}
            </h3>
            <p className="mt-1 text-sm text-muted-foreground">
              {search
                ? "Try adjusting your search terms"
                : "Create your first project to get started"}
            </p>
            {!search && (
              <Button onClick={() => setCreateOpen(true)} className="mt-4">
                <HugeiconsIcon icon={Add01Icon} className="size-4" strokeWidth={2} />
                Create Project
              </Button>
            )}
          </CardContent>
        </Card>
      ) : (
        <div className="grid gap-4 sm:grid-cols-2 lg:grid-cols-3">
          {filteredProjects.map((project) => (
            <Card key={project.id} className="group relative transition-colors hover:bg-muted/50">
              <Link to={`/projects/${project.id}`} className="absolute inset-0 z-0" />
              <CardHeader className="pb-2">
                <div className="flex items-start justify-between">
                  <CardTitle className="text-base font-mono">{project.name}</CardTitle>
                  <DropdownMenu>
                    <DropdownMenuTrigger asChild>
                      <Button
                        variant="ghost"
                        size="icon-sm"
                        className="relative z-10 opacity-0 group-hover:opacity-100"
                      >
                        <HugeiconsIcon icon={MoreHorizontalIcon} className="size-4" strokeWidth={2} />
                      </Button>
                    </DropdownMenuTrigger>
                    <DropdownMenuContent align="end">
                      <DropdownMenuItem asChild>
                        <Link to={`/projects/${project.id}`}>
                          <HugeiconsIcon icon={Edit01Icon} className="size-3.5" strokeWidth={2} />
                          Open
                        </Link>
                      </DropdownMenuItem>
                      <DropdownMenuSeparator />
                      <DropdownMenuItem
                        className="text-destructive focus:text-destructive"
                        onClick={() => setDeleteProject({ id: project.id, name: project.name })}
                      >
                        <HugeiconsIcon icon={Delete01Icon} className="size-3.5" strokeWidth={2} />
                        Delete
                      </DropdownMenuItem>
                    </DropdownMenuContent>
                  </DropdownMenu>
                </div>
                {project.description && (
                  <CardDescription className="line-clamp-2">{project.description}</CardDescription>
                )}
              </CardHeader>
              <CardContent>
                <div className="flex items-center justify-between text-sm text-muted-foreground">
                  <Badge variant="secondary">
                    <HugeiconsIcon icon={Database01Icon} className="size-3" strokeWidth={2} />
                    {(project as { database_count?: number }).database_count || 0} databases
                  </Badge>
                  <span className="text-xs">
                    {format(new Date(project.created_at), "MMM d, yyyy")}
                  </span>
                </div>
              </CardContent>
            </Card>
          ))}
        </div>
      )}

      {/* Create Project Dialog */}
      <CreateProjectDialog open={createOpen} onOpenChange={setCreateOpen} />

      {/* Delete Dialog */}
      <AlertDialog open={!!deleteProject} onOpenChange={() => setDeleteProject(null)}>
        <AlertDialogContent>
          <AlertDialogHeader>
            <AlertDialogTitle>Delete Project</AlertDialogTitle>
            <AlertDialogDescription>
              Are you sure you want to delete "{deleteProject?.name}"? This will also delete all
              databases in this project. This action cannot be undone.
            </AlertDialogDescription>
          </AlertDialogHeader>
          <AlertDialogFooter>
            <AlertDialogCancel>Cancel</AlertDialogCancel>
            <AlertDialogAction
              onClick={() => deleteProject && deleteMutation.mutate(deleteProject.id)}
              className="bg-destructive text-destructive-foreground hover:bg-destructive/90"
            >
              Delete
            </AlertDialogAction>
          </AlertDialogFooter>
        </AlertDialogContent>
      </AlertDialog>
    </div>
  );
}
