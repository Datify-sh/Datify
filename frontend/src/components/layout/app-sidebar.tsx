import { CreateProjectDialog } from "@/components/create-project-dialog";
import { Avatar, AvatarFallback } from "@/components/ui/avatar";
import { Collapsible, CollapsibleContent, CollapsibleTrigger } from "@/components/ui/collapsible";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import {
  Sidebar,
  SidebarContent,
  SidebarFooter,
  SidebarGroup,
  SidebarGroupAction,
  SidebarGroupContent,
  SidebarGroupLabel,
  SidebarHeader,
  SidebarMenu,
  SidebarMenuButton,
  SidebarMenuItem,
  SidebarMenuSub,
  SidebarMenuSubButton,
  SidebarMenuSubItem,
} from "@/components/ui/sidebar";
import { useAuth } from "@/contexts/auth-context";
import { databasesApi, projectsApi } from "@/lib/api";
import { cn } from "@/lib/utils";
import {
  Add01Icon,
  ArrowRight01Icon,
  ComputerIcon,
  Folder01Icon,
  GitBranchIcon,
  Home01Icon,
  Logout01Icon,
  Moon02Icon,
  Settings01Icon,
  Sun03Icon,
} from "@hugeicons/core-free-icons";
import { HugeiconsIcon } from "@hugeicons/react";
import { useQuery } from "@tanstack/react-query";
import { useTheme } from "next-themes";
import * as React from "react";
import { NavLink, useLocation } from "react-router-dom";

const mainNavItems = [
  {
    title: "Dashboard",
    href: "/",
    icon: Home01Icon,
  },
  {
    title: "Projects",
    href: "/projects",
    icon: Folder01Icon,
  },
];

const statusColors: Record<string, string> = {
  running: "bg-green-500",
  stopped: "bg-neutral-400",
  starting: "bg-yellow-500 animate-pulse",
  stopping: "bg-yellow-500 animate-pulse",
  error: "bg-red-500",
};

