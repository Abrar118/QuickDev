import { Outlet } from "react-router-dom";
import Sidebar from "./sidebar";
import { ModeToggle } from "./mode-toggle";
import { Bell, Search } from "lucide-react";
import { Button } from "./ui/button";
import { Input } from "./ui/input";
import { useEffect, useState } from "react";
import { cn } from "@/lib/utils";

export default function Layout() {
  const [scrolled, setScrolled] = useState(false);

  useEffect(() => {
    const handleScroll = () => {
      const isScrolled = window.scrollY > 10;
      if (isScrolled !== scrolled) {
        setScrolled(isScrolled);
      }
    };

    // Add scroll event listener
    window.addEventListener("scroll", handleScroll, { passive: true });

    // Initial check
    handleScroll();

    // Clean up
    return () => {
      window.removeEventListener("scroll", handleScroll);
    };
  }, [scrolled]);

  return (
    <div className="flex h-screen bg-background">
      {/* Sidebar */}
      <Sidebar />

      <div className="flex flex-col flex-1 overflow-hidden">
        {/* Header */}
        <header
          className={cn(
            "flex items-center justify-between h-14 px-6 border-b sticky top-0 z-10 transition-all duration-200",
            scrolled
              ? "bg-card/80 backdrop-blur-md border-b shadow-sm"
              : "bg-card"
          )}
        >
          <div className="relative w-full max-w-md hidden md:block">
            <Search className="absolute left-2.5 top-2.5 h-4 w-4 text-muted-foreground" />
            <Input
              placeholder="Search projects, tasks..."
              className="pl-8 w-full bg-muted/40 border-none"
            />
          </div>
          <div className="flex items-center ml-auto gap-2">
            <Button variant="ghost" size="icon" className="relative">
              <Bell className="h-5 w-5" />
              <span className="absolute top-1 right-1 h-2 w-2 rounded-full bg-primary" />
            </Button>
            <ModeToggle />
          </div>
        </header>

        {/* main content */}
        <main
          className="flex-1 overflow-auto p-6"
          onScroll={(e) => {
            const target = e.currentTarget;
            setScrolled(target.scrollTop > 10);
          }}
        >
          <Outlet />
        </main>
      </div>
    </div>
  );
}
