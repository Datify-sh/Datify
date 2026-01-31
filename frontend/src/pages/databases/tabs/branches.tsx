import { BranchPanel } from "@/components/database/branch-panel";
import { Card, CardContent } from "@/components/ui/card";
import { useDatabaseDetailContext } from "@/pages/databases/detail-context";

export function DatabaseBranchesTab() {
  const { id, hasBranches, openCreateBranch } = useDatabaseDetailContext();

  if (!hasBranches) {
    return (
      <Card>
        <CardContent className="py-12 text-center text-sm text-muted-foreground">
          Branches are not available for this database.
        </CardContent>
      </Card>
    );
  }

  return (
    <BranchPanel databaseId={id} currentBranchId={id} onCreateBranch={() => openCreateBranch()} />
  );
}
