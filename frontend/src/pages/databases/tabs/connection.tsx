import { useDatabaseDetailContext } from "@/pages/databases/detail-context";
import ConnectionPanel from "@/pages/databases/panels/connection-panel";

export function DatabaseConnectionTab() {
  const { database } = useDatabaseDetailContext();

  return <ConnectionPanel database={database} />;
}
