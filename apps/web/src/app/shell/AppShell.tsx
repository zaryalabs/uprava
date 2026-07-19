import { LayoutDashboard, Menu, Settings, Wrench } from "lucide-react";
import { useState } from "react";
import { Link, NavLink, Outlet, useSearchParams } from "react-router-dom";

import { useInventory } from "../../features/inventory/api";
import { InventoryTree } from "../../features/inventory/InventoryTree";
import { TrustedProfileBanner } from "../../features/warnings/TrustedProfileBanner";
import { InspectorStack } from "../../workbench/inspector/InspectorStack";
import {
  decodeInspectorStack,
  INSPECT_QUERY_PARAM,
} from "../../workbench/references/refs";
import { preferredSidebarOpen, rememberSidebarOpen } from "./preferences";

export function AppShell() {
  const inventory = useInventory();
  const [searchParams] = useSearchParams();
  const [sidebarOpen, setSidebarOpen] = useState(preferredSidebarOpen);
  const inspectorOpen =
    decodeInspectorStack(searchParams.get(INSPECT_QUERY_PARAM)).length > 0;
  const apiState = inventory.data
    ? "Connected"
    : inventory.isError
      ? "Unavailable"
      : "Connecting";

  return (
    <div
      className={`zarya-sheet uprava-app-shell min-h-screen ${sidebarOpen ? "sidebar-open" : "sidebar-collapsed"} ${inspectorOpen ? "inspector-open" : ""}`}
    >
      <a href="#main-content" className="zarya-skip-link">
        Skip to Main Content
      </a>
      <header className="uprava-topbar">
        <div className="uprava-brand-area">
          <button
            type="button"
            className="inline-flex h-9 w-9 shrink-0 items-center justify-center border border-transparent text-[var(--color-muted)] hover:border-[var(--color-muted)] hover:bg-[var(--color-bg-muted)] hover:text-[var(--color-ink)]"
            aria-controls="workspace-navigation"
            aria-expanded={sidebarOpen}
            aria-label={sidebarOpen ? "Hide navigation" : "Show navigation"}
            title={sidebarOpen ? "Hide navigation" : "Show navigation"}
            onClick={() => {
              setSidebarOpen((current) => {
                const next = !current;
                rememberSidebarOpen(next);
                return next;
              });
            }}
          >
            <Menu size={16} aria-hidden="true" />
          </button>
          <Link to="/dashboard" className="uprava-brand-link">
            <span
              aria-hidden="true"
              className="grid h-6 w-6 place-items-center border border-[var(--color-ink)] text-xs"
            >
              У
            </span>
            <span>Uprava</span>
          </Link>
        </div>
        <div className="flex min-w-0 items-center justify-between gap-4 px-4 text-xs text-[var(--color-muted)] max-md:justify-end">
          <span className="truncate max-md:hidden">
            Control Plane / Work Surface
          </span>
          <span className="flex shrink-0 items-center gap-2">
            <span
              aria-hidden="true"
              className={`h-2 w-2 border ${inventory.isError ? "border-[var(--color-risk)] bg-[var(--color-risk)]" : "border-[var(--color-ink)]"}`}
            />
            API {apiState}
          </span>
        </div>
        <div className="flex h-full items-center justify-end border-l border-black/10 px-3">
          <Link
            to="/settings/tooling"
            className="inline-flex h-8 items-center gap-2 border border-transparent px-2 text-xs hover:border-[var(--color-muted)] hover:bg-[var(--color-bg-muted)]"
            aria-label="Agent Tooling"
          >
            <Wrench size={15} aria-hidden="true" />
            <span className="max-sm:sr-only">Tooling</span>
          </Link>
          <Link
            to="/settings/runtime"
            className="inline-flex h-8 items-center gap-2 border border-transparent px-2 text-xs hover:border-[var(--color-muted)] hover:bg-[var(--color-bg-muted)]"
            aria-label="Runtime Settings"
          >
            <Settings size={15} aria-hidden="true" />
            <span className="max-sm:sr-only">Settings</span>
          </Link>
        </div>
      </header>
      <TrustedProfileBanner />
      <div className="uprava-shell-grid">
        <aside
          id="workspace-navigation"
          className="uprava-sidebar"
          aria-label="Node and workspace navigation"
          hidden={!sidebarOpen}
        >
          <nav
            aria-label="Primary Navigation"
            className="mb-5 grid gap-1 border-b border-black/10 pb-4"
          >
            <NavLink
              to="/dashboard"
              className={({ isActive }) =>
                `flex min-h-9 items-center gap-2 border-l px-2 text-sm hover:bg-[var(--color-bg-muted)] ${
                  isActive
                    ? "border-[var(--color-ink)] font-bold text-[var(--color-ink)]"
                    : "border-transparent text-[var(--color-muted)]"
                }`
              }
            >
              <LayoutDashboard size={15} aria-hidden="true" />
              <span>Dashboard</span>
            </NavLink>
          </nav>
          <InventoryTree />
        </aside>
        <main
          id="main-content"
          className="uprava-main min-w-0 px-5 py-8 lg:px-8"
          tabIndex={-1}
        >
          <Outlet />
        </main>
        {inspectorOpen ? (
          <aside
            className="uprava-context-inspector"
            aria-label="Context Inspector"
          >
            <InspectorStack />
          </aside>
        ) : null}
      </div>
    </div>
  );
}