export function AppSidebar({ variant = "inset" }: { variant?: "sidebar" | "floating" | "inset" }) {
  const { user, logout } = useAuth();
  const { theme, setTheme } = useTheme();
  const location = useLocation();
  const [createOpen, setCreateOpen] = React.useState(false);

  const { data: projectsData } = useQuery({
    queryKey: ["projects"],
    queryFn: () => projectsApi.list({ limit: 20 }),
    staleTime: 1000 * 60,
  });

  const projects = projectsData?.data || [];

  return (
    <>
      <Sidebar variant={variant} collapsible="icon" className="border-none">
        <SidebarHeader className="h-14 justify-center">
          <div className="flex items-center px-2">
            <span className="text-lg font-bold tracking-tight group-data-[collapsible=icon]:hidden">
              Datify
            </span>
          </div>
        </SidebarHeader>

        <SidebarContent>
          <SidebarGroup>
            <SidebarGroupLabel>Navigation</SidebarGroupLabel>
            <SidebarGroupContent>
              <SidebarMenu>
                {mainNavItems.map((item) => (
                  <SidebarMenuItem key={item.href}>
                    <NavLink to={item.href} end={item.href === "/"}>
                      {({ isActive: navIsActive }) => (
                        <SidebarMenuButton isActive={navIsActive} tooltip={item.title}>
                          <HugeiconsIcon icon={item.icon} className="size-[18px]" strokeWidth={2} />
                          <span>{item.title}</span>
                        </SidebarMenuButton>
                      )}
                    </NavLink>
                  </SidebarMenuItem>
                ))}
              </SidebarMenu>
            </SidebarGroupContent>
          </SidebarGroup>

          {projects.length > 0 && (
            <SidebarGroup>
              <SidebarGroupLabel>Projects</SidebarGroupLabel>
              <SidebarGroupAction onClick={() => setCreateOpen(true)} title="New Project">
                <HugeiconsIcon icon={Add01Icon} className="size-4" strokeWidth={2} />
              </SidebarGroupAction>
              <SidebarGroupContent>
                <SidebarMenu>
                  {projects.slice(0, 5).map((project) => (
                    <ProjectItem
                      key={project.id}
                      project={project}
                      currentPath={location.pathname}
                    />
                  ))}
                </SidebarMenu>
              </SidebarGroupContent>
            </SidebarGroup>
          )}

          <SidebarGroup>
            <SidebarGroupLabel>Actions</SidebarGroupLabel>
            <SidebarGroupContent>
              <SidebarMenu>
                <SidebarMenuItem>
                  <SidebarMenuButton onClick={() => setCreateOpen(true)} tooltip="New Project">
                    <HugeiconsIcon icon={Add01Icon} className="size-[18px]" strokeWidth={2} />
                    <span>New Project</span>
                  </SidebarMenuButton>
                </SidebarMenuItem>
              </SidebarMenu>
            </SidebarGroupContent>
          </SidebarGroup>
        </SidebarContent>

        <SidebarFooter>
          <SidebarMenu>
            <SidebarMenuItem>
              <SidebarMenuButton
                onClick={() => {
                  const nextTheme =
                    theme === "system" ? "light" : theme === "light" ? "dark" : "system";
                  setTheme(nextTheme);
                }}
                tooltip={`Theme: ${theme === "system" ? "System" : theme === "light" ? "Light" : "Dark"}`}
              >
                <HugeiconsIcon
                  icon={
                    theme === "system" ? ComputerIcon : theme === "light" ? Sun03Icon : Moon02Icon
                  }
                  className="size-[18px]"
                  strokeWidth={2}
                />
                <span>{theme === "system" ? "System" : theme === "light" ? "Light" : "Dark"}</span>
              </SidebarMenuButton>
            </SidebarMenuItem>
            <SidebarMenuItem>
              <DropdownMenu>
                <DropdownMenuTrigger asChild>
                  <SidebarMenuButton className="h-auto py-2">
                    <Avatar className="size-7 rounded-md">
                      <AvatarFallback className="rounded-md text-xs font-medium">
                        {user?.name?.charAt(0).toUpperCase() || "U"}
                      </AvatarFallback>
                    </Avatar>
                    <div className="flex flex-col items-start gap-0.5 group-data-[collapsible=icon]:hidden">
                      <span className="text-sm font-medium">{user?.name}</span>
                      <span className="text-xs text-muted-foreground">{user?.email}</span>
                    </div>
                  </SidebarMenuButton>
                </DropdownMenuTrigger>
                <DropdownMenuContent side="top" align="start" className="w-56">
                  <DropdownMenuItem disabled>
                    <HugeiconsIcon icon={Settings01Icon} className="size-4" strokeWidth={2} />
                    Settings
                  </DropdownMenuItem>
                  <DropdownMenuSeparator />
                  <DropdownMenuItem
                    onClick={logout}
                    className="text-destructive focus:text-destructive"
                  >
                    <HugeiconsIcon icon={Logout01Icon} className="size-4" strokeWidth={2} />
                    Log out
                  </DropdownMenuItem>
                </DropdownMenuContent>
              </DropdownMenu>
            </SidebarMenuItem>
          </SidebarMenu>
        </SidebarFooter>
      </Sidebar>

      <CreateProjectDialog open={createOpen} onOpenChange={setCreateOpen} />
    </>
  );
}

function ProjectItem({
  project,
  currentPath,
}: {
  project: { id: string; name: string; database_count?: number };
  currentPath: string;
}) {
  const isProjectActive = currentPath === `/projects/${project.id}`;
  const [isOpen, setIsOpen] = React.useState(
    isProjectActive || currentPath.includes("/databases/"),
  );

  const { data: databasesData } = useQuery({
    queryKey: ["databases", project.id],
    queryFn: () => databasesApi.list(project.id, { limit: 20 }),
    enabled: isOpen,
    staleTime: 1000 * 30,
  });

  const databases = databasesData?.data || [];
  const mainDatabases = databases.filter((db) => !db.branch?.parent_id);
  const branchesByParent = React.useMemo(() => {
    const map = new Map<string, typeof databases>();
    for (const db of databases) {
      if (db.branch?.parent_id) {
        const existing = map.get(db.branch.parent_id) || [];
        map.set(db.branch.parent_id, [...existing, db]);
      }
    }
    return map;
  }, [databases]);

  return (
    <Collapsible open={isOpen} onOpenChange={setIsOpen}>
      <SidebarMenuItem>
        <CollapsibleTrigger asChild>
          <SidebarMenuButton tooltip={project.name}>
            <HugeiconsIcon
              icon={ArrowRight01Icon}
              className={cn("size-4 transition-transform", isOpen && "rotate-90")}
              strokeWidth={2}
            />
            <span className="truncate">{project.name}</span>
            {(project.database_count ?? 0) > 0 && (
              <span className="ml-auto text-xs text-muted-foreground">
                {project.database_count}
              </span>
            )}
          </SidebarMenuButton>
        </CollapsibleTrigger>
        <CollapsibleContent>
          <SidebarMenuSub>
            {databases.length === 0 ? (
              <SidebarMenuSubItem>
                <span className="text-xs text-muted-foreground px-2 py-1">No databases</span>
              </SidebarMenuSubItem>
            ) : (
              mainDatabases.map((db) => (
                <DatabaseItem
                  key={db.id}
                  database={db}
                  branches={branchesByParent.get(db.id) || []}
                  currentPath={currentPath}
                />
              ))
            )}
          </SidebarMenuSub>
        </CollapsibleContent>
      </SidebarMenuItem>
    </Collapsible>
  );
}

