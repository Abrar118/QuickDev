package cli

import (
	"bufio"
	"errors"
	"fmt"
	"os"
	"os/exec"
	"path/filepath"
	"runtime"
	"strings"

	"github.com/spf13/cobra"

	"github.com/Abrar118/QuickDev/internal/adapters"
	"github.com/Abrar118/QuickDev/internal/apps"
	"github.com/Abrar118/QuickDev/internal/config"
	"github.com/Abrar118/QuickDev/internal/doctor"
	"github.com/Abrar118/QuickDev/internal/fzf"
	"github.com/Abrar118/QuickDev/internal/launch"
	"github.com/Abrar118/QuickDev/internal/models"
	"github.com/Abrar118/QuickDev/internal/shellwords"
	"github.com/Abrar118/QuickDev/internal/ui"
	"github.com/Abrar118/QuickDev/internal/validate"
)

func cwd() (string, error) {
	dir, err := os.Getwd()
	if err != nil {
		return "", fmt.Errorf("cannot read current directory: %w", err)
	}
	return dir, nil
}

func projectNames(global *models.GlobalConfig) []string {
	names := make([]string, len(global.Projects))
	for i, p := range global.Projects {
		names[i] = p.Name
	}
	return names
}

func findProject(global *models.GlobalConfig, name, what string) (*models.GlobalProjectEntry, error) {
	for i := range global.Projects {
		if global.Projects[i].Name == name {
			return &global.Projects[i], nil
		}
	}
	suggestion, ok := ui.Closest(name, projectNames(global))
	return nil, errors.New(ui.WithSuggestion(fmt.Sprintf("%s '%s' not found in global index", what, name), suggestion, ok))
}

var stdin = bufio.NewReader(os.Stdin)

func prompt(message string) (string, error) {
	ui.Eprint(ui.Paint(ui.Cyan.With(ui.Bold), "?") + " " + message)
	line, err := stdin.ReadString('\n')
	if err != nil && line == "" {
		return "", fmt.Errorf("failed to read input: %w", err)
	}
	return strings.TrimSpace(line), nil
}

// buildItemDisplayList returns picker rows for every terminal then every
// application. A row's position identifies it (see selectedItems).
func buildItemDisplayList(cfg *models.ProjectConfig) []string {
	var items []string
	for _, t := range cfg.Terminals {
		cmd := ""
		if t.Command != nil {
			cmd = " (" + fzf.SanitizeRow(*t.Command) + ")"
		}
		items = append(items, fmt.Sprintf("[terminal] %s — %s%s", fzf.SanitizeRow(t.Name), fzf.SanitizeRow(t.Path), cmd))
	}
	for _, a := range cfg.Applications {
		items = append(items, fmt.Sprintf("[app] %s — %s", fzf.SanitizeRow(a.Name), fzf.SanitizeRow(a.Path)))
	}
	return items
}

// selectedItems splits picker positions into terminal and application sets.
func selectedItems(cfg *models.ProjectConfig, positions []int) (terminals, applications map[int]bool) {
	terminals, applications = map[int]bool{}, map[int]bool{}
	boundary := len(cfg.Terminals)
	for _, p := range positions {
		if p < boundary {
			terminals[p] = true
		} else {
			applications[p-boundary] = true
		}
	}
	return terminals, applications
}

// ---------------------------------------------------------------------------
// init

