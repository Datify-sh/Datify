import {
  editorHighlightDark,
  editorHighlightLight,
  editorThemeDark,
  editorThemeLight,
} from "@/components/editor/editor-theme";
import type { DatabaseType } from "@/lib/api/types";
import { cn } from "@/lib/utils";
import { StreamLanguage, type StringStream } from "@codemirror/language";
import type { Extension } from "@codemirror/state";
import { CodeIcon, Database01Icon } from "@hugeicons/core-free-icons";
import { HugeiconsIcon } from "@hugeicons/react";
import { useTheme } from "next-themes";
import * as React from "react";

// Lazy load CodeMirror to avoid SSR issues
const CodeMirror = React.lazy(() => import("@uiw/react-codemirror"));
const sqlLang = () => import("@codemirror/lang-sql").then((m) => m.sql());

type KvState = { lineStart: boolean };

const kvLanguage = StreamLanguage.define<KvState>({
  name: "keyvalue",
  startState: (): KvState => ({ lineStart: true }),
  token(stream: StringStream, state: KvState) {
    if (stream.sol()) {
      state.lineStart = true;
    }

    if (stream.eatSpace()) return null;

    if (stream.match(/^(#|--|\/\/).*/)) {
      stream.skipToEnd();
      return "comment";
    }

    const markToken = <T extends string>(token: T) => {
      state.lineStart = false;
      return token;
    };

    if (stream.match(/^"(?:[^"\\]|\\.)*"/) || stream.match(/^'(?:[^'\\]|\\.)*'/)) {
      return markToken("string");
    }

    if (stream.match(/^-?\d+(?:\.\d+)?/)) {
      return markToken("number");
    }

    if (stream.match(/^[A-Za-z][A-Za-z0-9:_-]*/)) {
      const token = state.lineStart ? "keyword" : "variableName";
      state.lineStart = false;
      return token;
    }

    stream.next();
    state.lineStart = false;
    return null;
  },
});

interface QueryEditorProps {
  content: string;
  onChange: (content: string) => void;
  databaseType: DatabaseType;
  onRun: () => void;
  className?: string;
}

/**
 * Renders an embedded code editor for SQL (Postgres) or a lightweight key-value mode and exposes content editing and run controls.
 *
 * The editor adapts to the current theme, lazy-loads Postgres SQL support when `databaseType` is "postgres", shows a loading placeholder before initialization, and listens for Cmd/Ctrl+Enter to invoke `onRun`.
 *
 * @param content - The current editor content
 * @param onChange - Called when the editor content changes
 * @param databaseType - The active database type; selects SQL highlighting for "postgres" and a key-value mode otherwise
 * @param onRun - Invoked when the user triggers the run shortcut (Cmd/Ctrl+Enter)
 * @param className - Optional container class names for styling
 * @returns The rendered QueryEditor React element
 */
export function QueryEditor({
  content,
  onChange,
  databaseType,
  onRun,
  className,
}: QueryEditorProps) {
  const { resolvedTheme } = useTheme();
  const isDark = resolvedTheme !== "light";
  const [extensions, setExtensions] = React.useState<Extension[]>([]);
  const [mounted, setMounted] = React.useState(false);

  React.useEffect(() => {
    setMounted(true);
    let isActive = true;

    const loadExtensions = async () => {
      const sql = await (databaseType === "postgres" ? sqlLang() : Promise.resolve(null));

      if (!isActive) return;
      setExtensions(
        databaseType === "postgres" && sql
          ? [isDark ? editorHighlightDark : editorHighlightLight, sql as Extension]
          : [isDark ? editorHighlightDark : editorHighlightLight, kvLanguage as Extension],
      );
    };

    loadExtensions();

    return () => {
      isActive = false;
    };
  }, [databaseType, isDark]);

  const handleKeyDown = React.useCallback(
    (e: React.KeyboardEvent) => {
      if ((e.metaKey || e.ctrlKey) && e.key === "Enter") {
        e.preventDefault();
        onRun();
      }
    },
    [onRun],
  );

  if (!mounted) {
    return (
      <div className={cn("h-full w-full flex items-center justify-center bg-muted/30", className)}>
        <div className="flex items-center gap-2 text-muted-foreground">
          <HugeiconsIcon
            icon={databaseType === "postgres" ? Database01Icon : CodeIcon}
            className="size-5"
            strokeWidth={2}
          />
          <span className="text-sm">Loading editor...</span>
        </div>
      </div>
    );
  }

  return (
    <div className={cn("h-full w-full relative group", className)} onKeyDown={handleKeyDown}>
      <React.Suspense
        fallback={
          <div className="h-full w-full flex items-center justify-center bg-muted/30">
            <div className="flex items-center gap-2 text-muted-foreground">
              <HugeiconsIcon
                icon={databaseType === "postgres" ? Database01Icon : CodeIcon}
                className="size-5"
                strokeWidth={2}
              />
              <span className="text-sm">Loading editor...</span>
            </div>
          </div>
        }
      >
        <CodeMirror
          value={content}
          height="100%"
          extensions={extensions}
          theme={isDark ? editorThemeDark : editorThemeLight}
          onChange={(value) => onChange(value)}
          className="h-full text-sm [&_.cm-editor]:h-full [&_.cm-gutters]:bg-muted/50 [&_.cm-gutters]:border-r [&_.cm-gutters]:border-border [&_.cm-activeLineGutter]:bg-muted [&_.cm-lineNumbers]:text-muted-foreground"
          basicSetup={{
            lineNumbers: true,
            highlightActiveLineGutter: true,
            highlightActiveLine: true,
            foldGutter: false,
            dropCursor: true,
            allowMultipleSelections: true,
            indentOnInput: true,
            bracketMatching: true,
            closeBrackets: true,
            autocompletion: true,
            rectangularSelection: true,
            crosshairCursor: true,
            highlightSelectionMatches: true,
            closeBracketsKeymap: true,
            defaultKeymap: true,
            searchKeymap: true,
            historyKeymap: true,
            foldKeymap: true,
            completionKeymap: true,
            lintKeymap: true,
          }}
        />
      </React.Suspense>

      {/* Keyboard shortcut hint */}
      <div className="absolute bottom-2 right-2 text-[10px] text-muted-foreground opacity-0 group-hover:opacity-100 transition-opacity pointer-events-none bg-background/80 px-2 py-1 rounded border">
        {databaseType === "postgres" ? "Cmd/Ctrl+Enter to run" : "Cmd/Ctrl+Enter to execute"}
      </div>
    </div>
  );
}