import React from "react";
import ReactDOM from "react-dom/client";
import "./index.css";
import { setupWindowEvents } from "./lib/window-events";
import { RouterProvider } from "react-router-dom";
import { router } from "./router";

// Setup window events for minimize to tray functionality
// await setupWindowEvents().catch(console.error);

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <RouterProvider router={router} />
  </React.StrictMode>
);