func initCmd() *cobra.Command {
	var from string
	cmd := &cobra.Command{
		Use:   "init",
		Short: "Create .quickdev.toml in the current directory and register the project",
		Args:  cobra.NoArgs,
		RunE: run(func(cmd *cobra.Command, args []string) error {
			dir, err := cwd()
			if err != nil {
				return err
			}
			configPath := filepath.Join(dir, config.ProjectFileName)
			dirName := filepath.Base(dir)
			if dirName == "/" || dirName == "." || dirName == string(filepath.Separator) {
				dirName = "project"
			}

			globalPath, err := config.GlobalConfigPath()
			if err != nil {
				return err
			}
			global, err := config.LoadGlobalConfig(globalPath)
			if err != nil {
				return err
			}
			alreadyIndexed := false
			for _, p := range global.Projects {
				if p.Path == dir {
					alreadyIndexed = true
				}
			}
			_, statErr := os.Stat(configPath)
			configExists := statErr == nil

			if configExists && alreadyIndexed {
				return fmt.Errorf(".quickdev.toml already exists and is registered in %s", dir)
			}
			if configExists {
				name, err := config.RegisterExistingProjectConfig(configPath, dir, global)
				if err != nil {
					return err
				}
				if err := config.SaveGlobalConfig(globalPath, global); err != nil {
					return err
				}
				ui.Println(ui.OK("Re-registered project " + ui.Paint(ui.Bold, name) + " in the global index"))
				return nil
			}

			name := config.UniqueProjectName(dirName, global)
			project := &models.ProjectConfig{Project: models.ProjectEntry{Name: name}}
			if from != "" {
				source, err := findProject(global, from, "source project")
				if err != nil {
					return err
				}
				src, err := config.LoadProjectConfig(filepath.Join(source.Path, config.ProjectFileName))
				if err != nil {
					return err
				}
				project.Terminals, project.Applications = src.Terminals, src.Applications
			}

			if err := config.SaveProjectConfig(configPath, project); err != nil {
				return err
			}
			global.Projects = append(global.Projects, models.GlobalProjectEntry{Name: name, Path: dir})
			if err := config.SaveGlobalConfig(globalPath, global); err != nil {
				// Roll back rather than leave an unregistered .quickdev.toml
				// that a retried `init` would refuse.
				os.Remove(configPath)
				return err
			}

			template := ""
			if from != "" {
				template = " from template"
			}
			ui.Println(ui.OK(fmt.Sprintf("Initialized project %s%s in %s", ui.Paint(ui.Bold, name), template, ui.Tilde(dir))))
			ui.Println("  " + ui.Paint(ui.Dim, "Global index: "+ui.Tilde(globalPath)))
			ui.Println("  " + ui.Paint(ui.Dim, "Next: 'quickdev add' to configure terminals and apps"))
			return nil
		}),
	}
	cmd.Flags().StringVar(&from, "from", "", "Clone config from another project by name")
	_ = cmd.RegisterFlagCompletionFunc("from", projectCandidates)
	return cmd
}

// ---------------------------------------------------------------------------
// launch

func launchCmd() *cobra.Command {
	var all, dryRun bool
	cmd := &cobra.Command{
		Use:               "launch [project]",
		Short:             "Launch terminals and applications for a project",
		Args:              cobra.MaximumNArgs(1),
		ValidArgsFunction: onlyFirstArg(projectCandidates),
		RunE: run(func(cmd *cobra.Command, args []string) error {
			globalPath, err := config.GlobalConfigPath()
			if err != nil {
				return err
			}
			global, err := config.LoadGlobalConfig(globalPath)
			if err != nil {
				return err
			}

			var root string
			var cfg *models.ProjectConfig
			if len(args) == 1 {
				entry, err := findProject(global, args[0], "project")
				if err != nil {
					return err
				}
				root = entry.Path
				if cfg, err = config.LoadProjectConfig(filepath.Join(root, config.ProjectFileName)); err != nil {
					return err
				}
			} else {
				dir, err := cwd()
				if err != nil {
					return err
				}
				var configPath string
				if configPath, root, err = config.ResolveProjectConfig(dir); err != nil {
					return err
				}
				if cfg, err = config.LoadProjectConfig(configPath); err != nil {
					return err
				}
			}

			if len(cfg.Terminals) == 0 && len(cfg.Applications) == 0 {
				return errors.New("no terminals or applications configured")
			}

			if !all {
				if items := buildItemDisplayList(cfg); len(items) > 1 {
					picked, err := fzf.SelectMultiIndexed(items, "Select items to launch (TAB to toggle, ENTER to confirm):")
					if err != nil {
						return err
					}
					terminals, applications := selectedItems(cfg, picked)
					filtered := &models.ProjectConfig{Project: cfg.Project}
					for i, t := range cfg.Terminals {
						if terminals[i] {
							filtered.Terminals = append(filtered.Terminals, t)
						}
					}
					for i, a := range cfg.Applications {
						if applications[i] {
							filtered.Applications = append(filtered.Applications, a)
						}
					}
					cfg = filtered
				}
			}

			if dryRun {
				plan := launch.PlanLaunch(cfg, root)
				would := 0
				for _, r := range plan {
					if r.Success {
						would++
					}
				}
				ui.Print(launch.RenderResults("Dry run — would launch "+ui.Count(would, "item"), plan))
				return nil
			}

			globalEmulator := models.Deref(global.Emulator)
			if shouldPromptForTerminalAppTabbing(cfg, globalEmulator) {
				outcome := launch.PromptToEnableTerminalTabbing(global.TerminalAppTabbingPromptDeclined)
				switch {
				case outcome.Declined:
					global.TerminalAppTabbingPromptDeclined = true
					if err := config.SaveGlobalConfig(globalPath, global); err != nil {
						return err
					}
				case outcome.WriteErr != nil:
					ui.Eprintln(ui.Warn(outcome.WriteErr.Error()))
				}
			}

			results := launch.LaunchProject(cfg, root, globalEmulator)
			success := 0
			for _, r := range results {
				if r.Success {
					success++
				}
			}
			ui.Print(launch.RenderResults(fmt.Sprintf("Launched %d/%s", success, ui.Count(len(results), "item")), results))
			if success == 0 {
				return silentExit(1)
			}
			// Best effort: a failed recency stamp must not fail the launch.
			if err := config.RecordLaunch(globalPath, root); err != nil {
				ui.Eprintln(ui.Warn("could not record launch time: " + err.Error()))
			}
			return nil
		}),
	}
	cmd.Flags().BoolVar(&all, "all", false, "Launch all items without interactive selection")
	cmd.Flags().BoolVar(&dryRun, "dry-run", false, "Print what would launch without starting anything")
	return cmd
}

