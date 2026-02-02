import { useDatabaseDetailContext } from "@/pages/databases/detail-context";
import { EditorPanel } from "@/pages/databases/panels/editor-panel";

export function DatabaseEditorTab() {
  const { database, isRunning } = useDatabaseDetailContext();

  return <EditorPanel key={database.id} database={database} isRunning={isRunning} />;
}
