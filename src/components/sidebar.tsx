import { Link, useLocation } from "react-router-dom";
import { cn } from "../lib/utils";
import {
  LayoutDashboard,
  FolderKanban,
  CheckSquare,
  Timer,
  Settings,
  Code,
  ChevronDown,
} from "lucide-react";

const navItems = [
  { path: "/", label: "Dashboard", icon: LayoutDashboard },
  { path: "/projects", label: "Projects", icon: FolderKanban },
  { path: "/tasks", label: "Tasks", icon: CheckSquare },
  { path: "/timer", label: "Work Timer", icon: Timer },
  { path: "/settings", label: "Settings", icon: Settings },
];

export default function Sidebar() {
  const location = useLocation();

  return (
    <div className="w-64 border-r bg-card h-screen flex flex-col">
      <div className="p-4 border-b flex items-center gap-2">
        <Code className="h-6 w-6 text-primary" />
        <div>
          <h1 className="text-xl font-bold">QuickDev</h1>
          <div className="flex items-center text-xs text-muted-foreground">
            <span>Personal Workspace</span>
            <ChevronDown className="h-3 w-3 ml-1" />
          </div>
        </div>
      </div>
      <nav className="flex-1 p-4 overflow-auto">
        <div className="mb-4">
          <h2 className="text-xs uppercase font-semibold text-muted-foreground tracking-wider mb-2 px-3">
            Main
          </h2>
          <ul className="space-y-1">
            {navItems.slice(0, 4).map((item) => (
              <li key={item.path}>
                <Link
                  to={item.path}
                  className={cn(
                    "flex items-center gap-3 px-3 py-2 rounded-md text-sm transition-colors",
                    location.pathname === item.path
                      ? "bg-primary text-primary-foreground"
                      : "text-muted-foreground hover:bg-muted hover:text-foreground"
                  )}
                >
                  <item.icon className="h-5 w-5" />
                  {item.label}
                </Link>
              </li>
            ))}
          </ul>
        </div>

        <div>
          <h2 className="text-xs uppercase font-semibold text-muted-foreground tracking-wider mb-2 px-3">
            Settings
          </h2>
          <ul className="space-y-1">
            {navItems.slice(4).map((item) => (
              <li key={item.path}>
                <Link
                  to={item.path}
                  className={cn(
                    "flex items-center gap-3 px-3 py-2 rounded-md text-sm transition-colors",
                    location.pathname === item.path
                      ? "bg-primary text-primary-foreground"
                      : "text-muted-foreground hover:bg-muted hover:text-foreground"
                  )}
                >
                  <item.icon className="h-5 w-5" />
                  {item.label}
                </Link>
              </li>
            ))}
          </ul>
        </div>
      </nav>

      <div className="p-4 border-t">
        <div className="flex items-center gap-3 px-3 py-2 rounded-md bg-muted/50">
          <div className="h-8 w-8 rounded-full bg-primary/10 flex items-center justify-center text-primary font-medium">
            JD
          </div>
          <div className="flex-1 min-w-0">
            <p className="text-sm font-medium truncate">John Doe</p>
            <p className="text-xs text-muted-foreground truncate">
              john@example.com
            </p>
          </div>
        </div>
      </div>
    </div>
  );
}