func shouldPromptForTerminalAppTabbing(cfg *models.ProjectConfig, globalEmulator string) bool {
	if runtime.GOOS != "darwin" || len(cfg.Terminals) <= 1 || launch.ReadTabbingPreference() == launch.TabbingAlways {
		return false
	}
	for _, t := range cfg.Terminals {
		effective := models.Deref(t.Emulator)
		if effective == "" {
			effective = globalEmulator
		}
		// Only prompt when Terminal.app is what will actually open: auto-detect
		// prefers kitty and Ghostty when installed.
		if effective == "terminal" {
			return true
		}
		if effective == "" {
			_, kitty := adapters.ResolveKitty()
			if !kitty && !adapters.CommandExists("ghostty") {
				return true
			}
		}
	}
	return false
}

// ---------------------------------------------------------------------------
// add

func addCmd() *cobra.Command {
	cmd := &cobra.Command{
		Use:   "add",
		Short: "Add a terminal or application entry (interactive if no subcommand given)",
		Args:  cobra.NoArgs,
		RunE: run(func(cmd *cobra.Command, args []string) error {
			configPath, root, cfg, err := loadCurrentProject()
			if err != nil {
				return err
			}
			return addInteractive(configPath, root, cfg)
		}),
	}

	var command, emulator string
	terminal := &cobra.Command{
		Use:   "terminal <name> <path>",
		Short: "Add a terminal entry",
		Args:  cobra.ExactArgs(2),
		Long: `Add a terminal entry.

Examples:
  quickdev add terminal server .                        Open shell in project root
  quickdev add terminal dev . --command "npm run dev"   Run a command on open
  quickdev add terminal logs ./logs                     Open shell in subdirectory`,
		ValidArgsFunction: func(_ *cobra.Command, args []string, _ string) ([]string, cobra.ShellCompDirective) {
			if len(args) == 1 {
				return nil, cobra.ShellCompDirectiveFilterDirs
			}
			return nil, cobra.ShellCompDirectiveNoFileComp
		},
		RunE: run(func(cmd *cobra.Command, args []string) error {
			configPath, root, cfg, err := loadCurrentProject()
			if err != nil {
				return err
			}
			entry := models.TerminalEntry{Name: args[0], Path: args[1]}
			if cmd.Flags().Changed("command") {
				entry.Command = models.Str(command)
			}
			if cmd.Flags().Changed("emulator") {
				entry.Emulator = models.Str(emulator)
			}
			return addTerminal(configPath, root, cfg, entry)
		}),
	}
	terminal.Flags().StringVar(&command, "command", "", "Startup command to run in the terminal")
	terminal.Flags().StringVar(&emulator, "emulator", "",
		"Terminal emulator to use (ghostty, terminal, gnome-terminal, ptyxis, kitty). Omit for auto-detect.")
	_ = terminal.RegisterFlagCompletionFunc("emulator", fixed(config.SupportedEmulators...))

	var appArgs []string
	app := &cobra.Command{
		Use:   "app <name> <path>",
		Short: "Add an application entry",
		Args:  cobra.ExactArgs(2),
		Long: `Add an application entry.

Examples:
  quickdev add app Cursor /Applications/Cursor.app --args "."
  quickdev add app Firefox /usr/bin/firefox`,
		ValidArgsFunction: func(_ *cobra.Command, args []string, _ string) ([]string, cobra.ShellCompDirective) {
			if len(args) == 1 {
				return nil, cobra.ShellCompDirectiveDefault
			}
			return nil, cobra.ShellCompDirectiveNoFileComp
		},
		RunE: run(func(cmd *cobra.Command, args []string) error {
			configPath, _, cfg, err := loadCurrentProject()
			if err != nil {
				return err
			}
			return addApp(configPath, cfg, models.AppEntry{Name: args[0], Path: args[1], Args: appArgs})
		}),
	}
	app.Flags().StringArrayVar(&appArgs, "args", nil, "Arguments passed to the application")

	cmd.AddCommand(terminal, app)
	return cmd
}

func loadCurrentProject() (string, string, *models.ProjectConfig, error) {
	dir, err := cwd()
	if err != nil {
		return "", "", nil, err
	}
	configPath, root, err := config.ResolveProjectConfig(dir)
	if err != nil {
		return "", "", nil, err
	}
	cfg, err := config.LoadProjectConfig(configPath)
	if err != nil {
		return "", "", nil, err
	}
	return configPath, root, cfg, nil
}

