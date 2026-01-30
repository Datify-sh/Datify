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
import { Skeleton } from "@/components/ui/skeleton";
import { Spinner } from "@/components/ui/spinner";
import { Tooltip, TooltipContent, TooltipTrigger } from "@/components/ui/tooltip";
import { type BranchResponse, databasesApi } from "@/lib/api";
import { cn } from "@/lib/utils";
import {
  Add01Icon,
  ArrowRight01Icon,
  CheckmarkCircle02Icon,
  Clock01Icon,
  Delete01Icon,
  GitBranchIcon,
} from "@hugeicons/core-free-icons";
import { HugeiconsIcon } from "@hugeicons/react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { format, formatDistanceToNow } from "date-fns";
import { Link } from "react-router-dom";
import { toast } from "sonner";

interface BranchPanelProps {
  databaseId: string;
  currentBranchId: string;
  onCreateBranch: () => void;
}

const statusConfig: Record<
  string,
  { color: string; variant: "default" | "secondary" | "destructive" | "outline" }
> = {
  running: { color: "bg-green-500", variant: "default" },
  stopped: { color: "bg-neutral-400", variant: "secondary" },
  starting: { color: "bg-yellow-500", variant: "outline" },
  stopping: { color: "bg-yellow-500", variant: "outline" },
  error: { color: "bg-red-500", variant: "destructive" },
};

export function BranchPanel({ databaseId, currentBranchId, onCreateBranch }: BranchPanelProps) {
  const queryClient = useQueryClient();

  const { data: branches, isLoading } = useQuery({
    queryKey: ["branches", databaseId],
    queryFn: () => databasesApi.listBranches(databaseId),
  });

  const deleteMutation = useMutation({
    mutationFn: (branchId: string) => databasesApi.delete(branchId),
    onSuccess: (_, deletedBranchId) => {
      queryClient.invalidateQueries({ queryKey: ["branches"] });
      queryClient.invalidateQueries({ queryKey: ["databases"] });
      queryClient.invalidateQueries({ queryKey: ["projects"] });
      queryClient.invalidateQueries({ queryKey: ["all-databases-for-stats"] });
      queryClient.removeQueries({ queryKey: ["database", deletedBranchId] });
      toast.success("Branch deleted");
    },
    onError: () => toast.error("Failed to delete branch"),
  });

  if (isLoading) {
    return (
      <Card>
        <CardHeader>
          <Skeleton className="h-5 w-24" />
          <Skeleton className="h-4 w-48" />
        </CardHeader>
        <CardContent className="space-y-3">
          <Skeleton className="h-16" />
          <Skeleton className="h-16" />
        </CardContent>
      </Card>
    );
  }

  const mainBranch = branches?.find((b) => b.is_default);
  const childBranches = branches?.filter((b) => !b.is_default) ?? [];

  return (
    <Card>
      <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-4">
        <div>
          <CardTitle className="text-base flex items-center gap-2">
            <HugeiconsIcon icon={GitBranchIcon} className="size-4" strokeWidth={2} />
            Branches
          </CardTitle>
          <CardDescription>
            {branches?.length ?? 0} branch{(branches?.length ?? 0) !== 1 && "es"} in this database
          </CardDescription>
        </div>
        <Button size="sm" onClick={onCreateBranch}>
          <HugeiconsIcon icon={Add01Icon} className="size-4" strokeWidth={2} />
          New Branch
        </Button>
      </CardHeader>
      <CardContent className="space-y-2">
        {mainBranch && (
          <BranchRow
            branch={mainBranch}
            isCurrent={mainBranch.id === currentBranchId}
            isMain={true}
            onDelete={() => deleteMutation.mutate(mainBranch.id)}
            isDeleting={deleteMutation.isPending}
          />
        )}

        {childBranches.length > 0 && (
          <div className="relative ml-4 space-y-2 before:absolute before:left-0 before:top-0 before:bottom-4 before:w-px before:bg-border">
            {childBranches.map((branch) => (
              <div key={branch.id} className="relative pl-4">
                <div className="absolute left-0 top-5 w-4 h-px bg-border" />
                <BranchRow
                  branch={branch}
                  isCurrent={branch.id === currentBranchId}
                  isMain={false}
                  onDelete={() => deleteMutation.mutate(branch.id)}
                  isDeleting={deleteMutation.isPending}
                />
              </div>
            ))}
          </div>
        )}

        {branches?.length === 1 && (
          <div className="text-center py-4 text-sm text-muted-foreground">
            <p>No child branches yet.</p>
            <p className="text-xs mt-1">Create a branch to test changes without affecting main.</p>
          </div>
        )}
      </CardContent>
    </Card>
  );
}

