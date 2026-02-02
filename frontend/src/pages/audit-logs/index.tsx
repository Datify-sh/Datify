import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardContent } from "@/components/ui/card";
import {
  Pagination,
  PaginationContent,
  PaginationEllipsis,
  PaginationItem,
  PaginationLink,
  PaginationNext,
  PaginationPrevious,
} from "@/components/ui/pagination";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { Skeleton } from "@/components/ui/skeleton";
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table";
import { auditLogsApi, getErrorMessage } from "@/lib/api";
import type { AuditAction, AuditEntityType, AuditLogResponse, AuditStatus } from "@/lib/api/types";
import { NoteIcon, RefreshIcon } from "@hugeicons/core-free-icons";
import { HugeiconsIcon } from "@hugeicons/react";
import { useQuery } from "@tanstack/react-query";
import { formatDistanceToNow, parseISO } from "date-fns";
import * as React from "react";

const ACTION_LABELS: Record<AuditAction, string> = {
  login: "Login",
  logout: "Logout",
  register: "Register",
  create_project: "Create Project",
  update_project: "Update Project",
  delete_project: "Delete Project",
  create_database: "Create Database",
  update_database: "Update Database",
  delete_database: "Delete Database",
  start_database: "Start Database",
  stop_database: "Stop Database",
  change_password: "Change Password",
  create_branch: "Create Branch",
  sync_from_parent: "Sync from Parent",
  execute_query: "Execute Query",
};

const ENTITY_LABELS: Record<AuditEntityType, string> = {
  user: "User",
  project: "Project",
  database: "Database",
  branch: "Branch",
  query: "Query",
};

function ActionBadge({ action }: { action: AuditAction }) {
  const isDestructive = action.includes("delete");
  const isCreate = action.includes("create") || action === "register";

  return (
    <Badge
      variant={isDestructive ? "destructive" : isCreate ? "default" : "secondary"}
      className="font-mono text-xs"
    >
      {ACTION_LABELS[action] || action}
    </Badge>
  );
}

function StatusBadge({ status }: { status: AuditStatus }) {
  return (
    <Badge variant={status === "success" ? "outline" : "destructive"} className="text-xs">
      {status}
    </Badge>
  );
}

function AuditLogRow({ log }: { log: AuditLogResponse }) {
  const createdAt = parseISO(
    log.created_at.includes("T") ? log.created_at : `${log.created_at.replace(" ", "T")}Z`,
  );

  return (
    <TableRow>
      <TableCell className="text-muted-foreground text-sm">
        {formatDistanceToNow(createdAt, { addSuffix: true })}
      </TableCell>
      <TableCell>
        <div className="flex flex-col">
          <span className="font-medium text-sm">{log.user_email || "Unknown"}</span>
          <span className="text-xs text-muted-foreground">
            {log.user_id ? `ID ${log.user_id.slice(0, 8)}` : "No user ID"}
          </span>
        </div>
      </TableCell>
      <TableCell>
        <ActionBadge action={log.action} />
      </TableCell>
      <TableCell>
        <span className="text-sm">{ENTITY_LABELS[log.entity_type] || log.entity_type}</span>
        {log.entity_id && (
          <span className="text-xs text-muted-foreground ml-1 font-mono">
            ({log.entity_id.slice(0, 8)})
          </span>
        )}
      </TableCell>
      <TableCell>
        <StatusBadge status={log.status} />
      </TableCell>
      <TableCell className="text-muted-foreground text-xs font-mono">
        {log.ip_address || "-"}
      </TableCell>
    </TableRow>
  );
}

type PageItem = { type: "page"; value: number } | { type: "ellipsis"; key: string };

function buildPagination(currentPage: number, totalPages: number): PageItem[] {
  if (totalPages <= 1) return [{ type: "page", value: 1 }];

  if (totalPages <= 7) {
    return Array.from({ length: totalPages }, (_, i) => ({
      type: "page",
      value: i + 1,
    }));
  }

  const items: PageItem[] = [{ type: "page", value: 1 }];
  const start = Math.max(2, currentPage - 1);
  const end = Math.min(totalPages - 1, currentPage + 1);

  if (start > 2) {
    items.push({ type: "ellipsis", key: "start" });
  }

  for (let page = start; page <= end; page += 1) {
    items.push({ type: "page", value: page });
  }

  if (end < totalPages - 1) {
    items.push({ type: "ellipsis", key: "end" });
  }

  items.push({ type: "page", value: totalPages });
  return items;
}

