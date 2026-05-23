import { Link, useLocation } from "react-router-dom";
import { cn } from "../lib/utils";
import {
  LayoutDashboard,
  FolderKanban,
  Settings,
  Code,
  Clock3,
} from "lucide-react";

const navItems = [
  { path: "/", label: "Overview", icon: LayoutDashboard },
  { path: "/projects", label: "Projects", icon: FolderKanban },
  { path: "/timer", label: "Work Timer", icon: Clock3 },
  { path: "/settings", label: "Settings", icon: Settings },
];

export default function Sidebar() {
  const location = useLocation();

  return (
    <div className="w-[17rem] border-r border-sidebar-border bg-sidebar h-screen flex flex-col">
      <div className="px-4 py-4 border-b border-sidebar-border flex items-center gap-3">
        <div className="h-9 w-9 rounded-lg bg-primary/20 flex items-center justify-center border border-primary/30">
          <Code className="h-5 w-5 text-primary" />
        </div>
        <div>
          <h1 className="text-lg font-semibold tracking-tight">QuickDev</h1>
          <p className="text-xs text-muted-foreground">Project launcher</p>
        </div>
      </div>

      <nav className="flex-1 px-3 py-4 overflow-auto">
        <ul className="space-y-1.5">
          {navItems.map((item) => (
            <li key={item.path}>
              <Link
                to={item.path}
                className={cn(
                  "flex items-center gap-3 px-3 py-2.5 rounded-lg text-sm transition-colors border",
                  location.pathname === item.path
                    ? "bg-sidebar-primary text-sidebar-primary-foreground border-sidebar-primary/70 shadow-md shadow-primary/20"
                    : "text-sidebar-foreground/80 border-transparent hover:bg-sidebar-accent hover:border-sidebar-border hover:text-sidebar-foreground"
                )}
              >
                <item.icon className="h-4 w-4" />
                {item.label}
              </Link>
            </li>
          ))}
        </ul>
      </nav>

      <div className="px-4 py-3 border-t border-sidebar-border text-xs text-muted-foreground">
        Local first. No cloud dependency.
      </div>
    </div>
  );
}
