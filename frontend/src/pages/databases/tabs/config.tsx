import { useDatabaseDetailContext } from "@/pages/databases/detail-context";
import ConfigPanel from "@/pages/databases/panels/config-panel";

export function DatabaseConfigTab() {
  const { database } = useDatabaseDetailContext();

  return <ConfigPanel database={database} />;
}
