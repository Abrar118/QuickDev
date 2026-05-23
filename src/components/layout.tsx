import { Outlet } from "react-router-dom";
import Sidebar from "./sidebar";
import { ModeToggle } from "./mode-toggle";
import { Search } from "lucide-react";
import { Input } from "./ui/input";

export default function Layout() {
  return (
    <div className="flex h-screen bg-background text-foreground">
      <Sidebar />

      <div className="flex flex-col flex-1 overflow-hidden">
        <header className="flex items-center justify-between h-16 px-5 border-b border-border/80 bg-card/70 backdrop-blur-sm">
          <div className="relative w-full max-w-md hidden md:block">
            <Search className="absolute left-3 top-2.5 h-4 w-4 text-muted-foreground" />
            <Input
              placeholder="Search projects..."
              className="pl-9 w-full bg-muted/40 border-border/70"
            />
          </div>

          <div className="flex items-center ml-auto gap-2">
            <ModeToggle />
          </div>
        </header>

        <main className="flex-1 overflow-auto p-5">
          <Outlet />
        </main>
      </div>
    </div>
  );
}
