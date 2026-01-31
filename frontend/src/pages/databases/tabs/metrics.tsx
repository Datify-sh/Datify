import { MetricsPanel } from "@/components/database/metrics-panel";
import { QueryLogsPanel } from "@/components/database/query-logs-panel";
import { useDatabaseDetailContext } from "@/pages/databases/detail-context";

export function DatabaseMetricsTab() {
  const { database, isRunning, isKeyValue } = useDatabaseDetailContext();

  return (
    <div className="space-y-6">
      <MetricsPanel
        databaseId={database.id}
        databaseType={database.database_type}
        isRunning={isRunning}
      />
      {!isKeyValue && <QueryLogsPanel databaseId={database.id} isRunning={isRunning} />}
    </div>
  );
}
