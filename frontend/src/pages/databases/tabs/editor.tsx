import { useDatabaseDetailContext } from "@/pages/databases/detail-context";
import { EditorPanel } from "@/pages/databases/panels/editor-panel";

/**
 * Renders an editor panel for the current database.
 *
 * Retrieves the active database and its running state from the database detail context
 * and returns an EditorPanel configured with that data.
 *
 * @returns The EditorPanel React element for editing the current database.
 */
export function DatabaseEditorTab() {
  const { database, isRunning } = useDatabaseDetailContext();

  return <EditorPanel key={database.id} database={database} isRunning={isRunning} />;
}