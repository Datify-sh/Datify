import { useDatabaseDetailContext } from "@/pages/databases/detail-context";
import LogsPanel from "@/pages/databases/panels/logs-panel";

export function DatabaseLogsTab() {
  const { database } = useDatabaseDetailContext();

  return <LogsPanel databaseId={database.id} />;
}
