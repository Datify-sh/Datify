import { HighlightStyle, syntaxHighlighting } from "@codemirror/language";
import { EditorView } from "@codemirror/view";
import { tags as t } from "@lezer/highlight";

export const editorThemeDark = EditorView.theme(
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
  },
  { dark: true },
);

export const editorThemeLight = EditorView.theme(
  {
    "&": {
      backgroundColor: "transparent",
      color: "#0f172a",
      fontFamily:
        'ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, "Liberation Mono", "Courier New", monospace',
      fontSize: "12px",
    },
    ".cm-scroller": {
      backgroundColor: "#f8fafc",
    },
    ".cm-content": {
      padding: "16px",
      caretColor: "#0f766e",
    },
    ".cm-selectionBackground": {
      backgroundColor: "rgba(14, 116, 144, 0.18)",
    },
    ".cm-activeLine": {
      backgroundColor: "rgba(15, 23, 42, 0.04)",
    },
    ".cm-gutters": {
      backgroundColor: "#f1f5f9",
      color: "#94a3b8",
      border: "none",
    },
    ".cm-activeLineGutter": {
      color: "#0f172a",
    },
  },
  { dark: false },
);

const darkHighlightStyle = HighlightStyle.define([
  { tag: t.comment, color: "#64748b", fontStyle: "italic" },
  { tag: t.keyword, color: "#7dd3fc" },
  { tag: t.operatorKeyword, color: "#7dd3fc" },
  { tag: t.controlKeyword, color: "#7dd3fc" },
  { tag: t.definitionKeyword, color: "#7dd3fc" },
  { tag: t.moduleKeyword, color: "#7dd3fc" },
  { tag: t.string, color: "#a7f3d0" },
  { tag: t.special(t.string), color: "#a7f3d0" },
  { tag: t.number, color: "#facc15" },
  { tag: t.bool, color: "#fbbf24" },
  { tag: t.null, color: "#fbbf24" },
  { tag: t.variableName, color: "#e2e8f0" },
  { tag: t.propertyName, color: "#cbd5f5" },
  { tag: t.typeName, color: "#93c5fd" },
  { tag: t.className, color: "#93c5fd" },
  { tag: t.function(t.variableName), color: "#fdba74" },
  { tag: t.labelName, color: "#cbd5f5" },
  { tag: t.operator, color: "#94a3b8" },
  { tag: t.punctuation, color: "#94a3b8" },
  { tag: t.meta, color: "#94a3b8" },
]);

const lightHighlightStyle = HighlightStyle.define([
  { tag: t.comment, color: "#64748b", fontStyle: "italic" },
  { tag: t.keyword, color: "#0f766e" },
  { tag: t.operatorKeyword, color: "#0f766e" },
  { tag: t.controlKeyword, color: "#0f766e" },
  { tag: t.definitionKeyword, color: "#0f766e" },
  { tag: t.moduleKeyword, color: "#0f766e" },
  { tag: t.string, color: "#059669" },
  { tag: t.special(t.string), color: "#059669" },
  { tag: t.number, color: "#b45309" },
  { tag: t.bool, color: "#a16207" },
  { tag: t.null, color: "#a16207" },
  { tag: t.variableName, color: "#0f172a" },
  { tag: t.propertyName, color: "#1d4ed8" },
  { tag: t.typeName, color: "#2563eb" },
  { tag: t.className, color: "#2563eb" },
  { tag: t.function(t.variableName), color: "#c2410c" },
  { tag: t.labelName, color: "#1d4ed8" },
  { tag: t.operator, color: "#64748b" },
  { tag: t.punctuation, color: "#64748b" },
  { tag: t.meta, color: "#64748b" },
]);

export const editorHighlightDark = syntaxHighlighting(darkHighlightStyle);
export const editorHighlightLight = syntaxHighlighting(lightHighlightStyle);
