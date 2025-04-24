import { createBrowserRouter } from "react-router-dom";
import { Toaster } from "./components/ui/sonner";
import Layout from "./components/layout";
import { ThemeProvider } from "./components/theme-provider";
import Dashboard from "./pages/dashboard";
import ProjectManagement from "./pages/project-management";
import TaskManagement from "./pages/task-management";
import WorkTimer from "./pages/work-timer";
import Settings from "./pages/settings";

const router = createBrowserRouter([
  {
    path: "/",
    element: (
      <ThemeProvider
        attribute="class"
        defaultTheme="system"
        enableSystem
        disableTransitionOnChange
      >
        <Layout />
        <Toaster />
      </ThemeProvider>
    ),
    children: [
      {
        index: true,
        element: <Dashboard />,
      },
      {
        path: "projects",
        element: <ProjectManagement />,
      },
      {
        path: "tasks",
        element: <TaskManagement />,
      },
      {
        path: "timer",
        element: <WorkTimer />,
      },
      {
        path: "settings",
        element: <Settings />,
      },
    ],
  },
]);

export { router };