func addTerminal(configPath, root string, cfg *models.ProjectConfig, entry models.TerminalEntry) error {
	for _, t := range cfg.Terminals {
		if t.Name == entry.Name {
			return fmt.Errorf("terminal '%s' already exists", entry.Name)
		}
	}
	if err := validate.TerminalEntry(entry, root); err != nil {
		return err
	}
	cfg.Terminals = append(cfg.Terminals, entry)
	// Announce only after the write succeeds, so a failed save is never
	// preceded by "Added …".
	if err := config.SaveProjectConfig(configPath, cfg); err != nil {
		return err
	}
	ui.Println(ui.OK("Added terminal " + ui.Paint(ui.Bold, entry.Name)))
	return nil
}

func addApp(configPath string, cfg *models.ProjectConfig, entry models.AppEntry) error {
	for _, a := range cfg.Applications {
		if a.Name == entry.Name {
			return fmt.Errorf("application '%s' already exists", entry.Name)
		}
	}
	if err := validate.AppEntry(entry); err != nil {
		return err
	}
	cfg.Applications = append(cfg.Applications, entry)
	if err := config.SaveProjectConfig(configPath, cfg); err != nil {
		return err
	}
	ui.Println(ui.OK("Added application " + ui.Paint(ui.Bold, entry.Name)))
	return nil
}

func addInteractive(configPath, root string, cfg *models.ProjectConfig) error {
	selected, err := fzf.SelectOne([]string{"Terminal", "Application"}, "Select what to add:")
	if err != nil {
		return err
	}
	switch selected {
	case "Terminal":
		path, err := prompt("Path (. for current directory): ")
		if err != nil {
			return err
		}
		if path == "" {
			path = "."
		}
		name, err := prompt("Name for this tab: ")
		if err != nil {
			return err
		}
		for _, t := range cfg.Terminals {
			if t.Name == name {
				return fmt.Errorf("terminal '%s' already exists", name)
			}
		}
		command, err := prompt("Startup command (optional, press Enter to skip): ")
		if err != nil {
			return err
		}
		emulator, err := pickEmulator()
		if err != nil {
			return err
		}
		entry := models.TerminalEntry{Name: name, Path: path, Emulator: emulator}
		if command != "" {
			entry.Command = models.Str(command)
		}
		return addTerminal(configPath, root, cfg, entry)
	case "Application":
		app, err := pickApplication()
		if err != nil {
			return err
		}
		for _, a := range cfg.Applications {
			if a.Name == app.Name {
				return fmt.Errorf("application '%s' already exists", app.Name)
			}
		}
		input, err := prompt(`Arguments (e.g., "." to open project root, Enter to skip): `)
		if err != nil {
			return err
		}
		var userArgs []string
		if input != "" {
			if userArgs, err = shellwords.ParseArgs(input); err != nil {
				return err
			}
		}
		app.Args = apps.CombineAppArgs(app.Args, userArgs)
		return addApp(configPath, cfg, app)
	}
	return errors.New("invalid selection")
}

func pickEmulator() (*string, error) {
	type option struct {
		name      string
		installed bool
	}
	var installed, missing []option
	for _, e := range config.SupportedEmulators {
		if adapters.EmulatorInstalled(e) {
			installed = append(installed, option{e, true})
		} else {
			missing = append(missing, option{e, false})
		}
	}
	// Installed ones first, canonical order within each group.
	options := append(installed, missing...)
	rows := []string{"Auto-detect (default)"}
	for _, o := range options {
		if o.installed {
			rows = append(rows, o.name+" (installed)")
		} else {
			rows = append(rows, o.name)
		}
	}
	index, err := fzf.SelectOneIndexed(rows, "Select terminal emulator:")
	if err != nil {
		return nil, err
	}
	if index == 0 {
		return nil, nil
	}
	return models.Str(options[index-1].name), nil
}

func pickApplication() (models.AppEntry, error) {
	discovered := apps.DiscoverApps()
	if len(discovered) == 0 {
		return manualAppEntry()
	}
	items := []string{"[Enter path manually]"}
	for _, a := range discovered {
		items = append(items, fmt.Sprintf("%s  (%s)", a.Name, a.Path))
	}
	// Indexed picker: names and paths come from the filesystem, so matching
	// display text back to entries would be fragile.
	index, err := fzf.SelectOneIndexed(items, "Select an application:")
	if err != nil {
		return models.AppEntry{}, err
	}
	if index == 0 {
		return manualAppEntry()
	}
	return discovered[index-1], nil
}

