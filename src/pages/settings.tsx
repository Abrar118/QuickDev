"use client"

import { useState, useEffect } from "react"
import { Button } from "../components/ui/button"
import { Card, CardContent, CardDescription, CardFooter, CardHeader, CardTitle } from "../components/ui/card"
import { Input } from "../components/ui/input"
import { Label } from "../components/ui/label"
import { Switch } from "../components/ui/switch"
import { Tabs, TabsContent, TabsList, TabsTrigger } from "../components/ui/tabs"
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "../components/ui/select"
import { Slider } from "../components/ui/slider"
import { toast } from "sonner"
import {
  getSettings,
  saveGeneralSettings,
  saveTimerSettings,
  saveThemeSettings,
  saveDataSettings,
  backupData,
  restoreData,
  showSaveDialog,
} from "../lib/tauri-api"

export default function Settings() {
  // General settings
  const [defaultApplicationPaths, setDefaultApplicationPaths] = useState({
    editor: "/usr/bin/code",
    browser: "/usr/bin/google-chrome",
    terminal: "/usr/bin/gnome-terminal",
  })

  const [startupSettings, setStartupSettings] = useState({
    launchOnStartup: true,
    minimizeToTray: true,
    reopenLastProject: false,
  })

  // Timer settings
  const [timerSettings, setTimerSettings] = useState({
    pomodoroLength: 25,
    shortBreakLength: 5,
    longBreakLength: 15,
    autoStartBreaks: false,
    autoStartPomodoros: false,
    showNotifications: true,
    playSound: true,
  })

  // Theme settings
  const [themeSettings, setThemeSettings] = useState({
    theme: "system",
    accentColor: "#3b82f6",
  })

  // Data settings
  const [dataSettings, setDataSettings] = useState({
    dataDirectory: "~/Music/QuickDev",
    autoBackup: true,
    backupFrequency: "daily",
  })

  const [isLoading, setIsLoading] = useState(true)

  useEffect(() => {
    const loadSettings = async () => {
      try {
        const settings = await getSettings()

        if (settings.general) {
          setDefaultApplicationPaths(settings.general.default_application_paths)
          setStartupSettings(settings.general.startup)
        }

        if (settings.timer) {
          setTimerSettings({
            pomodoroLength: settings.timer.pomodoro_length,
            shortBreakLength: settings.timer.short_break_length,
            longBreakLength: settings.timer.long_break_length,
            autoStartBreaks: settings.timer.auto_start_breaks,
            autoStartPomodoros: settings.timer.auto_start_pomodoros,
            showNotifications: settings.timer.show_notifications,
            playSound: settings.timer.play_sound,
          })
        }

        if (settings.theme) {
          setThemeSettings({
            theme: settings.theme.theme,
            accentColor: settings.theme.accent_color,
          })
        }

        if (settings.data) {
          setDataSettings({
            dataDirectory: settings.data.data_directory,
            autoBackup: settings.data.auto_backup,
            backupFrequency: settings.data.backup_frequency,
          })
        }

        setIsLoading(false)
      } catch (error) {
        console.error("Failed to load settings:", error)
        toast.error("Failed to load settings", {
          description: error instanceof Error ? error.message : "Unknown error occurred",
        })
        setIsLoading(false)
      }
    }

    loadSettings()
  }, [])

  const handleSaveGeneralSettings = async () => {
    try {
      const generalSettings = {
        default_application_paths: {
          editor: defaultApplicationPaths.editor,
          browser: defaultApplicationPaths.browser,
          terminal: defaultApplicationPaths.terminal,
        },
        startup: startupSettings,
      }

      await saveGeneralSettings(generalSettings)

      toast.success("Settings saved", {
        description: "Your general settings have been saved successfully.",
      })
    } catch (error) {
      toast.error("Failed to save settings", {
        description: error instanceof Error ? error.message : "Unknown error occurred",
      })
    }
  }

  const handleSaveTimerSettings = async () => {
    try {
      const settings = {
        pomodoro_length: timerSettings.pomodoroLength,
        short_break_length: timerSettings.shortBreakLength,
        long_break_length: timerSettings.longBreakLength,
        auto_start_breaks: timerSettings.autoStartBreaks,
        auto_start_pomodoros: timerSettings.autoStartPomodoros,
        show_notifications: timerSettings.showNotifications,
        play_sound: timerSettings.playSound,
      }

      await saveTimerSettings(settings)

      toast.success("Timer settings saved", {
        description: "Your timer settings have been saved successfully.",
      })
    } catch (error) {
      toast.error("Failed to save settings", {
        description: error instanceof Error ? error.message : "Unknown error occurred",
      })
    }
  }

  const handleSaveThemeSettings = async () => {
    try {
      const settings = {
        theme: themeSettings.theme,
        accent_color: themeSettings.accentColor,
      }

      await saveThemeSettings(settings)

      toast.success("Theme settings saved", {
        description: "Your theme settings have been saved successfully.",
      })
    } catch (error) {
      toast.error("Failed to save settings", {
        description: error instanceof Error ? error.message : "Unknown error occurred",
      })
    }
  }

  const handleSaveDataSettings = async () => {
    try {
      const settings = {
        data_directory: dataSettings.dataDirectory,
        auto_backup: dataSettings.autoBackup,
        backup_frequency: dataSettings.backupFrequency,
      }

      await saveDataSettings(settings)

      toast.success("Data settings saved", {
        description: "Your data settings have been saved successfully.",
      })
    } catch (error) {
      toast.error("Failed to save settings", {
        description: error instanceof Error ? error.message : "Unknown error occurred",
      })
    }
  }

  const handleBackupNow = async () => {
    try {
      await backupData()
      toast.success("Backup completed", {
        description: "Your data has been backed up successfully.",
      })
    } catch (error) {
      toast.error("Backup failed", {
        description: error instanceof Error ? error.message : "Unknown error occurred",
      })
    }
  }

  const handleRestoreBackup = async () => {
    try {
      const backupPath = await showSaveDialog()
      if (backupPath) {
        await restoreData(backupPath)
        toast.success("Restore completed", {
          description: "Your data has been restored successfully.",
        })
      }
    } catch (error) {
      toast.error("Restore failed", {
        description: error instanceof Error ? error.message : "Unknown error occurred",
      })
    }
  }

  if (isLoading) {
    return (
      <div className="flex items-center justify-center h-full">
        <div className="animate-spin rounded-full h-12 w-12 border-b-2 border-primary"></div>
      </div>
    )
  }

  return (
    <div className="space-y-6">
      <h1 className="text-3xl font-bold tracking-tight">Settings</h1>

      <Tabs defaultValue="general">
        <TabsList className="grid grid-cols-4 mb-6">
          <TabsTrigger value="general">General</TabsTrigger>
          <TabsTrigger value="timer">Timer</TabsTrigger>
          <TabsTrigger value="theme">Theme</TabsTrigger>
          <TabsTrigger value="data">Data</TabsTrigger>
        </TabsList>

        <TabsContent value="general">
          <Card>
            <CardHeader>
              <CardTitle>General Settings</CardTitle>
              <CardDescription>Configure default application paths and startup behavior.</CardDescription>
            </CardHeader>
            <CardContent className="space-y-6">
              <div className="space-y-4">
                <h3 className="text-lg font-medium">Default Application Paths</h3>
                <div className="space-y-4">
                  <div className="grid gap-2">
                    <Label htmlFor="editor-path">Code Editor</Label>
                    <Input
                      id="editor-path"
                      value={defaultApplicationPaths.editor}
                      onChange={(e) =>
                        setDefaultApplicationPaths({
                          ...defaultApplicationPaths,
                          editor: e.target.value,
                        })
                      }
                      placeholder="/usr/bin/code"
                    />
                  </div>

                  <div className="grid gap-2">
                    <Label htmlFor="browser-path">Web Browser</Label>
                    <Input
                      id="browser-path"
                      value={defaultApplicationPaths.browser}
                      onChange={(e) =>
                        setDefaultApplicationPaths({
                          ...defaultApplicationPaths,
                          browser: e.target.value,
                        })
                      }
                      placeholder="/usr/bin/google-chrome"
                    />
                  </div>

                  <div className="grid gap-2">
                    <Label htmlFor="terminal-path">Terminal</Label>
                    <Input
                      id="terminal-path"
                      value={defaultApplicationPaths.terminal}
                      onChange={(e) =>
                        setDefaultApplicationPaths({
                          ...defaultApplicationPaths,
                          terminal: e.target.value,
                        })
                      }
                      placeholder="/usr/bin/gnome-terminal"
                    />
                  </div>
                </div>
              </div>

              <div className="space-y-4">
                <h3 className="text-lg font-medium">Startup Behavior</h3>
                <div className="space-y-4">
                  <div className="flex items-center justify-between">
                    <Label htmlFor="launch-startup">Launch on system startup</Label>
                    <Switch
                      id="launch-startup"
                      checked={startupSettings.launchOnStartup}
                      onCheckedChange={(checked) =>
                        setStartupSettings({
                          ...startupSettings,
                          launchOnStartup: checked,
                        })
                      }
                    />
                  </div>

                  <div className="flex items-center justify-between">
                    <Label htmlFor="minimize-tray">Minimize to system tray</Label>
                    <Switch
                      id="minimize-tray"
                      checked={startupSettings.minimizeToTray}
                      onCheckedChange={(checked) =>
                        setStartupSettings({
                          ...startupSettings,
                          minimizeToTray: checked,
                        })
                      }
                    />
                  </div>

                  <div className="flex items-center justify-between">
                    <Label htmlFor="reopen-project">Reopen last project on startup</Label>
                    <Switch
                      id="reopen-project"
                      checked={startupSettings.reopenLastProject}
                      onCheckedChange={(checked) =>
                        setStartupSettings({
                          ...startupSettings,
                          reopenLastProject: checked,
                        })
                      }
                    />
                  </div>
                </div>
              </div>
            </CardContent>
            <CardFooter>
              <Button onClick={handleSaveGeneralSettings}>Save Changes</Button>
            </CardFooter>
          </Card>
        </TabsContent>

        <TabsContent value="timer">
          <Card>
            <CardHeader>
              <CardTitle>Timer Settings</CardTitle>
              <CardDescription>Configure the Pomodoro timer and notification preferences.</CardDescription>
            </CardHeader>
            <CardContent className="space-y-6">
              <div className="space-y-4">
                <h3 className="text-lg font-medium">Timer Durations</h3>
                <div className="space-y-6">
                  <div className="space-y-2">
                    <div className="flex justify-between">
                      <Label htmlFor="pomodoro-length">Pomodoro Length: {timerSettings.pomodoroLength} minutes</Label>
                    </div>
                    <Slider
                      id="pomodoro-length"
                      min={5}
                      max={60}
                      step={5}
                      value={[timerSettings.pomodoroLength]}
                      onValueChange={(value) =>
                        setTimerSettings({
                          ...timerSettings,
                          pomodoroLength: value[0],
                        })
                      }
                    />
                  </div>

                  <div className="space-y-2">
                    <div className="flex justify-between">
                      <Label htmlFor="short-break-length">
                        Short Break Length: {timerSettings.shortBreakLength} minutes
                      </Label>
                    </div>
                    <Slider
                      id="short-break-length"
                      min={1}
                      max={15}
                      step={1}
                      value={[timerSettings.shortBreakLength]}
                      onValueChange={(value) =>
                        setTimerSettings({
                          ...timerSettings,
                          shortBreakLength: value[0],
                        })
                      }
                    />
                  </div>

                  <div className="space-y-2">
                    <div className="flex justify-between">
                      <Label htmlFor="long-break-length">
                        Long Break Length: {timerSettings.longBreakLength} minutes
                      </Label>
                    </div>
                    <Slider
                      id="long-break-length"
                      min={5}
                      max={30}
                      step={5}
                      value={[timerSettings.longBreakLength]}
                      onValueChange={(value) =>
                        setTimerSettings({
                          ...timerSettings,
                          longBreakLength: value[0],
                        })
                      }
                    />
                  </div>
                </div>
              </div>

              <div className="space-y-4">
                <h3 className="text-lg font-medium">Timer Behavior</h3>
                <div className="space-y-4">
                  <div className="flex items-center justify-between">
                    <Label htmlFor="auto-start-breaks">Auto-start breaks</Label>
                    <Switch
                      id="auto-start-breaks"
                      checked={timerSettings.autoStartBreaks}
                      onCheckedChange={(checked) =>
                        setTimerSettings({
                          ...timerSettings,
                          autoStartBreaks: checked,
                        })
                      }
                    />
                  </div>

                  <div className="flex items-center justify-between">
                    <Label htmlFor="auto-start-pomodoros">Auto-start pomodoros</Label>
                    <Switch
                      id="auto-start-pomodoros"
                      checked={timerSettings.autoStartPomodoros}
                      onCheckedChange={(checked) =>
                        setTimerSettings({
                          ...timerSettings,
                          autoStartPomodoros: checked,
                        })
                      }
                    />
                  </div>

                  <div className="flex items-center justify-between">
                    <Label htmlFor="show-notifications">Show notifications</Label>
                    <Switch
                      id="show-notifications"
                      checked={timerSettings.showNotifications}
                      onCheckedChange={(checked) =>
                        setTimerSettings({
                          ...timerSettings,
                          showNotifications: checked,
                        })
                      }
                    />
                  </div>

                  <div className="flex items-center justify-between">
                    <Label htmlFor="play-sound">Play sound when timer ends</Label>
                    <Switch
                      id="play-sound"
                      checked={timerSettings.playSound}
                      onCheckedChange={(checked) =>
                        setTimerSettings({
                          ...timerSettings,
                          playSound: checked,
                        })
                      }
                    />
                  </div>
                </div>
              </div>
            </CardContent>
            <CardFooter>
              <Button onClick={handleSaveTimerSettings}>Save Changes</Button>
            </CardFooter>
          </Card>
        </TabsContent>

        <TabsContent value="theme">
          <Card>
            <CardHeader>
              <CardTitle>Theme Settings</CardTitle>
              <CardDescription>Customize the appearance of the application.</CardDescription>
            </CardHeader>
            <CardContent className="space-y-6">
              <div className="space-y-4">
                <div className="grid gap-2">
                  <Label htmlFor="theme">Theme</Label>
                  <Select
                    value={themeSettings.theme}
                    onValueChange={(value) =>
                      setThemeSettings({
                        ...themeSettings,
                        theme: value,
                      })
                    }
                  >
                    <SelectTrigger id="theme">
                      <SelectValue placeholder="Select a theme" />
                    </SelectTrigger>
                    <SelectContent>
                      <SelectItem value="light">Light</SelectItem>
                      <SelectItem value="dark">Dark</SelectItem>
                      <SelectItem value="system">System</SelectItem>
                    </SelectContent>
                  </Select>
                </div>
              </div>
            </CardContent>
            <CardFooter>
              <Button onClick={handleSaveThemeSettings}>Save Changes</Button>
            </CardFooter>
          </Card>
        </TabsContent>

        <TabsContent value="data">
          <Card>
            <CardHeader>
              <CardTitle>Data Settings</CardTitle>
              <CardDescription>Configure data storage and backup options.</CardDescription>
            </CardHeader>
            <CardContent className="space-y-6">
              <div className="space-y-4">
                <div className="grid gap-2">
                  <Label htmlFor="data-directory">Data Directory</Label>
                  <Input
                    id="data-directory"
                    value={dataSettings.dataDirectory}
                    onChange={(e) =>
                      setDataSettings({
                        ...dataSettings,
                        dataDirectory: e.target.value,
                      })
                    }
                    placeholder="~/Music/QuickDev"
                  />
                  <p className="text-sm text-muted-foreground">
                    This is where your project data and settings are stored.
                  </p>
                </div>
              </div>

              <div className="space-y-4">
                <h3 className="text-lg font-medium">Backup Settings</h3>
                <div className="space-y-4">
                  <div className="flex items-center justify-between">
                    <Label htmlFor="auto-backup">Automatic backups</Label>
                    <Switch
                      id="auto-backup"
                      checked={dataSettings.autoBackup}
                      onCheckedChange={(checked) =>
                        setDataSettings({
                          ...dataSettings,
                          autoBackup: checked,
                        })
                      }
                    />
                  </div>

                  <div className="grid gap-2">
                    <Label htmlFor="backup-frequency">Backup Frequency</Label>
                    <Select
                      value={dataSettings.backupFrequency}
                      onValueChange={(value) =>
                        setDataSettings({
                          ...dataSettings,
                          backupFrequency: value,
                        })
                      }
                      disabled={!dataSettings.autoBackup}
                    >
                      <SelectTrigger id="backup-frequency">
                        <SelectValue placeholder="Select frequency" />
                      </SelectTrigger>
                      <SelectContent>
                        <SelectItem value="daily">Daily</SelectItem>
                        <SelectItem value="weekly">Weekly</SelectItem>
                        <SelectItem value="monthly">Monthly</SelectItem>
                      </SelectContent>
                    </Select>
                  </div>

                  <div className="flex flex-col gap-2 sm:flex-row">
                    <Button onClick={handleBackupNow}>Backup Now</Button>
                    <Button variant="outline" onClick={handleRestoreBackup}>
                      Restore from Backup
                    </Button>
                  </div>
                </div>
              </div>
            </CardContent>
            <CardFooter>
              <Button onClick={handleSaveDataSettings}>Save Changes</Button>
            </CardFooter>
          </Card>
        </TabsContent>
      </Tabs>
    </div>
  )
}
