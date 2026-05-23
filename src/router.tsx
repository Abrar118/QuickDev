import { createBrowserRouter } from "react-router-dom";
import { Toaster } from "./lib/toast";
import Layout from "./components/layout";
import { ThemeProvider } from "./components/theme-provider";
import Dashboard from "./pages/dashboard";
import ProjectManagement from "./pages/project-management";
import Settings from "./pages/settings";
import WorkTimer from "./pages/work-timer";
import ErrorPage from "./pages/error-page";

const router = createBrowserRouter([
  {
    path: "/",
    errorElement: (
      <ThemeProvider
        attribute="class"
        defaultTheme="system"
        enableSystem
        disableTransitionOnChange
      >
        <ErrorPage />
        <Toaster position="top-center" />
      </ThemeProvider>
    ),
    element: (
      <ThemeProvider
        attribute="class"
        defaultTheme="system"
        enableSystem
        disableTransitionOnChange
      >
        <Layout />
        <Toaster position="top-center" />
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
        path: "settings",
        element: <Settings />,
      },
      {
        path: "timer",
        element: <WorkTimer />,
      },
    ],
  },
]);

export { router };