func manualAppEntry() (models.AppEntry, error) {
	path, err := prompt("Application path: ")
	if err != nil {
		return models.AppEntry{}, err
	}
	if path == "" {
		return models.AppEntry{}, errors.New("path cannot be empty")
	}
	name, err := prompt("Application name: ")
	if err != nil {
		return models.AppEntry{}, err
	}
	if name == "" {
		return models.AppEntry{}, errors.New("name cannot be empty")
	}
	return models.AppEntry{Name: name, Path: path}, nil
}

// ---------------------------------------------------------------------------
// remove

func removeCmd() *cobra.Command {
	cmd := &cobra.Command{
		Use:   "remove",
		Short: "Remove terminals/apps (interactive picker, or specify: remove terminal <name>)",
		Args:  cobra.NoArgs,
		RunE: run(func(cmd *cobra.Command, args []string) error {
			configPath, _, cfg, err := loadCurrentProject()
			if err != nil {
				return err
			}
			return removeInteractive(configPath, cfg)
		}),
	}
	terminal := &cobra.Command{
		Use:               "terminal <name>",
		Short:             "Remove a terminal entry by name",
		Args:              cobra.ExactArgs(1),
		ValidArgsFunction: currentProjectItems(false),
		RunE: run(func(cmd *cobra.Command, args []string) error {
			configPath, _, cfg, err := loadCurrentProject()
			if err != nil {
				return err
			}
			name := args[0]
			var kept []models.TerminalEntry
			var names []string
			for _, t := range cfg.Terminals {
				names = append(names, t.Name)
				if t.Name != name {
					kept = append(kept, t)
				}
			}
			if len(kept) == len(cfg.Terminals) {
				s, ok := ui.Closest(name, names)
				return errors.New(ui.WithSuggestion(fmt.Sprintf("terminal '%s' not found", name), s, ok))
			}
			cfg.Terminals = kept
			if err := config.SaveProjectConfig(configPath, cfg); err != nil {
				return err
			}
			ui.Println(ui.OK("Removed terminal " + ui.Paint(ui.Bold, name)))
			return nil
		}),
	}
	app := &cobra.Command{
		Use:               "app <name>",
		Short:             "Remove an application entry by name",
		Args:              cobra.ExactArgs(1),
		ValidArgsFunction: currentProjectItems(true),
		RunE: run(func(cmd *cobra.Command, args []string) error {
			configPath, _, cfg, err := loadCurrentProject()
			if err != nil {
				return err
			}
			name := args[0]
			var kept []models.AppEntry
			var names []string
			for _, a := range cfg.Applications {
				names = append(names, a.Name)
				if a.Name != name {
					kept = append(kept, a)
				}
			}
			if len(kept) == len(cfg.Applications) {
				s, ok := ui.Closest(name, names)
				return errors.New(ui.WithSuggestion(fmt.Sprintf("application '%s' not found", name), s, ok))
			}
			cfg.Applications = kept
			if err := config.SaveProjectConfig(configPath, cfg); err != nil {
				return err
			}
			ui.Println(ui.OK("Removed application " + ui.Paint(ui.Bold, name)))
			return nil
		}),
	}
	cmd.AddCommand(terminal, app)
	return cmd
}

func removeInteractive(configPath string, cfg *models.ProjectConfig) error {
	items := buildItemDisplayList(cfg)
	if len(items) == 0 {
		return errors.New("no terminals or applications configured")
	}
	picked, err := fzf.SelectMultiIndexed(items, "Select items to remove (TAB to toggle, ENTER to confirm):")
	if err != nil {
		return err
	}
	terminals, applications := selectedItems(cfg, picked)

	var removedTerminals, removedApps []string
	var keptTerminals []models.TerminalEntry
	for i, t := range cfg.Terminals {
		if terminals[i] {
			removedTerminals = append(removedTerminals, t.Name)
		} else {
			keptTerminals = append(keptTerminals, t)
		}
	}
	var keptApps []models.AppEntry
	for i, a := range cfg.Applications {
		if applications[i] {
			removedApps = append(removedApps, a.Name)
		} else {
			keptApps = append(keptApps, a)
		}
	}
	cfg.Terminals, cfg.Applications = keptTerminals, keptApps
	if err := config.SaveProjectConfig(configPath, cfg); err != nil {
		return err
	}
	for _, n := range removedTerminals {
		ui.Println(ui.OK("Removed terminal " + ui.Paint(ui.Bold, n)))
	}
	for _, n := range removedApps {
		ui.Println(ui.OK("Removed application " + ui.Paint(ui.Bold, n)))
	}
	return nil
}

// ---------------------------------------------------------------------------
// edit

