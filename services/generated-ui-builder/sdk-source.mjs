export const SDK_SOURCE = String.raw`
import React, { useCallback, useSyncExternalStore } from "react";

let port = null;
let nextRequestId = 1;
let snapshot = {
  artifact: null,
  dataModel: null,
  state: { revision: 0, values: null },
  layout: "canvas",
  actions: [],
};
const listeners = new Set();
const pending = new Map();

function emit() {
  for (const listener of listeners) listener();
}

function request(type, payload) {
  if (!port) return Promise.reject(new Error("Uprava UI bridge is not ready"));
  const requestId = String(nextRequestId++);
  return new Promise((resolve, reject) => {
    pending.set(requestId, { resolve, reject });
    port.postMessage({ protocol: 1, type, requestId, payload });
  });
}

export function __initializeUpravaUi(nextPort, initialSnapshot) {
  if (port) return;
  port = nextPort;
  snapshot = initialSnapshot;
  port.onmessage = (event) => {
    const message = event.data;
    if (!message || message.protocol !== 1 || typeof message.type !== "string") return;
    if (message.type === "host.snapshot") {
      snapshot = message.payload;
      emit();
      return;
    }
    if (message.type !== "host.response" || typeof message.requestId !== "string") return;
    const waiter = pending.get(message.requestId);
    if (!waiter) return;
    pending.delete(message.requestId);
    if (message.ok) waiter.resolve(message.payload);
    else waiter.reject(new Error(message.error || "Uprava action failed"));
  };
  port.start();
}

function subscribe(listener) {
  listeners.add(listener);
  return () => listeners.delete(listener);
}

function getSnapshot() {
  return snapshot;
}

export function useArtifactData() {
  return useSyncExternalStore(subscribe, getSnapshot, getSnapshot).dataModel;
}

export function usePersistedState() {
  const current = useSyncExternalStore(subscribe, getSnapshot, getSnapshot).state;
  const save = useCallback(
    (values) => request("state.update", { expected_revision: current.revision, values }),
    [current.revision],
  );
  return [current.values, save, current.revision];
}

export function useAction(actionId) {
  const actions = useSyncExternalStore(subscribe, getSnapshot, getSnapshot).actions;
  const definition = actions.find((action) => action.action_id === actionId) || null;
  const invoke = useCallback(
    (input, options = {}) => request("action.invoke", {
      actionId,
      input,
      confirmed: options.confirmed === true,
    }),
    [actionId],
  );
  return { definition, invoke, available: definition !== null };
}

export function useContainer() {
  const current = useSyncExternalStore(subscribe, getSnapshot, getSnapshot);
  const requestLayout = useCallback(
    (layout) => request("layout.request", { layout }),
    [],
  );
  return { layout: current.layout, requestLayout };
}

export function Stack({ children, gap = "md", className = "" }) {
  return <div className={"uprava-stack uprava-gap-" + gap + " " + className}>{children}</div>;
}

export function Row({ children, className = "" }) {
  return <div className={"uprava-row " + className}>{children}</div>;
}

export function Section({ title, children }) {
  return <section className="uprava-section">{title ? <h2>{title}</h2> : null}{children}</section>;
}

export function Card({ children, className = "" }) {
  return <div className={"uprava-card " + className}>{children}</div>;
}

export function Heading({ children, level = 2 }) {
  const Tag = level === 1 ? "h1" : level === 3 ? "h3" : "h2";
  return <Tag>{children}</Tag>;
}

export function Text({ children, muted = false }) {
  return <p className={muted ? "uprava-muted" : ""}>{children}</p>;
}

export function Badge({ children, tone = "neutral" }) {
  return <span className={"uprava-badge uprava-badge-" + tone}>{children}</span>;
}

export function Button({ children, ...props }) {
  return <button {...props} type="button">{children}</button>;
}

export function TextInput(props) {
  return <input {...props} type="text" />;
}

export function NumberInput(props) {
  return <input {...props} type="number" />;
}

export function Select({ children, ...props }) {
  return <select {...props}>{children}</select>;
}

export function Checkbox(props) {
  return <input {...props} type="checkbox" />;
}

export function Table({ columns, rows }) {
  return <div className="uprava-table-wrap"><table><thead><tr>{columns.map((column) => <th key={column.key}>{column.label}</th>)}</tr></thead><tbody>{rows.map((row, index) => <tr key={row.id || index}>{columns.map((column) => <td key={column.key}>{String(row[column.key] ?? "")}</td>)}</tr>)}</tbody></table></div>;
}
`;
