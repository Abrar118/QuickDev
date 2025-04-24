import { getCurrentWindow } from "@tauri-apps/api/window";
import { minimizeToTray } from "./tauri-api";
import { listen } from "@tauri-apps/api/event";
import { getSettings } from "./tauri-api";

export async function setupWindowEvents() {
  try {
    // Load settings to check if minimize to tray is enabled
    const settings = await getSettings();
    const minimizeToTrayEnabled =
      settings?.general?.startup?.minimizeToTray || false;

    // Listen for window close event
    await listen("tauri://close-requested", async (event) => {
      if (minimizeToTrayEnabled) {
        // Prevent the window from closing
        event.preventDefault();

        // Minimize to tray instead
        await minimizeToTray();
      } else {
        // Close the window normally
        const appWindow = getCurrentWindow();
        await appWindow.close();
      }
    });

    // Listen for window minimize event
    await listen("tauri://window-minimized", async (_event) => {
      if (minimizeToTrayEnabled) {
        // Minimize to tray instead of minimizing the window
        await minimizeToTray();
      } else {
        const appWindow = getCurrentWindow();
        await appWindow.minimize();
      }
    });
  } catch (error) {
    console.error("Failed to setup window events:", error);
  }
}