func editCmd() *cobra.Command {
	var global bool
	cmd := &cobra.Command{
		Use:   "edit",
		Short: "Open .quickdev.toml in $EDITOR (or --global for global config)",
		Args:  cobra.NoArgs,
		RunE: run(func(cmd *cobra.Command, args []string) error {
			var path string
			var err error
			if global {
				path, err = config.GlobalConfigPath()
			} else {
				var dir string
				if dir, err = cwd(); err == nil {
					path, _, err = config.ResolveProjectConfig(dir)
				}
			}
			if err != nil {
				return err
			}

			editor, ok := os.LookupEnv("VISUAL")
			if !ok {
				if editor, ok = os.LookupEnv("EDITOR"); !ok {
					editor = "vi"
				}
			}
			parts, err := shellwords.ParseArgs(editor)
			if err != nil {
				return err
			}
			if len(parts) == 0 {
				return errors.New("editor command is empty")
			}
			c := exec.Command(parts[0], append(parts[1:], path)...)
			c.Stdin, c.Stdout, c.Stderr = os.Stdin, os.Stdout, os.Stderr
			if err := c.Start(); err != nil {
				return fmt.Errorf("failed to open editor '%s': %v", editor, err)
			}
			if err := c.Wait(); err != nil {
				return fmt.Errorf("editor '%s' exited with %v", editor, err)
			}
			return nil
		}),
	}
	cmd.Flags().BoolVar(&global, "global", false, "Edit global config instead of project config")
	return cmd
}

// ---------------------------------------------------------------------------
// deregister

func deregisterCmd() *cobra.Command {
	var del bool
	cmd := &cobra.Command{
		Use:   "deregister",
		Short: "Remove current project from global index",
		Args:  cobra.NoArgs,
		RunE: run(func(cmd *cobra.Command, args []string) error {
			dir, err := cwd()
			if err != nil {
				return err
			}
			configPath, root, err := config.ResolveProjectConfig(dir)
			if err != nil {
				return err
			}
			globalPath, err := config.GlobalConfigPath()
			if err != nil {
				return err
			}
			global, err := config.LoadGlobalConfig(globalPath)
			if err != nil {
				return err
			}
			removedName := ""
			var kept []models.GlobalProjectEntry
			for _, p := range global.Projects {
				if p.Path == root {
					if removedName == "" {
						removedName = p.Name
					}
					continue
				}
				kept = append(kept, p)
			}
			if len(kept) == len(global.Projects) {
				return errors.New("project not found in global index")
			}
			global.Projects = kept

			if del {
				// Neither order is safe alone: RemoveConfigWith stages the config
				// aside and restores it if the index write fails.
				_, err := config.RemoveConfigWith(configPath, func() (struct{}, error) {
					return struct{}{}, config.SaveGlobalConfig(globalPath, global)
				})
				if err != nil {
					return err
				}
				ui.Println(ui.OK("Deregistered " + ui.Paint(ui.Bold, removedName) + " and deleted its .quickdev.toml"))
				return nil
			}
			if err := config.SaveGlobalConfig(globalPath, global); err != nil {
				return err
			}
			ui.Println(ui.OK("Deregistered project " + ui.Paint(ui.Bold, removedName)))
			return nil
		}),
	}
	cmd.Flags().BoolVar(&del, "delete", false, "Also delete the .quickdev.toml file")
	return cmd
}

// ---------------------------------------------------------------------------
// config

func configCmd() *cobra.Command {
	cmd := &cobra.Command{
		Use:   "config",
		Short: "Get or set global settings (currently: emulator)",
		Args:  cobra.NoArgs,
		RunE: func(cmd *cobra.Command, args []string) error {
			return errors.New("'quickdev config' requires a subcommand: get, set, or unset")
		},
	}
	settings := fixed("emulator")
	withGlobal := func(fn func(path string, global *models.GlobalConfig, args []string) error) func(*cobra.Command, []string) error {
		return run(func(_ *cobra.Command, args []string) error {
			path, err := config.GlobalConfigPath()
			if err != nil {
				return err
			}
			global, err := config.LoadGlobalConfig(path)
			if err != nil {
				return err
			}
			return fn(path, global, args)
		})
	}

	set := &cobra.Command{
		Use:   "set <key> <value>",
		Short: "Set a global setting, e.g. `config set emulator ghostty`",
		Args:  cobra.ExactArgs(2),
		ValidArgsFunction: func(cmd *cobra.Command, args []string, s string) ([]string, cobra.ShellCompDirective) {
			if len(args) == 0 {
				return settings(cmd, args, s)
			}
			if len(args) == 1 {
				return config.SupportedEmulators, cobra.ShellCompDirectiveNoFileComp
			}
			return nil, cobra.ShellCompDirectiveNoFileComp
		},
		RunE: withGlobal(func(path string, global *models.GlobalConfig, args []string) error {
			msg, err := config.SetGlobalSetting(global, args[0], args[1])
			if err != nil {
				return err
			}
			if err := config.SaveGlobalConfig(path, global); err != nil {
				return err
			}
			ui.Println(ui.OK(msg))
			return nil
		}),
	}

	get := &cobra.Command{
		Use:               "get <key>",
		Short:             "Print a global setting, e.g. `config get emulator`",
		Args:              cobra.ExactArgs(1),
		ValidArgsFunction: onlyFirstArg(settings),
		RunE: withGlobal(func(_ string, global *models.GlobalConfig, args []string) error {
			msg, err := config.GetGlobalSetting(global, args[0])
			if err != nil {
				return err
			}
			ui.Println(msg)
			return nil
		}),
	}

	unset := &cobra.Command{
		Use:               "unset <key>",
		Short:             "Clear a global setting, e.g. `config unset emulator`",
		Args:              cobra.ExactArgs(1),
		ValidArgsFunction: onlyFirstArg(settings),
		RunE: withGlobal(func(path string, global *models.GlobalConfig, args []string) error {
			msg, err := config.UnsetGlobalSetting(global, args[0])
			if err != nil {
				return err
			}
			if err := config.SaveGlobalConfig(path, global); err != nil {
				return err
			}
			ui.Println(ui.OK(msg))
			return nil
		}),
	}
	cmd.AddCommand(set, get, unset)
	return cmd
}