function DatabaseItem({
  database,
  branches,
  currentPath,
}: {
  database: {
    id: string;
    name: string;
    status: string;
    database_type?: string;
    branch?: { name: string };
  };
  branches: Array<{
    id: string;
    name: string;
    status: string;
    database_type?: string;
    branch?: { name: string };
  }>;
  currentPath: string;
}) {
  const hasBranches = branches.length > 0;
  const isDbActive = currentPath === `/databases/${database.id}`;
  const isBranchActive = branches.some((b) => currentPath === `/databases/${b.id}`);
  const [isOpen, setIsOpen] = React.useState(isDbActive || isBranchActive);

  if (!hasBranches) {
    return (
      <SidebarMenuSubItem>
        <NavLink to={`/databases/${database.id}`}>
          {({ isActive }) => (
            <SidebarMenuSubButton isActive={isActive}>
              <span
                className={cn(
                  "size-2 rounded-full",
                  statusColors[database.status] || "bg-neutral-400",
                )}
              />
              <span className="truncate font-mono text-xs">{database.name}</span>
              <span className="ml-auto text-[9px] text-muted-foreground uppercase">
                {database.database_type === "valkey"
                  ? "valkey"
                  : database.database_type === "redis"
                    ? "redis"
                    : "pg"}
              </span>
            </SidebarMenuSubButton>
          )}
        </NavLink>
      </SidebarMenuSubItem>
    );
  }

  return (
    <SidebarMenuSubItem>
      <Collapsible open={isOpen} onOpenChange={setIsOpen}>
        <div className="flex items-center">
          <CollapsibleTrigger className="p-1 hover:bg-muted rounded">
            <HugeiconsIcon
              icon={ArrowRight01Icon}
              className={cn("size-3 transition-transform", isOpen && "rotate-90")}
              strokeWidth={2}
            />
          </CollapsibleTrigger>
          <NavLink to={`/databases/${database.id}`} className="flex-1">
            {({ isActive }) => (
              <SidebarMenuSubButton isActive={isActive} className="flex-1">
                <span
                  className={cn(
                    "size-2 rounded-full",
                    statusColors[database.status] || "bg-neutral-400",
                  )}
                />
                <span className="truncate font-mono text-xs">{database.name}</span>
                <span className="ml-auto flex items-center gap-1.5">
                  <span className="text-[9px] text-muted-foreground uppercase">
                    {database.database_type === "valkey"
                      ? "valkey"
                      : database.database_type === "redis"
                        ? "redis"
                        : "pg"}
                  </span>
                  <span className="text-[10px] text-muted-foreground">{branches.length}</span>
                </span>
              </SidebarMenuSubButton>
            )}
          </NavLink>
        </div>
        <CollapsibleContent>
          <div className="ml-4 border-l border-border pl-2 space-y-0.5">
            {branches.map((branch) => (
              <NavLink key={branch.id} to={`/databases/${branch.id}`}>
                {({ isActive }) => (
                  <SidebarMenuSubButton isActive={isActive} className="h-7">
                    <HugeiconsIcon
                      icon={GitBranchIcon}
                      className="size-3 text-muted-foreground"
                      strokeWidth={2}
                    />
                    <span
                      className={cn(
                        "size-1.5 rounded-full",
                        statusColors[branch.status] || "bg-neutral-400",
                      )}
                    />
                    <span className="truncate font-mono text-[11px]">
                      {branch.branch?.name || branch.name}
                    </span>
                    <span className="ml-auto text-[9px] text-muted-foreground uppercase">
                      {branch.database_type === "valkey"
                        ? "valkey"
                        : branch.database_type === "redis"
                          ? "redis"
                          : "pg"}
                    </span>
                  </SidebarMenuSubButton>
                )}
              </NavLink>
            ))}
          </div>
        </CollapsibleContent>
      </Collapsible>
    </SidebarMenuSubItem>
  );
}