export function AuditLogsPage() {
  const [page, setPage] = React.useState(1);
  const [action, setAction] = React.useState<string>("all");
  const [entityType, setEntityType] = React.useState<string>("all");
  const [status, setStatus] = React.useState<string>("all");

  const { data, isLoading, isFetching, isError, error, refetch } = useQuery({
    queryKey: ["audit-logs", page, action, entityType, status],
    queryFn: () =>
      auditLogsApi.list({
        page,
        page_size: 20,
        action: action !== "all" ? action : undefined,
        entity_type: entityType !== "all" ? entityType : undefined,
        status: status !== "all" ? status : undefined,
      }),
    staleTime: 1000 * 30,
    refetchOnMount: "always",
    refetchOnWindowFocus: true,
    placeholderData: (previousData) => previousData,
  });

  const logs = data?.data || [];
  const pagination = data?.pagination;
  const totalPages = Math.max(1, pagination?.total_pages ?? 1);
  const errorMessage = isError ? getErrorMessage(error, "Failed to load audit logs") : "";
  const isRefreshing = isFetching && !isLoading;
  const pageItems = React.useMemo(() => buildPagination(page, totalPages), [page, totalPages]);

  React.useEffect(() => {
    if (pagination && page > totalPages) {
      setPage(totalPages);
    }
  }, [pagination, page, totalPages]);

  const handleActionChange = (value: string) => {
    setPage(1);
    setAction(value);
  };

  const handleEntityTypeChange = (value: string) => {
    setPage(1);
    setEntityType(value);
  };

  const handleStatusChange = (value: string) => {
    setPage(1);
    setStatus(value);
  };

  const handlePageChange = (nextPage: number) => {
    setPage(Math.min(Math.max(1, nextPage), totalPages));
  };

  return (
    <div className="space-y-6">
      <div className="flex flex-wrap items-start justify-between gap-3">
        <div>
          <h1 className="text-2xl font-bold tracking-tight">Audit Logs</h1>
          <p className="text-muted-foreground">Track user activity and system events</p>
        </div>
        <Button variant="outline" size="sm" onClick={() => refetch()} disabled={isFetching}>
          <HugeiconsIcon
            icon={RefreshIcon}
            className={`size-4 ${isRefreshing ? "animate-spin" : ""}`}
            strokeWidth={2}
          />
          {isRefreshing ? "Refreshing" : "Refresh"}
        </Button>
      </div>

      <div className="flex flex-wrap gap-3">
        <Select value={action} onValueChange={handleActionChange}>
          <SelectTrigger className="w-[180px]">
            <SelectValue placeholder="Filter by action" />
          </SelectTrigger>
          <SelectContent>
            <SelectItem value="all">All Actions</SelectItem>
            <SelectItem value="login">Login</SelectItem>
            <SelectItem value="logout">Logout</SelectItem>
            <SelectItem value="register">Register</SelectItem>
            <SelectItem value="create_project">Create Project</SelectItem>
            <SelectItem value="update_project">Update Project</SelectItem>
            <SelectItem value="delete_project">Delete Project</SelectItem>
            <SelectItem value="create_database">Create Database</SelectItem>
            <SelectItem value="update_database">Update Database</SelectItem>
            <SelectItem value="delete_database">Delete Database</SelectItem>
            <SelectItem value="start_database">Start Database</SelectItem>
            <SelectItem value="stop_database">Stop Database</SelectItem>
            <SelectItem value="change_password">Change Password</SelectItem>
            <SelectItem value="create_branch">Create Branch</SelectItem>
            <SelectItem value="sync_from_parent">Sync from Parent</SelectItem>
            <SelectItem value="execute_query">Execute Query</SelectItem>
          </SelectContent>
        </Select>

        <Select value={entityType} onValueChange={handleEntityTypeChange}>
          <SelectTrigger className="w-[150px]">
            <SelectValue placeholder="Entity type" />
          </SelectTrigger>
          <SelectContent>
            <SelectItem value="all">All Types</SelectItem>
            <SelectItem value="user">User</SelectItem>
            <SelectItem value="project">Project</SelectItem>
            <SelectItem value="database">Database</SelectItem>
            <SelectItem value="branch">Branch</SelectItem>
            <SelectItem value="query">Query</SelectItem>
          </SelectContent>
        </Select>

        <Select value={status} onValueChange={handleStatusChange}>
          <SelectTrigger className="w-[130px]">
            <SelectValue placeholder="Status" />
          </SelectTrigger>
          <SelectContent>
            <SelectItem value="all">All Status</SelectItem>
            <SelectItem value="success">Success</SelectItem>
            <SelectItem value="failure">Failure</SelectItem>
          </SelectContent>
        </Select>
      </div>

      {isLoading ? (
        <Card>
          <CardContent className="p-0">
            <Table>
              <TableHeader>
                <TableRow>
                  <TableHead className="w-[140px]">Time</TableHead>
                  <TableHead className="w-[200px]">User</TableHead>
                  <TableHead className="w-[150px]">Action</TableHead>
                  <TableHead className="w-[150px]">Entity</TableHead>
                  <TableHead className="w-[100px]">Status</TableHead>
                  <TableHead>IP Address</TableHead>
                </TableRow>
              </TableHeader>
              <TableBody>
                {Array.from({ length: 10 }).map((_, i) => (
                  <TableRow key={i}>
                    <TableCell>
                      <Skeleton className="h-4 w-24" />
                    </TableCell>
                    <TableCell>
                      <Skeleton className="h-8 w-32" />
                    </TableCell>
                    <TableCell>
                      <Skeleton className="h-5 w-24" />
                    </TableCell>
                    <TableCell>
                      <Skeleton className="h-4 w-20" />
                    </TableCell>
                    <TableCell>
                      <Skeleton className="h-5 w-16" />
                    </TableCell>
                    <TableCell>
                      <Skeleton className="h-4 w-28" />
                    </TableCell>
                  </TableRow>
                ))}
              </TableBody>
            </Table>
          </CardContent>
        </Card>
      ) : isError ? (
        <Card>
          <CardContent className="flex flex-col items-center justify-center py-16">
            <div className="flex size-12 items-center justify-center rounded-full bg-muted">
              <HugeiconsIcon
                icon={NoteIcon}
                className="size-6 text-muted-foreground"
                strokeWidth={2}
              />
            </div>
            <h3 className="mt-4 text-lg font-semibold">Unable to load audit logs</h3>
            <p className="mt-1 text-sm text-muted-foreground">{errorMessage}</p>
            <Button className="mt-4" variant="outline" size="sm" onClick={() => refetch()}>
              Retry
            </Button>
          </CardContent>
        </Card>
      ) : logs.length === 0 ? (
        <Card>
          <CardContent className="flex flex-col items-center justify-center py-16">
            <div className="flex size-12 items-center justify-center rounded-full bg-muted">
              <HugeiconsIcon
                icon={NoteIcon}
                className="size-6 text-muted-foreground"
                strokeWidth={2}
              />
            </div>
            <h3 className="mt-4 text-lg font-semibold">No audit logs found</h3>
            <p className="mt-1 text-sm text-muted-foreground">
              {action !== "all" || entityType !== "all" || status !== "all"
                ? "Try adjusting your filters"
                : "Activity will appear here as users perform actions"}
            </p>
          </CardContent>
        </Card>
      ) : (
        <>
          <Card>
            <CardContent className="p-0">
              <Table>
                <TableHeader>
                  <TableRow>
                    <TableHead className="w-[140px]">Time</TableHead>
                    <TableHead className="w-[200px]">User</TableHead>
                    <TableHead className="w-[150px]">Action</TableHead>
                    <TableHead className="w-[150px]">Entity</TableHead>
                    <TableHead className="w-[100px]">Status</TableHead>
                    <TableHead>IP Address</TableHead>
                  </TableRow>
                </TableHeader>
                <TableBody>
                  {logs.map((log) => (
                    <AuditLogRow key={log.id} log={log} />
                  ))}
                </TableBody>
              </Table>
            </CardContent>
          </Card>

          {pagination && (
            <div className="flex items-center justify-between">
              <p className="text-sm text-muted-foreground">
                Page {pagination.page} of {pagination.total_pages} ({pagination.total_items} total)
              </p>
              {totalPages > 1 && (
                <Pagination>
                  <PaginationContent>
                    <PaginationItem>
                      <PaginationPrevious
                        href="#"
                        onClick={(event) => {
                          event.preventDefault();
                          if (pagination.has_prev) {
                            handlePageChange(page - 1);
                          }
                        }}
                        aria-disabled={!pagination.has_prev}
                        className={!pagination.has_prev ? "pointer-events-none opacity-50" : ""}
                      />
                    </PaginationItem>
                    {pageItems.map((item) => (
                      <PaginationItem key={item.type === "page" ? item.value : item.key}>
                        {item.type === "page" ? (
                          <PaginationLink
                            href="#"
                            isActive={item.value === page}
                            onClick={(event) => {
                              event.preventDefault();
                              handlePageChange(item.value);
                            }}
                          >
                            {item.value}
                          </PaginationLink>
                        ) : (
                          <PaginationEllipsis />
                        )}
                      </PaginationItem>
                    ))}
                    <PaginationItem>
                      <PaginationNext
                        href="#"
                        onClick={(event) => {
                          event.preventDefault();
                          if (pagination.has_next) {
                            handlePageChange(page + 1);
                          }
                        }}
                        aria-disabled={!pagination.has_next}
                        className={!pagination.has_next ? "pointer-events-none opacity-50" : ""}
                      />
                    </PaginationItem>
                  </PaginationContent>
                </Pagination>
              )}
            </div>
          )}
        </>
      )}
    </div>
  );
}
