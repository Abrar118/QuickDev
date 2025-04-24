import { Moon, SmartphoneCharging, Sun } from "lucide-react";
import { Button } from "@/components/ui/button";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import { cn } from "@/lib/utils";
import { useTheme } from "next-themes";

export function ModeToggle() {
  const { theme, setTheme } = useTheme();

  return (
    <DropdownMenu>
      <DropdownMenuTrigger asChild>
        <Button variant="outline" size="icon" className="text-chart-5">
          <Sun className="h-[1.2rem] w-[1.2rem] rotate-0 scale-100 transition-all dark:-rotate-90 dark:scale-0" />
          <Moon className="absolute h-[1.2rem] w-[1.2rem] rotate-90 scale-0 transition-all dark:rotate-0 dark:scale-100" />
          <span className="sr-only">Toggle theme</span>
        </Button>
      </DropdownMenuTrigger>
      <DropdownMenuContent align="end" className="bg-popover">
        <DropdownMenuItem
          onClick={() => setTheme("light")}
          className={cn(
            "cursor-pointer hover:bg-chart-5 group",
            theme === "light" && "bg-chart-5"
          )}
        >
          Light
          <Sun className="ml-2 h-4 w-4 text-chart-3 group-hover:text-foreground" />
        </DropdownMenuItem>
        <DropdownMenuItem
          onClick={() => setTheme("dark")}
          className={cn(
            "cursor-pointer hover:bg-chart-5 group",
            theme === "dark" && "bg-chart-5"
          )}
        >
          Dark
          <Moon className="ml-2 h-4 w-4 text-chart-2 group-hover:text-foreground" />
        </DropdownMenuItem>
        <DropdownMenuItem
          onClick={() => setTheme("system")}
          className={cn(
            "cursor-pointer hover:bg-chart-5 group",
            theme === "system" && "bg-chart-5"
          )}
        >
          System
          <SmartphoneCharging className="ml-2 h-4 w-4 text-chart-2 group-hover:text-foreground" />
        </DropdownMenuItem>
      </DropdownMenuContent>
    </DropdownMenu>
  );
}
