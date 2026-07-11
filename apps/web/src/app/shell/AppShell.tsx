import { LayoutDashboard, Server, Settings } from "lucide-react";
import type { ReactNode } from "react";
import { Link, NavLink, Outlet } from "react-router-dom";

import { useInventory } from "../../features/inventory/api";
import { InventoryTree } from "../../features/inventory/InventoryTree";
import { TrustedProfileBanner } from "../../features/warnings/TrustedProfileBanner";
import { InspectorStack } from "../../workbench/inspector/InspectorStack";

export function AppShell() {
  const inventory = useInventory();
  const apiState = inventory.data
    ? "Connected"
    : inventory.isError
      ? "Unavailable"
      : "Connecting";

  return (
    <div className="zarya-sheet min-h-screen">
      <a href="#main-content" className="zarya-skip-link">
        Skip to Main Content
      </a>
      <header className="grid min-h-12 grid-cols-[248px_minmax(0,1fr)_320px] items-center border-b border-black/10 max-xl:grid-cols-[224px_minmax(0,1fr)] max-md:grid-cols-[1fr_auto]">
        <Link
          to="/dashboard"
          className="flex h-full items-center gap-3 border-r border-black/10 px-4 font-bold max-md:border-r-0"
        >
          <span
            aria-hidden="true"
            className="grid h-6 w-6 place-items-center border border-[var(--color-ink)] text-xs"
          >
            У
          </span>
          <span>Uprava</span>
        </Link>
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
        <div className="flex h-full items-center justify-end border-l border-black/10 px-3 max-xl:hidden">
          <Link
            to="/settings/runtime"
            className="inline-flex h-8 items-center gap-2 border border-transparent px-2 text-xs hover:border-[var(--color-muted)] hover:bg-[var(--color-bg-muted)]"
            aria-label="Runtime Settings"
          >
            <Settings size={15} aria-hidden="true" />
            Settings
          </Link>
        </div>
      </header>
      <TrustedProfileBanner />
      <div className="grid min-h-[calc(100vh-48px)] grid-cols-[248px_minmax(0,1fr)_320px] max-xl:grid-cols-[224px_minmax(0,1fr)] max-md:grid-cols-1">
        <aside
          className="border-r border-black/10 px-3 py-4 max-md:border-b max-md:border-r-0"
          aria-label="Workspace Navigation"
        >
          <nav aria-label="Primary Navigation" className="mb-6 grid gap-1">
            <SidebarLink
              to="/dashboard"
              icon={<LayoutDashboard size={15} aria-hidden="true" />}
            >
              Dashboard
            </SidebarLink>
            <SidebarLink
              to="/nodes"
              icon={<Server size={15} aria-hidden="true" />}
            >
              Nodes
            </SidebarLink>
          </nav>
          <InventoryTree />
        </aside>
        <main
          id="main-content"
          className="min-w-0 px-5 py-8 lg:px-8"
          tabIndex={-1}
        >
          <Outlet />
        </main>
        <aside
          className="border-l border-black/10 px-4 py-6 max-xl:hidden"
          aria-label="Inspector"
        >
          <InspectorStack />
        </aside>
      </div>
    </div>
  );
}

function SidebarLink({
  to,
  icon,
  children,
}: {
  to: string;
  icon: ReactNode;
  children: ReactNode;
}) {
  return (
    <NavLink
      to={to}
      className={({ isActive }) =>
        `flex min-h-9 items-center gap-2 border-l px-2 text-sm hover:bg-[var(--color-bg-muted)] ${
          isActive
            ? "border-[var(--color-ink)] font-bold text-[var(--color-ink)]"
            : "border-transparent text-[var(--color-muted)]"
        }`
      }
    >
      {icon}
      <span>{children}</span>
    </NavLink>
  );
}
