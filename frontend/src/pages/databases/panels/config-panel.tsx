import { Button } from "@/components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Skeleton } from "@/components/ui/skeleton";
import { Spinner } from "@/components/ui/spinner";
import { databasesApi, getErrorMessage } from "@/lib/api";
import { Decoration, EditorView, ViewPlugin, type ViewUpdate, placeholder } from "@codemirror/view";
import { RefreshIcon } from "@hugeicons/core-free-icons";
import { HugeiconsIcon } from "@hugeicons/react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import CodeMirror from "@uiw/react-codemirror";
import * as React from "react";
import { toast } from "sonner";

const configTheme = EditorView.theme(
  {
    "&": {
      backgroundColor: "transparent",
      color: "#d1fae5",
      fontFamily:
        'ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, "Liberation Mono", "Courier New", monospace',
      fontSize: "12px",
    },
    ".cm-scroller": {
      backgroundColor: "#0a0a0a",
    },
    ".cm-content": {
      padding: "16px",
      caretColor: "#34d399",
    },
    ".cm-selectionBackground": {
      backgroundColor: "rgba(16, 185, 129, 0.25)",
    },
    ".cm-activeLine": {
      backgroundColor: "rgba(255, 255, 255, 0.04)",
    },
    ".cm-gutters": {
      backgroundColor: "#0a0a0a",
      color: "#6b7280",
      border: "none",
    },
    ".cm-activeLineGutter": {
      color: "#e5e7eb",
    },
    ".cm-config-comment": {
      color: "#64748b",
      fontStyle: "italic",
    },
    ".cm-config-key": {
      color: "#7dd3fc",
    },
    ".cm-config-string": {
      color: "#f9a8d4",
    },
    ".cm-config-number": {
      color: "#facc15",
    },
    ".cm-config-bool": {
      color: "#a78bfa",
    },
    ".cm-config-operator": {
      color: "#94a3b8",
    },
  },
  { dark: true },
);

const configKeyDecoration = Decoration.mark({ class: "cm-config-key" });
const configStringDecoration = Decoration.mark({ class: "cm-config-string" });
const configNumberDecoration = Decoration.mark({ class: "cm-config-number" });
const configBoolDecoration = Decoration.mark({ class: "cm-config-bool" });
const configCommentDecoration = Decoration.mark({ class: "cm-config-comment" });
const configOperatorDecoration = Decoration.mark({ class: "cm-config-operator" });

const configBooleanPattern = /^(true|false|yes|no|on|off)$/i;
const configNumberPattern = /^[0-9]+(?:\.[0-9]+)?[a-z%]*$/i;

