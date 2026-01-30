import {
  Breadcrumb,
  BreadcrumbItem,
  BreadcrumbLink,
  BreadcrumbList,
  BreadcrumbPage,
  BreadcrumbSeparator,
} from "@/components/ui/breadcrumb";
import { SidebarInset, SidebarProvider, SidebarTrigger } from "@/components/ui/sidebar";
import { Spinner } from "@/components/ui/spinner";
import { useAuth } from "@/contexts/auth-context";
import { databasesApi, projectsApi } from "@/lib/api";
import { useQuery } from "@tanstack/react-query";
import * as React from "react";
import { Link, Navigate, Outlet, useLocation } from "react-router-dom";
import { AppSidebar } from "./app-sidebar";

function useBreadcrumbs() {
  const location = useLocation();
  const segments = React.useMemo(
    () => location.pathname.split("/").filter(Boolean),
    [location.pathname],
  );

  const projectId =
    segments[0] === "projects" && segments[1] && segments[1] !== "new" ? segments[1] : null;
  const databaseId = segments[0] === "databases" && segments[1] ? segments[1] : null;

  const { data: project } = useQuery({
    queryKey: ["project", projectId],
    queryFn: () => (projectId ? projectsApi.get(projectId) : Promise.reject()),
    enabled: !!projectId,
    staleTime: 1000 * 60 * 5,
  });

  const { data: database } = useQuery({
    queryKey: ["database", databaseId],
    queryFn: () => (databaseId ? databasesApi.get(databaseId) : Promise.reject()),
    enabled: !!databaseId,
    staleTime: 1000 * 60,
    retry: 3,
    retryDelay: 1000,
  });

  const { data: dbProject } = useQuery({
    queryKey: ["project", database?.project_id],
    queryFn: () => (database?.project_id ? projectsApi.get(database.project_id) : Promise.reject()),
    enabled: !!database?.project_id,
    staleTime: 1000 * 60 * 5,
  });

  return React.useMemo(() => {
    const crumbs: { label: string; href?: string }[] = [];

    if (location.pathname === "/") {
      crumbs.push({ label: "Dashboard" });
    } else if (segments[0] === "projects") {
      crumbs.push({ label: "Projects", href: "/projects" });
      if (segments[1] === "new") {
        crumbs.push({ label: "New Project" });
      } else if (project) {
        crumbs.push({ label: project.name });
      }
    } else if (segments[0] === "databases" && database) {
      crumbs.push({ label: "Projects", href: "/projects" });
      if (dbProject) {
        crumbs.push({ label: dbProject.name, href: `/projects/${dbProject.id}` });
      }
      crumbs.push({ label: database.name });
    } else if (segments[0] === "tables") {
      crumbs.push({ label: "Table Explorer" });
    } else if (segments[0] === "sql-editor") {
      crumbs.push({ label: "SQL Editor" });
    }

    return crumbs;
  }, [location.pathname, segments, project, database, dbProject]);
}

export function DashboardLayout() {
  const { isAuthenticated, isLoading } = useAuth();
  const breadcrumbs = useBreadcrumbs();

  if (isLoading) {
    return (
      <div className="flex h-screen w-full items-center justify-center">
        <Spinner className="size-8" />
      </div>
    );
  }

  if (!isAuthenticated) {
    return <Navigate to="/login" replace />;
  }

  return (
    <SidebarProvider
      style={
        {
          "--sidebar-width": "calc(var(--spacing) * 64)",
        } as React.CSSProperties
      }
    >
      <AppSidebar variant="inset" />
      <SidebarInset>
        <header className="flex h-12 shrink-0 items-center gap-2 px-6">
          <SidebarTrigger className="-ml-1" />
          <Breadcrumb>
            <BreadcrumbList>
              {breadcrumbs.map((crumb, index) => (
                <BreadcrumbItem key={`${crumb.label}-${crumb.href ?? "current"}`}>
                  {index > 0 && <BreadcrumbSeparator />}
                  {crumb.href ? (
                    <BreadcrumbLink asChild>
                      <Link to={crumb.href}>{crumb.label}</Link>
                    </BreadcrumbLink>
                  ) : (
                    <BreadcrumbPage>{crumb.label}</BreadcrumbPage>
                  )}
                </BreadcrumbItem>
              ))}
            </BreadcrumbList>
          </Breadcrumb>
        </header>
        <div className="flex flex-1 flex-col overflow-hidden">
          <main className="flex-1 overflow-auto p-6">
            <Outlet />
          </main>
        </div>
      </SidebarInset>
    </SidebarProvider>
  );
}