// ---------------------------------------------------------------------------
// prune

func pruneCmd() *cobra.Command {
	return &cobra.Command{
		Use:   "prune",
		Short: "Remove registrations whose path or .quickdev.toml no longer exists",
		Args:  cobra.NoArgs,
		RunE: run(func(cmd *cobra.Command, args []string) error {
			path, err := config.GlobalConfigPath()
			if err != nil {
				return err
			}
			global, err := config.LoadGlobalConfig(path)
			if err != nil {
				return err
			}
			total := len(global.Projects)
			removed := config.PruneProjects(global)
			if len(removed) == 0 {
				ui.Println(ui.OK("Nothing to prune — all " + ui.Count(total, "registered project") + " healthy"))
				return nil
			}
			if err := config.SaveGlobalConfig(path, global); err != nil {
				return err
			}
			ui.Println(ui.OK("Pruned " + ui.Count(len(removed), "project")))
			for _, name := range removed {
				ui.Println("  " + ui.Paint(ui.Dim, "-") + " " + name)
			}
			return nil
		}),
	}
}

// ---------------------------------------------------------------------------
// validate

func validateCmd() *cobra.Command {
	var asJSON bool
	cmd := &cobra.Command{
		Use:   "validate",
		Short: "Check the current project's .quickdev.toml for problems",
		Args:  cobra.NoArgs,
		RunE: run(func(cmd *cobra.Command, args []string) error {
			dir, err := os.Getwd()
			if err != nil {
				return fmt.Errorf("could not determine current directory: %w", err)
			}
			// JSON output is for scripts and CI: never open the picker, and
			// report a missing config in the JSON itself.
			var configPath, root string
			if asJSON {
				configPath, root, err = config.FindProjectConfig(dir)
			} else {
				configPath, root, err = config.ResolveProjectConfig(dir)
			}
			if err != nil {
				if asJSON && !fzf.IsCancellation(err) {
					ui.Println(validate.Report{Errors: []string{err.Error()}}.JSON(nil))
				}
				return err
			}

			var report validate.Report
			if cfg, err := config.LoadProjectConfig(configPath); err != nil {
				report = validate.Report{Errors: []string{err.Error()}}
			} else {
				report = validate.ProjectConfig(cfg, root)
			}
			var result error
			if !report.OK() {
				result = fmt.Errorf("%s has %d error(s)", configPath, len(report.Errors))
			}

			if asJSON {
				ui.Println(report.JSON(&configPath))
				return result
			}
			for _, e := range report.Errors {
				ui.Println(ui.Fail(e))
			}
			for _, w := range report.Warnings {
				ui.Println(ui.Warn(w))
			}
			if report.OK() {
				path := ui.Tilde(configPath)
				if len(report.Warnings) == 0 {
					ui.Println(ui.OK(path + " is valid"))
				} else {
					ui.Println(ui.OK(path + " is valid " +
						ui.Paint(ui.Yellow, "("+ui.Count(len(report.Warnings), "warning")+")")))
				}
			}
			return result
		}),
	}
	cmd.Flags().BoolVar(&asJSON, "json", false, "Output the result as JSON (exit status still reports validity)")
	return cmd
}

// ---------------------------------------------------------------------------
// doctor