function buildConfigDecorations(view: EditorView) {
  const ranges: ReturnType<typeof configKeyDecoration.range>[] = [];

  for (const { from, to } of view.visibleRanges) {
    let line = view.state.doc.lineAt(from);
    while (line.from <= to) {
      const text = line.text;
      const lineStart = line.from;

      const commentMatch = /(^|\s)([#;])/.exec(text);
      let contentEnd = text.length;
      if (commentMatch) {
        const commentIndex = commentMatch.index + commentMatch[1].length;
        ranges.push(configCommentDecoration.range(lineStart + commentIndex, line.to));
        contentEnd = commentIndex;
      }

      const leadingIndex = text.slice(0, contentEnd).search(/\S/);
      if (leadingIndex >= 0) {
        let cursor = leadingIndex;
        while (cursor < contentEnd && /[A-Za-z0-9_.-]/.test(text[cursor])) {
          cursor += 1;
        }

        if (cursor > leadingIndex) {
          ranges.push(configKeyDecoration.range(lineStart + leadingIndex, lineStart + cursor));
        }

        let separatorIndex = cursor;
        while (separatorIndex < contentEnd && /\s/.test(text[separatorIndex])) {
          separatorIndex += 1;
        }
        if (separatorIndex < contentEnd && text[separatorIndex] === "=") {
          ranges.push(
            configOperatorDecoration.range(
              lineStart + separatorIndex,
              lineStart + separatorIndex + 1,
            ),
          );
          separatorIndex += 1;
        }
        while (separatorIndex < contentEnd && /\s/.test(text[separatorIndex])) {
          separatorIndex += 1;
        }

        if (separatorIndex < contentEnd) {
          let valueEnd = contentEnd;
          while (valueEnd > separatorIndex && /\s/.test(text[valueEnd - 1])) {
            valueEnd -= 1;
          }
          const value = text.slice(separatorIndex, valueEnd);
          if (value.startsWith('"') || value.startsWith("'")) {
            ranges.push(
              configStringDecoration.range(lineStart + separatorIndex, lineStart + valueEnd),
            );
          } else if (configBooleanPattern.test(value)) {
            ranges.push(
              configBoolDecoration.range(lineStart + separatorIndex, lineStart + valueEnd),
            );
          } else if (configNumberPattern.test(value)) {
            ranges.push(
              configNumberDecoration.range(lineStart + separatorIndex, lineStart + valueEnd),
            );
          } else {
            ranges.push(
              configStringDecoration.range(lineStart + separatorIndex, lineStart + valueEnd),
            );
          }
        }
      }

      if (line.to >= to) {
        break;
      }
      line = view.state.doc.lineAt(line.to + 1);
    }
  }

  return Decoration.set(ranges, true);
}

const configHighlightPlugin = ViewPlugin.fromClass(
  class {
    decorations;
    view: EditorView;

    constructor(view: EditorView) {
      this.view = view;
      this.decorations = buildConfigDecorations(view);
    }

    update(update: ViewUpdate) {
      if (update.docChanged || update.viewportChanged) {
        this.decorations = buildConfigDecorations(update.view);
      }
    }
  },
  {
    decorations: (value) => value.decorations,
  },
);

const baseConfigExtensions = [configTheme, configHighlightPlugin, EditorView.lineWrapping];

type DatabaseDetail = NonNullable<Awaited<ReturnType<typeof databasesApi.get>>>;

const ConfigPanel = React.memo(function ConfigPanel({ database }: { database: DatabaseDetail }) {
  const queryClient = useQueryClient();

  const {
    data: config,
    isLoading,
    isError,
    isFetching,
    refetch,
  } = useQuery({
    queryKey: ["database-config", database.id],
    queryFn: () => databasesApi.getConfig(database.id),
    staleTime: 30_000,
    refetchOnWindowFocus: false,
  });

  const [draft, setDraft] = React.useState("");

  React.useEffect(() => {
    if (config) {
      setDraft(config.content ?? "");
    }
  }, [config]);

  const updateMutation = useMutation({
    mutationFn: (content: string) => databasesApi.updateConfig(database.id, { content }),
    onSuccess: (response) => {
      queryClient.invalidateQueries({ queryKey: ["database-config", database.id] });
      toast.success(response.applied ? "Config applied" : "Config saved");
    },
    onError: (err) => toast.error(getErrorMessage(err, "Failed to update config")),
  });

  const hasChanges = draft !== (config?.content ?? "");
  const lineCount = React.useMemo(() => (draft.length ? draft.split("\n").length : 0), [draft]);

  const placeholderText =
    config?.format === "kv"
      ? "# maxmemory 256mb\n# maxmemory-policy allkeys-lru"
      : "# Add your config values here";

  const editorExtensions = React.useMemo(
    () => [...baseConfigExtensions, placeholder(placeholderText)],
    [placeholderText],
  );

  if (isError) {
    return (
      <Card>
        <CardContent className="py-10 text-center space-y-3">
          <p className="text-sm text-muted-foreground">Failed to load config.</p>
          <Button variant="outline" size="sm" onClick={() => refetch()}>
            <HugeiconsIcon icon={RefreshIcon} className="size-4" strokeWidth={2} />
            Try again
          </Button>
        </CardContent>
      </Card>
    );
  }

  return (
    <Card className="relative overflow-hidden">
      <CardHeader className="flex flex-col gap-3 sm:flex-row sm:items-center sm:justify-between">
        <div>
          <CardTitle className="text-base">Config Editor</CardTitle>
          <CardDescription>Edit and apply runtime settings.</CardDescription>
        </div>
        <div className="flex items-center gap-2">
          <Button variant="outline" size="sm" onClick={() => refetch()} disabled={isFetching}>
            <HugeiconsIcon icon={RefreshIcon} className="size-4" strokeWidth={2} />
            Refresh
          </Button>
          <Button
            variant="outline"
            size="sm"
            onClick={() => setDraft(config?.content ?? "")}
            disabled={!hasChanges}
          >
            Reset
          </Button>
          <Button
            size="sm"
            onClick={() => updateMutation.mutate(draft)}
            disabled={!hasChanges || updateMutation.isPending || isLoading}
          >
            {updateMutation.isPending ? <Spinner className="size-4" /> : "Save & Apply"}
          </Button>
        </div>
      </CardHeader>
      <CardContent className="space-y-4">
        {isLoading ? (
          <div className="space-y-3">
            <Skeleton className="h-6 w-1/3" />
            <Skeleton className="h-[520px] w-full" />
          </div>
        ) : (
          <CodeMirror
            value={draft}
            onChange={(value) => setDraft(value)}
            height="520px"
            theme={configTheme}
            extensions={editorExtensions}
            basicSetup={{
              lineNumbers: true,
              foldGutter: false,
              highlightActiveLineGutter: true,
            }}
          />
        )}
        <div className="flex flex-wrap items-center justify-between gap-2 text-xs text-muted-foreground">
          <span>{lineCount} lines</span>
          <span>{draft.length} chars</span>
        </div>
      </CardContent>
    </Card>
  );
});

export default ConfigPanel;