function BranchRow({
  branch,
  isCurrent,
  isMain,
  onDelete,
  isDeleting,
}: {
  branch: BranchResponse;
  isCurrent: boolean;
  isMain: boolean;
  onDelete: () => void;
  isDeleting: boolean;
}) {
  const status = statusConfig[branch.status] ?? {
    color: "bg-neutral-400",
    variant: "secondary" as const,
  };

  return (
    <div
      className={cn(
        "flex items-center justify-between gap-4 rounded-lg border p-3 transition-colors",
        isCurrent && "border-primary bg-primary/5",
      )}
    >
      <div className="flex items-center gap-3 min-w-0">
        <div className="flex items-center gap-2 min-w-0">
          <HugeiconsIcon
            icon={GitBranchIcon}
            className={cn("size-4 shrink-0", isCurrent ? "text-primary" : "text-muted-foreground")}
            strokeWidth={2}
          />
          <Link
            to={`/databases/${branch.id}`}
            className={cn(
              "font-mono text-sm truncate hover:underline",
              isCurrent && "text-primary font-medium",
            )}
          >
            {branch.name}
          </Link>
          {isMain && (
            <Badge variant="secondary" className="text-[10px] px-1.5 py-0 h-4 shrink-0">
              main
            </Badge>
          )}
          {isCurrent && (
            <Tooltip>
              <TooltipTrigger>
                <HugeiconsIcon
                  icon={CheckmarkCircle02Icon}
                  className="size-4 text-primary shrink-0"
                  strokeWidth={2}
                />
              </TooltipTrigger>
              <TooltipContent>Current branch</TooltipContent>
            </Tooltip>
          )}
        </div>
      </div>

      <div className="flex items-center gap-2 shrink-0">
        {branch.forked_at && !isMain && (
          <Tooltip>
            <TooltipTrigger className="flex items-center gap-1 text-xs text-muted-foreground">
              <HugeiconsIcon icon={Clock01Icon} className="size-3" strokeWidth={2} />
              {formatDistanceToNow(new Date(branch.forked_at), { addSuffix: false })}
            </TooltipTrigger>
            <TooltipContent>
              Forked {format(new Date(branch.forked_at), "MMM d, yyyy 'at' h:mm a")}
            </TooltipContent>
          </Tooltip>
        )}

        <Badge variant={status.variant} className="text-[10px] px-1.5 py-0 h-5">
          <span className={cn("size-1.5 rounded-full mr-1", status.color)} />
          {branch.status}
        </Badge>

        {!isCurrent && (
          <div className="flex items-center gap-1">
            <Button variant="ghost" size="icon-sm" asChild>
              <Link to={`/databases/${branch.id}`}>
                <HugeiconsIcon icon={ArrowRight01Icon} className="size-4" strokeWidth={2} />
              </Link>
            </Button>

            {!isMain && (
              <AlertDialog>
                <AlertDialogTrigger asChild>
                  <Button
                    variant="ghost"
                    size="icon-sm"
                    className="text-muted-foreground hover:text-destructive"
                    disabled={isDeleting}
                  >
                    {isDeleting ? (
                      <Spinner className="size-3.5" />
                    ) : (
                      <HugeiconsIcon icon={Delete01Icon} className="size-3.5" strokeWidth={2} />
                    )}
                  </Button>
                </AlertDialogTrigger>
                <AlertDialogContent>
                  <AlertDialogHeader>
                    <AlertDialogTitle>Delete Branch</AlertDialogTitle>
                    <AlertDialogDescription>
                      Are you sure you want to delete the branch "{branch.name}"? This action cannot
                      be undone and all data will be permanently lost.
                    </AlertDialogDescription>
                  </AlertDialogHeader>
                  <AlertDialogFooter>
                    <AlertDialogCancel>Cancel</AlertDialogCancel>
                    <AlertDialogAction
                      onClick={onDelete}
                      className="bg-destructive text-destructive-foreground hover:bg-destructive/90"
                    >
                      Delete
                    </AlertDialogAction>
                  </AlertDialogFooter>
                </AlertDialogContent>
              </AlertDialog>
            )}
          </div>
        )}
      </div>
    </div>
  );
}
