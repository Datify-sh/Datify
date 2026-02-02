import {
  AlertDialog,
  AlertDialogAction,
  AlertDialogCancel,
  AlertDialogContent,
  AlertDialogDescription,
  AlertDialogFooter,
  AlertDialogHeader,
  AlertDialogMedia,
  AlertDialogTitle,
} from "@/components/ui/alert-dialog";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader } from "@/components/ui/card";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import { Input } from "@/components/ui/input";
import { Skeleton } from "@/components/ui/skeleton";
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table";
import { useAuth } from "@/contexts/auth-context";
import { adminApi } from "@/lib/api";
import type { UpdateUserRequest, UserResponse } from "@/lib/api/types";
import {
  Delete01Icon,
  MoreHorizontalIcon,
  Search01Icon,
  SecurityCheckIcon,
  UserMultiple02Icon,
} from "@hugeicons/core-free-icons";
import { HugeiconsIcon } from "@hugeicons/react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { format } from "date-fns";
import { ShieldOff } from "lucide-react";
import { useMemo, useState } from "react";
import { toast } from "sonner";

type PendingAction = {
  type: "promote" | "demote" | "delete";
  user: UserResponse;
};

export function AdminUsersPage() {
  const { user: currentUser } = useAuth();
  const queryClient = useQueryClient();
  const [page] = useState(1);
  const [search, setSearch] = useState("");
  const [pendingAction, setPendingAction] = useState<PendingAction | null>(null);
  const pageSize = 50;

  const { data, isLoading } = useQuery({
    queryKey: ["admin", "users", page],
    queryFn: async () => {
      const offset = (page - 1) * pageSize;
      return adminApi.listUsers({ limit: pageSize, offset });
    },
  });

  const filteredUsers = useMemo(() => {
    if (!data) return [];
    if (!search) return data;
    const searchLower = search.toLowerCase();
    return data.filter(
      (user) =>
        user.email.toLowerCase().includes(searchLower) ||
        user.id.toLowerCase().includes(searchLower),
    );
  }, [data, search]);

  const updateUserMutation = useMutation({
    mutationFn: async ({ id, data }: { id: string; data: UpdateUserRequest }) => {
      await adminApi.updateUser(id, data);
    },
    onSuccess: () => {
      toast.success("User updated successfully");
      queryClient.invalidateQueries({ queryKey: ["admin", "users"] });
    },
    onError: (error) => {
      toast.error("Failed to update user");
      console.error(error);
    },
  });

  const deleteUserMutation = useMutation({
    mutationFn: async (id: string) => {
      await adminApi.deleteUser(id);
    },
    onSuccess: () => {
      toast.success("User deleted successfully");
      queryClient.invalidateQueries({ queryKey: ["admin", "users"] });
    },
    onError: (error) => {
      toast.error("Failed to delete user");
      console.error(error);
    },
  });

  const isMutating = updateUserMutation.isPending || deleteUserMutation.isPending;

  const confirmAction = () => {
    if (!pendingAction) return;
    const { type, user } = pendingAction;

    if (type === "promote") {
      updateUserMutation.mutate({ id: user.id, data: { role: "admin" } });
    }
    if (type === "demote") {
      updateUserMutation.mutate({ id: user.id, data: { role: "user" } });
    }
    if (type === "delete") {
      deleteUserMutation.mutate(user.id);
    }
    setPendingAction(null);
  };

  const dialogCopy = pendingAction
    ? {
        promote: {
          title: "Make admin?",
          description:
            "This grants full access to all projects, databases, audit logs, terminals, and settings.",
          action: "Promote to Admin",
          variant: "default" as const,
        },
        demote: {
          title: "Revoke admin?",
          description: "This removes admin privileges and limits access to the user's own data.",
          action: "Revoke Admin",
          variant: "default" as const,
        },
        delete: {
          title: "Delete user?",
          description:
            "This permanently deletes the user and ALL their projects and databases. This cannot be undone.",
          action: "Delete User",
          variant: "destructive" as const,
        },
      }[pendingAction.type]
    : null;

  let dialogIcon = null;
  if (pendingAction) {
    if (pendingAction.type === "promote") {
      dialogIcon = <HugeiconsIcon icon={SecurityCheckIcon} className="size-4" strokeWidth={2} />;
    } else if (pendingAction.type === "demote") {
      dialogIcon = <ShieldOff className="size-4" />;
    } else {
      dialogIcon = <HugeiconsIcon icon={Delete01Icon} className="size-4" strokeWidth={2} />;
    }
  }

  return (
    <div className="space-y-6">
      <div>
        <h1 className="text-2xl font-bold tracking-tight">User Management</h1>
        <p className="text-muted-foreground">
          Manage system users, assigned roles, and permissions.
        </p>
      </div>

      <div className="flex items-center justify-between">
        <div className="relative w-full max-w-sm">
          <HugeiconsIcon
            icon={Search01Icon}
            className="absolute left-3 top-1/2 size-4 -translate-y-1/2 text-muted-foreground"
            strokeWidth={2}
          />
          <Input
            placeholder="Search users by email..."
            value={search}
            onChange={(e) => setSearch(e.target.value)}
            className="pl-10"
          />
        </div>
      </div>

      <Card>
        <CardHeader className="p-0" />
        <CardContent className="p-0">
          <Table>
            <TableHeader>
              <TableRow className="hover:bg-transparent">
                <TableHead>Email</TableHead>
                <TableHead>Role</TableHead>
                <TableHead>Joined</TableHead>
                <TableHead className="w-[80px]" />
              </TableRow>
            </TableHeader>
            <TableBody>
              {isLoading ? (
                Array.from({ length: 5 }).map((_, i) => (
                  <TableRow key={i}>
                    <TableCell>
                      <Skeleton className="h-4 w-48" />
                    </TableCell>
                    <TableCell>
                      <Skeleton className="h-5 w-16 rounded-full" />
                    </TableCell>
                    <TableCell>
                      <Skeleton className="h-4 w-24" />
                    </TableCell>
                    <TableCell>
                      <Skeleton className="size-8" />
                    </TableCell>
                  </TableRow>
                ))
              ) : filteredUsers.length === 0 ? (
                <TableRow>
                  <TableCell colSpan={4} className="h-48 text-center">
                    <div className="flex flex-col items-center justify-center space-y-3">
                      <div className="flex size-12 items-center justify-center rounded-full bg-muted">
                        <HugeiconsIcon
                          icon={UserMultiple02Icon}
                          className="size-6 text-muted-foreground"
                          strokeWidth={2}
                        />
                      </div>
                      <div className="text-center">
                        <p className="text-sm font-medium">No users found</p>
                        <p className="text-xs text-muted-foreground">
                          {search ? "Try adjusting your search terms" : "No users registered yet"}
                        </p>
                      </div>
                    </div>
                  </TableCell>
                </TableRow>
              ) : (
                filteredUsers.map((user) => (
                  <TableRow key={user.id} className="group">
                    <TableCell className="font-medium">{user.email}</TableCell>
                    <TableCell>
                      <Badge
                        variant={user.role === "admin" ? "default" : "secondary"}
                        className="capitalize"
                      >
                        {user.role}
                      </Badge>
                    </TableCell>
                    <TableCell className="text-muted-foreground">
                      {format(new Date(user.created_at), "MMM d, yyyy")}
                    </TableCell>
                    <TableCell>
                      <DropdownMenu>
                        <DropdownMenuTrigger asChild>
                          <Button
                            variant="ghost"
                            size="icon-sm"
                            className="opacity-0 transition-opacity group-hover:opacity-100"
                          >
                            <span className="sr-only">Open menu</span>
                            <HugeiconsIcon
                              icon={MoreHorizontalIcon}
                              className="size-4"
                              strokeWidth={2}
                            />
                          </Button>
                        </DropdownMenuTrigger>
                        <DropdownMenuContent align="end">
                          {user.role === "user" ? (
                            <DropdownMenuItem
                              onClick={() => setPendingAction({ type: "promote", user })}
                            >
                              <HugeiconsIcon
                                icon={SecurityCheckIcon}
                                className="mr-2 size-4"
                                strokeWidth={2}
                              />
                              Make Admin
                            </DropdownMenuItem>
                          ) : (
                            <DropdownMenuItem
                              onClick={() => setPendingAction({ type: "demote", user })}
                              disabled={user.id === currentUser?.id}
                            >
                              <ShieldOff className="mr-2 size-4" strokeWidth={2} />
                              Revoke Admin
                            </DropdownMenuItem>
                          )}

                          <DropdownMenuItem
                            className="text-destructive focus:text-destructive"
                            onClick={() => setPendingAction({ type: "delete", user })}
                            disabled={user.id === currentUser?.id}
                          >
                            <HugeiconsIcon
                              icon={Delete01Icon}
                              className="mr-2 size-4"
                              strokeWidth={2}
                            />
                            Delete User
                          </DropdownMenuItem>
                        </DropdownMenuContent>
                      </DropdownMenu>
                    </TableCell>
                  </TableRow>
                ))
              )}
            </TableBody>
          </Table>
        </CardContent>
      </Card>

      <AlertDialog open={!!pendingAction} onOpenChange={(open) => !open && setPendingAction(null)}>
        {pendingAction && dialogCopy ? (
          <AlertDialogContent>
            <AlertDialogHeader>
              {dialogIcon ? <AlertDialogMedia>{dialogIcon}</AlertDialogMedia> : null}
              <AlertDialogTitle>{dialogCopy.title}</AlertDialogTitle>
              <AlertDialogDescription>{dialogCopy.description}</AlertDialogDescription>
            </AlertDialogHeader>
            <AlertDialogFooter>
              <AlertDialogCancel disabled={isMutating}>Cancel</AlertDialogCancel>
              <AlertDialogAction
                variant={dialogCopy.variant}
                onClick={confirmAction}
                disabled={isMutating}
              >
                {dialogCopy.action}
              </AlertDialogAction>
            </AlertDialogFooter>
          </AlertDialogContent>
        ) : null}
      </AlertDialog>
    </div>
  );
}
