import { Activity, LayoutDashboard, Server, Settings } from "lucide-react";
import { Outlet, Link, NavLink } from "react-router-dom";
import type { ReactNode } from "react";

import { useInventory } from "../../features/inventory/api";
import { InventoryTree } from "../../features/inventory/InventoryTree";
import { TrustedProfileBanner } from "../../features/warnings/TrustedProfileBanner";
import { InspectorStack } from "../../workbench/inspector/InspectorStack";

export function AppShell() {
  const inventory = useInventory();

  return (
    <div className="min-h-screen bg-[#f7f8f4] text-[#17211c]">
      <header className="flex h-12 items-center justify-between border-b border-[#d9ded4] bg-[#fbfcf8] px-4">
        <Link to="/dashboard" className="flex items-center gap-2 font-semibold">
          <Activity size={18} />
          Cortex
        </Link>
        <div className="flex items-center gap-3 text-sm text-[#536257]">
          <span>API {inventory.data ? "connected" : "pending"}</span>
          <Link
            to="/settings/runtime"
            className="inline-flex h-8 w-8 items-center justify-center rounded-md hover:bg-[#e7ebe3]"
            aria-label="Runtime settings"
          >
            <Settings size={16} />
          </Link>
        </div>
      </header>
      <TrustedProfileBanner />
      <div className="grid min-h-[calc(100vh-88px)] grid-cols-[280px_minmax(0,1fr)_320px] max-lg:grid-cols-[240px_minmax(0,1fr)] max-md:grid-cols-1">
        <aside className="border-r border-[#d9ded4] bg-[#eef2ea] p-3 max-md:border-b max-md:border-r-0">
          <nav aria-label="Primary navigation" className="mb-4 space-y-1">
            <SidebarLink to="/dashboard" icon={<LayoutDashboard size={15} />}>
              Dashboard
            </SidebarLink>
            <SidebarLink to="/nodes" icon={<Server size={15} />}>
              Nodes
            </SidebarLink>
          </nav>
          <InventoryTree />
        </aside>
        <main className="min-w-0 p-5">
          <Outlet />
        </main>
        <aside className="border-l border-[#d9ded4] bg-[#fbfcf8] p-3 max-lg:hidden">
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
        `flex min-h-9 items-center gap-2 rounded-md px-2 text-sm font-medium hover:bg-[#e2e8dd] ${
          isActive ? "bg-[#dfe8dc] text-[#1d4f3a]" : "text-[#27362f]"
        }`
      }
    >
      {icon}
      <span>{children}</span>
    </NavLink>
  );
}
