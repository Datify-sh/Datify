import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogClose,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Field, FieldDescription, FieldLabel } from "@/components/ui/field";
import { Input } from "@/components/ui/input";
import { Progress } from "@/components/ui/progress";
import { Spinner } from "@/components/ui/spinner";
import { databasesApi, getErrorMessage } from "@/lib/api";
import { cn } from "@/lib/utils";
import {
  Database01Icon,
  File01Icon,
  GitBranchIcon,
  InformationCircleIcon,
} from "@hugeicons/core-free-icons";
import { HugeiconsIcon, type IconSvgElement } from "@hugeicons/react";
import { useMutation, useQueryClient } from "@tanstack/react-query";
import * as React from "react";
import { useNavigate } from "react-router-dom";
import { toast } from "sonner";

interface CreateBranchDialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  databaseId: string;
  sourceBranchName: string;
  databaseType: string;
}

type BranchMode = "full" | "schema";

export function CreateBranchDialog({
  open,
  onOpenChange,
  databaseId,
  sourceBranchName,
  databaseType,
}: CreateBranchDialogProps) {
  const isKeyValue = databaseType === "valkey" || databaseType === "redis";
  const navigate = useNavigate();
  const queryClient = useQueryClient();
  const [name, setName] = React.useState("");
  const [mode, setMode] = React.useState<BranchMode>("full");
  const [progress, setProgress] = React.useState(0);

  React.useEffect(() => {
    if (!open) {
      setName("");
      setMode("full");
      setProgress(0);
    }
  }, [open]);

  const createMutation = useMutation({
    mutationFn: () => {
      setProgress(10);
      const progressInterval = setInterval(() => {
        setProgress((prev) => Math.min(prev + 5, 90));
      }, 500);

      return databasesApi
        .createBranch(databaseId, {
          name,
          include_data: mode === "full",
        })
        .finally(() => {
          clearInterval(progressInterval);
          setProgress(100);
        });
    },
    onSuccess: (newBranch) => {
      const branchId = newBranch.id;
      queryClient.invalidateQueries({ queryKey: ["branches"] });
      queryClient.invalidateQueries({ queryKey: ["databases", newBranch.project_id] });
      queryClient.invalidateQueries({ queryKey: ["projects"] });
      queryClient.invalidateQueries({ queryKey: ["all-databases-for-stats"] });
      queryClient.invalidateQueries({ queryKey: ["database", databaseId] });
      toast.success(`Branch "${name}" created successfully`);
      onOpenChange(false);
      setTimeout(() => {
        navigate(`/databases/${branchId}`);
      }, 150);
    },
    onError: (err) => {
      toast.error(getErrorMessage(err, "Failed to create branch"));
      setProgress(0);
    },
  });

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    if (name.trim()) {
      createMutation.mutate();
    }
  };

  const isValidName = name.trim().length > 0 && /^[a-z0-9-]+$/.test(name.trim());

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="sm:max-w-md">
        <DialogHeader>
          <DialogTitle className="flex items-center gap-2">
            <HugeiconsIcon icon={GitBranchIcon} className="size-5" strokeWidth={2} />
            Create Branch
          </DialogTitle>
          <DialogDescription>
            Create a new branch from{" "}
            <span className="font-mono font-medium">{sourceBranchName}</span>
          </DialogDescription>
        </DialogHeader>
        <form onSubmit={handleSubmit}>
          <div className="space-y-6 py-4">
            <Field>
              <FieldLabel>Branch Name</FieldLabel>
              <Input
                value={name}
                onChange={(e) =>
                  setName(
                    e.target.value
                      .toLowerCase()
                      .replace(/\s+/g, "-")
                      .replace(/[^a-z0-9-]/g, "")
                      .replace(/-+/g, "-"),
                  )
                }
                placeholder="feature-auth"
                required
                autoFocus
                disabled={createMutation.isPending}
                className="font-mono"
              />
              <FieldDescription>Lowercase letters, numbers, and hyphens only</FieldDescription>
            </Field>

            {!isKeyValue && (
              <Field>
                <FieldLabel>Branch Type</FieldLabel>
                <div className="grid grid-cols-2 gap-3 mt-2">
                  <BranchModeOption
                    mode="full"
                    currentMode={mode}
                    onSelect={setMode}
                    disabled={createMutation.isPending}
                    icon={Database01Icon}
                    title="With Data"
                    description="Full copy of all data"
                  />
                  <BranchModeOption
                    mode="schema"
                    currentMode={mode}
                    onSelect={setMode}
                    disabled={createMutation.isPending}
                    icon={File01Icon}
                    title="Schema Only"
                    description="Structure without data"
                  />
                </div>
              </Field>
            )}

            <div className="rounded-lg border bg-muted/50 p-3">
              <div className="flex gap-2">
                <HugeiconsIcon
                  icon={InformationCircleIcon}
                  className="size-4 text-muted-foreground shrink-0 mt-0.5"
                  strokeWidth={2}
                />
                <div className="text-xs text-muted-foreground space-y-1">
                  {isKeyValue ? (
                    <>
                      <p>
                        <strong className="text-foreground">Full copy</strong> creates an
                        independent database with all your keys and data using replication.
                      </p>
                      <p>Changes in the branch won't affect the source.</p>
                    </>
                  ) : mode === "full" ? (
                    <>
                      <p>
                        <strong className="text-foreground">Full copy</strong> creates an
                        independent database with all your data. Perfect for testing with real data.
                      </p>
                      <p>Changes in the branch won't affect the source.</p>
                    </>
                  ) : (
                    <>
                      <p>
                        <strong className="text-foreground">Schema only</strong> copies tables,
                        indexes, and constraints without row data.
                      </p>
                      <p>Ideal for development with sensitive production data.</p>
                    </>
                  )}
                </div>
              </div>
            </div>

            {createMutation.isPending && (
              <div className="space-y-2">
                <div className="flex items-center justify-between text-sm">
                  <span className="text-muted-foreground">
                    {mode === "full" ? "Creating branch and copying data..." : "Creating branch..."}
                  </span>
                  <span className="text-muted-foreground tabular-nums">{progress}%</span>
                </div>
                <Progress value={progress} className="h-2" />
              </div>
            )}
          </div>

          <DialogFooter>
            <DialogClose asChild>
              <Button type="button" variant="outline" disabled={createMutation.isPending}>
                Cancel
              </Button>
            </DialogClose>
            <Button type="submit" disabled={!isValidName || createMutation.isPending}>
              {createMutation.isPending && <Spinner className="size-4" />}
              Create Branch
            </Button>
          </DialogFooter>
        </form>
      </DialogContent>
    </Dialog>
  );
}

function BranchModeOption({
  mode,
  currentMode,
  onSelect,
  disabled,
  icon,
  title,
  description,
}: {
  mode: BranchMode;
  currentMode: BranchMode;
  onSelect: (mode: BranchMode) => void;
  disabled: boolean;
  icon: IconSvgElement;
  title: string;
  description: string;
}) {
  const isSelected = mode === currentMode;

  return (
    <button
      type="button"
      onClick={() => onSelect(mode)}
      disabled={disabled}
      className={cn(
        "flex flex-col items-start gap-1 rounded-lg border p-3 text-left transition-colors",
        "hover:bg-muted/50 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring",
        isSelected && "border-primary bg-primary/5",
        disabled && "opacity-50 cursor-not-allowed",
      )}
    >
      <div className="flex items-center gap-2">
        <HugeiconsIcon
          icon={icon}
          className={cn("size-4", isSelected ? "text-primary" : "text-muted-foreground")}
          strokeWidth={2}
        />
        <span className={cn("text-sm font-medium", isSelected && "text-primary")}>{title}</span>
      </div>
      <span className="text-xs text-muted-foreground">{description}</span>
    </button>
  );
}
