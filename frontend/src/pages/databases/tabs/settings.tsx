import { useDatabaseDetailContext } from "@/pages/databases/detail-context";
import SettingsPanel from "@/pages/databases/panels/settings-panel";

export function DatabaseSettingsTab() {
  const { database } = useDatabaseDetailContext();

  return <SettingsPanel database={database} />;
}