func doctorCmd() *cobra.Command {
	var fix, asJSON bool
	cmd := &cobra.Command{
		Use:   "doctor",
		Short: "Diagnose global config and registered projects (--fix to repair)",
		Args:  cobra.NoArgs,
		RunE: run(func(cmd *cobra.Command, args []string) error {
			if fix {
				if err := doctorFix(); err != nil {
					return err
				}
			}
			globalPath, err := config.GlobalConfigPath()
			if err != nil {
				return err
			}
			report := doctor.Report{FzfAvailable: fzf.CheckFzf()}
			if global, err := config.LoadGlobalConfig(globalPath); err == nil {
				report.GlobalConfigOK = true
				report.Projects = config.ProjectStatuses(global)
			}
			if asJSON {
				ui.Println(report.JSON())
			} else {
				ui.Print(report.Render())
			}
			if report.HasErrors() {
				return errors.New("doctor found problems (run 'quickdev doctor --fix' to repair)")
			}
			return nil
		}),
	}
	cmd.Flags().BoolVar(&fix, "fix", false, "Create missing config, prune dead registrations, normalize configs")
	cmd.Flags().BoolVar(&asJSON, "json", false, "Output the report as JSON (exit status still reports health)")
	cmd.MarkFlagsMutuallyExclusive("fix", "json")
	return cmd
}

func doctorFix() error {
	globalPath, err := config.GlobalConfigPath()
	if err != nil {
		return err
	}
	// 1. Create the global config if missing.
	if _, err := os.Stat(globalPath); err != nil {
		global, err := config.LoadGlobalConfig(globalPath)
		if err != nil {
			return err
		}
		if err := config.SaveGlobalConfig(globalPath, global); err != nil {
			return err
		}
		ui.Println(ui.OK("created global config at " + globalPath))
	}
	// 2. Prune dead registrations.
	global, err := config.LoadGlobalConfig(globalPath)
	if err != nil {
		return err
	}
	if removed := config.PruneProjects(global); len(removed) > 0 {
		if err := config.SaveGlobalConfig(globalPath, global); err != nil {
			return err
		}
		ui.Println(ui.OK(fmt.Sprintf("pruned %s: %s", ui.Count(len(removed), "dead registration"), strings.Join(removed, ", "))))
	}
	// 3. Normalize each remaining project config.
	for _, entry := range global.Projects {
		path := filepath.Join(entry.Path, config.ProjectFileName)
		cfg, err := config.LoadProjectConfig(path)
		if err != nil {
			ui.Println(ui.Warn(fmt.Sprintf("skipped %s (parse error): %v", path, err)))
			continue
		}
		if err := config.SaveProjectConfig(path, cfg); err != nil {
			ui.Println(ui.Warn(fmt.Sprintf("could not normalize %s: %v", path, err)))
			continue
		}
		ui.Println(ui.OK("normalized " + path))
	}
	return nil
}

// ---------------------------------------------------------------------------
// capture

func captureCmd() *cobra.Command {
	var all bool
	cmd := &cobra.Command{
		Use:   "capture",
		Short: "Capture currently-running apps into this project's .quickdev.toml",
		Args:  cobra.NoArgs,
		RunE: run(func(cmd *cobra.Command, args []string) error {
			if runtime.GOOS != "darwin" {
				return errors.New("capture is only supported on macOS")
			}
			dir, err := os.Getwd()
			if err != nil {
				return fmt.Errorf("failed to read current dir: %w", err)
			}
			configPath, _, err := config.ResolveProjectConfig(dir)
			if err != nil {
				return err
			}
			running, err := apps.DetectRunningApps()
			if err != nil {
				return err
			}
			// Path-unique: capture matches by path, so it must see a same-named
			// app in both /Applications and ~/Applications.
			candidates := apps.DetectedToApps(running, apps.DiscoverAppsUniqueByPath())
			project, err := config.LoadProjectConfig(configPath)
			if err != nil {
				return err
			}
			fresh := apps.MergeApps(project.Applications, candidates)
			if len(fresh) == 0 {
				ui.Println(ui.Paint(ui.Dim, "Nothing new to capture (no running apps, or all already configured)."))
				return nil
			}
			selected := fresh
			if !all {
				if !fzf.CheckFzf() {
					return errors.New("fzf is not installed. Install it, or re-run with --all. https://github.com/junegunn/fzf")
				}
				items := make([]string, len(fresh))
				for i, a := range fresh {
					items[i] = a.Name + " — " + a.Path
				}
				picked, err := fzf.SelectMultiIndexed(items, "Select apps to capture (TAB to toggle, ENTER to confirm):")
				if err != nil {
					return err
				}
				selected = nil
				for _, i := range picked {
					selected = append(selected, fresh[i])
				}
			}
			project.Applications = append(project.Applications, selected...)
			if err := config.SaveProjectConfig(configPath, project); err != nil {
				return err
			}
			ui.Println(ui.OK("Added " + ui.Count(len(selected), "app") + " to .quickdev.toml"))
			return nil
		}),
	}
	cmd.Flags().BoolVar(&all, "all", false, "Add all detected apps without interactive selection")
	return cmd
}
