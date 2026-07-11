import * as monaco from "monaco-editor/esm/vs/editor/editor.api.js";
import { useEffect, useRef } from "react";

import { ensureMonacoEnvironment } from "../../shared/monaco/setup";

export function MonacoFileEditor({
  placementId,
  path,
  value,
  readOnly,
  onChange,
}: {
  placementId: string;
  path: string;
  value: string;
  readOnly: boolean;
  onChange: (content: string) => void;
}) {
  const containerRef = useRef<HTMLDivElement | null>(null);
  const editorRef = useRef<monaco.editor.IStandaloneCodeEditor | null>(null);
  const modelRef = useRef<monaco.editor.ITextModel | null>(null);
  const onChangeRef = useRef(onChange);
  onChangeRef.current = onChange;

  useEffect(() => {
    ensureMonacoEnvironment();
    const language = languageForPath(path);
    void loadLanguage(language);
    const container = containerRef.current;
    if (!container) return;
    const uri = monaco.Uri.from({
      scheme: "uprava",
      authority: "workspace",
      path: `/${encodeURIComponent(placementId)}/${path}`,
    });
    const existingModel = monaco.editor.getModel(uri);
    const model =
      existingModel ?? monaco.editor.createModel(value, language, uri);
    if (model.getLanguageId() !== language) {
      monaco.editor.setModelLanguage(model, language);
    }
    if (model.getValue() !== value) model.setValue(value);
    modelRef.current = model;
    const editor = monaco.editor.create(container, {
      model,
      readOnly,
      automaticLayout: true,
      fontFamily:
        "ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace",
      fontSize: 12,
      lineHeight: 20,
      minimap: { enabled: false },
      scrollBeyondLastLine: false,
      wordWrap: "off",
      tabSize: 2,
    });
    editorRef.current = editor;
    const subscription = model.onDidChangeContent(() => {
      onChangeRef.current(model.getValue());
    });
    return () => {
      subscription.dispose();
      editor.dispose();
      editorRef.current = null;
      modelRef.current = null;
      model.dispose();
    };
  }, [path, placementId]);

  useEffect(() => {
    editorRef.current?.updateOptions({ readOnly });
  }, [readOnly]);

  useEffect(() => {
    const model = modelRef.current;
    if (model && model.getValue() !== value) model.setValue(value);
  }, [value]);

  return (
    <div
      ref={containerRef}
      className="min-h-0 flex-1"
      role="region"
      aria-label={`File editor ${path}`}
    />
  );
}

export function MonacoDiffTextViewer({ value }: { value: string }) {
  const containerRef = useRef<HTMLDivElement | null>(null);
  const modelRef = useRef<monaco.editor.ITextModel | null>(null);
  const modelIdRef = useRef(crypto.randomUUID());

  useEffect(() => {
    ensureMonacoEnvironment();
    const container = containerRef.current;
    if (!container) return;
    const model = monaco.editor.createModel(
      value,
      "plaintext",
      monaco.Uri.parse(`uprava://workspace/diff/${modelIdRef.current}`),
    );
    modelRef.current = model;
    const editor = monaco.editor.create(container, {
      model,
      readOnly: true,
      automaticLayout: true,
      fontFamily:
        "ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace",
      fontSize: 12,
      lineHeight: 20,
      minimap: { enabled: false },
      scrollBeyondLastLine: false,
      wordWrap: "off",
    });
    return () => {
      editor.dispose();
      model.dispose();
      modelRef.current = null;
    };
  }, []);

  useEffect(() => {
    const model = modelRef.current;
    if (model && model.getValue() !== value) model.setValue(value);
  }, [value]);

  return (
    <div
      ref={containerRef}
      className="h-80 overflow-hidden rounded-md border border-[#1f2a22]"
    />
  );
}

function languageForPath(path: string) {
  const extension = path.split(".").pop()?.toLowerCase() ?? "";
  const languages: Record<string, string> = {
    css: "css",
    html: "html",
    htm: "html",
    js: "javascript",
    jsx: "javascript",
    mjs: "javascript",
    cjs: "javascript",
    json: "json",
    jsonc: "json",
    md: "markdown",
    mdx: "markdown",
    rs: "rust",
    toml: "plaintext",
    ts: "typescript",
    tsx: "typescript",
    mts: "typescript",
    cts: "typescript",
    yaml: "yaml",
    yml: "yaml",
  };
  return languages[extension] ?? "plaintext";
}

async function loadLanguage(language: string) {
  switch (language) {
    case "css":
      await import("monaco-editor/esm/vs/basic-languages/css/css.contribution.js");
      break;
    case "html":
      await import("monaco-editor/esm/vs/basic-languages/html/html.contribution.js");
      break;
    case "javascript":
    case "typescript":
      await import("monaco-editor/esm/vs/basic-languages/typescript/typescript.contribution.js");
      break;
    case "json":
      await import("monaco-editor/esm/vs/language/json/monaco.contribution.js");
      break;
    case "markdown":
      await import("monaco-editor/esm/vs/basic-languages/markdown/markdown.contribution.js");
      break;
    case "rust":
      await import("monaco-editor/esm/vs/basic-languages/rust/rust.contribution.js");
      break;
    case "shell":
      await import("monaco-editor/esm/vs/basic-languages/shell/shell.contribution.js");
      break;
    case "yaml":
      await import("monaco-editor/esm/vs/basic-languages/yaml/yaml.contribution.js");
      break;
  }
}
