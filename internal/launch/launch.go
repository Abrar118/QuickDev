// Package launch opens a project's terminals and applications.
package launch

import (
	"bufio"
	"errors"
	"fmt"
	"os"
	"os/exec"
	"path/filepath"
	"runtime"
	"strings"
	"time"

	"golang.org/x/term"

	"github.com/Abrar118/QuickDev/internal/adapters"
	"github.com/Abrar118/QuickDev/internal/config"
	"github.com/Abrar118/QuickDev/internal/models"
	"github.com/Abrar118/QuickDev/internal/ui"
)

// Result is the outcome of launching (or planning) one item.
type Result struct {
	Label   string
	Kind    string // "terminal" or "app"
	Success bool
	Error   string
	Detail  string
}

// TabLaunchFailure says why a grouped (tabbed) launch failed and whether
// retrying it one window at a time is safe.
//
// A batch launch is not atomic: AppleScript and gnome-terminal --load-config
// report status only after the whole batch ran, so an error can arrive after
// tabs opened and their startup commands were submitted. Re-launching those
// would run every command a second time, so only failures that provably
// happened before the emulator acted may fall back.
type TabLaunchFailure struct {
	Message string
	// Started means the emulator ran and then reported an error: an unknown
	// number of tabs may already be open with their commands running.
	Started bool
}

func notStarted(msg string) *TabLaunchFailure { return &TabLaunchFailure{Message: msg} }

func possiblyStarted(msg string) *TabLaunchFailure {
	return &TabLaunchFailure{Message: msg, Started: true}
}

// MayRetry reports whether the caller may launch the terminals individually.
func (f *TabLaunchFailure) MayRetry() bool { return !f.Started }

// RenderResults renders a header plus one ✓/✗ line per item.
func RenderResults(header string, results []Result) string {
	kindWidth, labelWidth := 0, 0
	for _, r := range results {
		kindWidth = max(kindWidth, len(r.Kind))
		labelWidth = max(labelWidth, ui.Width(r.Label))
	}
	var b strings.Builder
	fmt.Fprintf(&b, "%s\n\n", ui.Paint(ui.Bold, header))
	for _, r := range results {
		kind := ui.Pad(ui.Dim, r.Kind, kindWidth)
		tail := ""
		if r.Success {
			if r.Detail != "" {
				tail = ui.Paint(ui.Dim, r.Detail)
			}
		} else {
			msg := r.Error
			if msg == "" {
				msg = "unknown error"
			}
			tail = ui.Paint(ui.Red, msg)
		}
		row := kind + "  " + ui.Paint(ui.Bold, r.Label)
		if tail != "" {
			row = kind + "  " + ui.Pad(ui.Bold, r.Label, labelWidth) + "  " + tail
		}
		if r.Success {
			row = ui.OK(row)
		} else {
			row = ui.Fail(row)
		}
		fmt.Fprintf(&b, "  %s\n", row)
	}
	return b.String()
}

// EmulatorWatchProcess is the long-running process to watch as a readiness
// proxy for the emulator terminal #0 will cold-start, or "" when there is none
// (the caller then treats the emulator as already warm).
func EmulatorWatchProcess(emulator string, ghosttyAvailable, ptyxisAvailable, kittyAvailable bool, goos string) string {
	switch emulator {
	case "ghostty":
		return "ghostty"
	case "terminal":
		return nativeTerminalProcess(goos, ptyxisAvailable)
	case "gnome-terminal":
		if goos == "linux" {
			return "gnome-terminal-server"
		}
		return ""
	case "ptyxis":
		if goos == "linux" {
			return "ptyxis"
		}
		return ""
	case "":
		// Mirror the auto-selection in launchTerminal. kitty is not
		// single-instance, so there is nothing to wait on when it is used.
		if kittyAvailable && (goos == "darwin" || goos == "linux") {
			return ""
		}
		if goos == "darwin" && ghosttyAvailable {
			return "ghostty"
		}
		return nativeTerminalProcess(goos, ptyxisAvailable)
	}
	// kitty (not single-instance) and unknown emulators.
	return ""
}

func nativeTerminalProcess(goos string, ptyxisAvailable bool) string {
	switch goos {
	case "darwin":
		return "Terminal"
	case "linux":
		if ptyxisAvailable {
			return "ptyxis"
		}
		return "gnome-terminal-server"
	case "windows":
		return "WindowsTerminal.exe"
	}
	return ""
}

// PollUntil checks ready up to attempts times, sleeping interval between
// checks, and reports whether it was ever satisfied.
func PollUntil(ready func() bool, attempts int, interval time.Duration) bool {
	for i := 0; i < attempts; i++ {
		if ready() {
			return true
		}
		time.Sleep(interval)
	}
	return false
}

// PgrepArgsForProcess are the pgrep arguments that find name.
func PgrepArgsForProcess(name string) []string {
	if name == "gnome-terminal-server" {
		return []string{"-f", "gnome-terminal-server"}
	}
	return []string{"-x", name}
}

func processRunning(name string) bool {
	if runtime.GOOS == "windows" {
		out, err := exec.Command("tasklist", "/FI", "IMAGENAME eq "+name).Output()
		return err == nil && strings.Contains(strings.ToLower(string(out)), strings.ToLower(name))
	}
	return exec.Command("pgrep", PgrepArgsForProcess(name)...).Run() == nil
}

// waitForEmulatorReady waits, after a cold start, until the emulator's process
// is up and has had a moment to settle, so later terminals don't race it.
func waitForEmulatorReady(name string) {
	if PollUntil(func() bool { return processRunning(name) }, 30, 100*time.Millisecond) {
		time.Sleep(500 * time.Millisecond)
	}
}

func terminalDetail(resolvedPath, command string) string {
	if command != "" {
		return resolvedPath + " · " + command
	}
	return resolvedPath
}

// MakePlaceholderContext gathers the values for {root} {config} {name} {cwd}.
func MakePlaceholderContext(cfg *models.ProjectConfig, projectRoot string) adapters.PlaceholderContext {
	cwd, err := os.Getwd()
	if err != nil {
		cwd = projectRoot
	}
	return adapters.PlaceholderContext{
		Root:   projectRoot,
		Config: filepath.Join(projectRoot, config.ProjectFileName),
		Name:   cfg.Project.Name,
		Cwd:    cwd,
	}
}

func appDetail(path string, args []string) string {
	if len(args) > 0 {
		return path + " · args: " + strings.Join(args, " ")
	}
	return path
}

func effectiveArgs(app models.AppEntry, ctx adapters.PlaceholderContext, root string) []string {
	var resolved []string
	if app.Args != nil {
		resolved = adapters.ResolveAppArgs(app.Args, ctx)
	}
	return adapters.EffectiveAppArgs(app.Name, app.Path, resolved, root)
}

// PlanLaunch resolves what LaunchProject would launch, without spawning
// anything, applying the same checks so a dry run reports real failures.
func PlanLaunch(cfg *models.ProjectConfig, projectRoot string) []Result {
	var results []Result
	for _, t := range cfg.Terminals {
		resolved, err := adapters.ResolveExistingTerminalPath(projectRoot, t.Path)
		if err != nil {
			results = append(results, Result{Label: t.Name, Kind: "terminal", Error: err.Error()})
			continue
		}
		results = append(results, Result{Label: t.Name, Kind: "terminal", Success: true,
			Detail: terminalDetail(resolved, models.Deref(t.Command))})
	}
	ctx := MakePlaceholderContext(cfg, projectRoot)
	for _, app := range cfg.Applications {
		args := effectiveArgs(app, ctx, projectRoot)
		ok := adapters.AppTargetResolvable(app.Path)
		r := Result{Label: app.Name, Kind: "app", Success: ok, Detail: appDetail(app.Path, args)}
		if !ok {
			r.Error = "path does not exist and is not on PATH: " + app.Path
		}
		results = append(results, r)
	}
	return results
}

type preparedTerminal struct {
	index    int
	name     string
	path     string
	command  string
	emulator string
}

func orDefault(value *string, fallback string) string {
	if value != nil {
		return *value
	}
	return fallback
}

// LaunchProject opens every terminal and application in cfg.
func LaunchProject(cfg *models.ProjectConfig, projectRoot, globalEmulator string) []Result {
	// Single-instance emulators (Ghostty, Terminal.app, gnome-terminal, Windows
	// Terminal) are cold-started by terminal #0; later terminals that fire
	// before it is ready open bare shells. So when it wasn't already running,
	// wait for it after terminal #0. Warm runs skip the wait.
	firstEmulator := globalEmulator
	if len(cfg.Terminals) > 0 {
		firstEmulator = orDefault(cfg.Terminals[0].Emulator, globalEmulator)
	}
	ghosttyAvailable := adapters.CommandExists("ghostty")
	ptyxisAvailable := adapters.CommandExists("ptyxis")
	_, kittyAvailable := adapters.ResolveKitty()
	watch := EmulatorWatchProcess(firstEmulator, ghosttyAvailable, ptyxisAvailable, kittyAvailable, runtime.GOOS)
	emulatorWasRunning := watch == "" || processRunning(watch)
	multiple := len(cfg.Terminals) > 1

	terminalResults := make([]*Result, len(cfg.Terminals))
	var prepared []preparedTerminal
	for i, t := range cfg.Terminals {
		resolved, err := adapters.ResolveExistingTerminalPath(projectRoot, t.Path)
		if err != nil {
			terminalResults[i] = &Result{Label: t.Name, Kind: "terminal", Error: err.Error()}
			continue
		}
		prepared = append(prepared, preparedTerminal{
			index: i, name: t.Name, path: resolved,
			command: models.Deref(t.Command), emulator: orDefault(t.Emulator, globalEmulator),
		})
	}

	tabsAttempted := len(prepared) > 1
	var tabFailure *TabLaunchFailure
	if tabsAttempted {
		tabFailure = launchTerminalTabs(prepared)
	}
	tabsLaunched := tabsAttempted && tabFailure == nil

	// Fall back to one window per terminal when tabbing was not attempted
	// (fewer than two terminals) or failed before the emulator started.
	fallBack := !tabsAttempted || (tabFailure != nil && tabFailure.MayRetry())
	switch {
	case tabsLaunched:
		for _, t := range prepared {
			terminalResults[t.index] = &Result{Label: t.name, Kind: "terminal", Success: true,
				Detail: terminalDetail(t.path, t.command)}
		}
	case tabsAttempted && !tabFailure.MayRetry():
		// The emulator already started: some tabs may be open with their
		// commands running. Report the group as failed rather than risk a
		// second execution of every command.
		msg := fmt.Sprintf("tab launch failed after the terminal started (%s); not retried — some tabs may already be open", tabFailure.Message)
		for _, t := range prepared {
			terminalResults[t.index] = &Result{Label: t.name, Kind: "terminal", Error: msg,
				Detail: terminalDetail(t.path, t.command)}
		}
	}

	if fallBack {
		for n, t := range prepared {
			if n > 0 {
				time.Sleep(100 * time.Millisecond)
			}
			err := launchTerminal(t.path, t.command, n, t.emulator)
			r := &Result{Label: t.name, Kind: "terminal", Success: err == nil,
				Detail: terminalDetail(t.path, t.command)}
			if err != nil {
				r.Error = err.Error()
			}
			terminalResults[t.index] = r
			if n == 0 && multiple && !emulatorWasRunning && watch != "" {
				waitForEmulatorReady(watch)
			}
		}
	}

	var results []Result
	for _, r := range terminalResults {
		if r != nil {
			results = append(results, *r)
		}
	}

	if watch != "" && !emulatorWasRunning &&
		(len(prepared) == 0 && multiple || len(prepared) > 0 && tabsLaunched) {
		waitForEmulatorReady(watch)
	}

	ctx := MakePlaceholderContext(cfg, projectRoot)
	for _, app := range cfg.Applications {
		args := effectiveArgs(app, ctx, projectRoot)
		err := launchApplication(app.Path, args)
		r := Result{Label: app.Name, Kind: "app", Success: err == nil, Detail: appDetail(app.Path, args)}
		if err != nil {
			r.Error = err.Error()
		}
		results = append(results, r)
	}
	return results
}

func launchTerminalTabs(terminals []preparedTerminal) *TabLaunchFailure {
	first := terminals[0].emulator
	for _, t := range terminals {
		if t.emulator != first {
			return notStarted("mixed terminal emulators cannot share tabs")
		}
	}
	tabs := make([]Tab, len(terminals))
	resolved := make([]ResolvedTerminal, len(terminals))
	for i, t := range terminals {
		tabs[i] = Tab{Title: t.name, Cwd: t.path, Command: t.command}
		resolved[i] = ResolvedTerminal{Cwd: t.path, Command: t.command}
	}

	switch runtime.GOOS {
	case "darwin":
		switch SelectTabStrategy("darwin", first, probeTabCapabilities()) {
		case KittySession:
			return launchKittySession(tabs)
		case AppleScriptTab:
			script, err := BuildGhosttyScript(resolved)
			if err != nil {
				return notStarted(err.Error())
			}
			return runOsascript(script, "Ghostty AppleScript tabs")
		case TerminalAppTab:
			script := BuildSystemEventsTabScript(resolved)
			if ReadTabbingPreference() == TabbingAlways {
				script = BuildAutoTabScript(resolved)
			}
			return runOsascript(script, "Terminal.app tab launch")
		}
		return notStarted("unsupported tab emulator")
	case "linux":
		switch SelectTabStrategy("linux", first, probeTabCapabilities()) {
		case KittySession:
			return launchKittySession(tabs)
		case GnomeTerminalLoadConfig:
			return launchGnomeTerminalLoadConfig(tabs)
		}
		return notStarted("unsupported tab emulator")
	}
	return notStarted("batch tabs are only handled on macOS and Linux (gnome-terminal)")
}

func probeTabCapabilities() TabCapabilities {
	_, kitty := adapters.ResolveKitty()
	caps := TabCapabilities{
		GhosttyAvailable:       adapters.CommandExists("ghostty"),
		PtyxisAvailable:        adapters.CommandExists("ptyxis"),
		GnomeTerminalAvailable: adapters.CommandExists("gnome-terminal"),
		KittyAvailable:         kitty,
		WtAvailable:            adapters.CommandExists("wt"),
	}
	if runtime.GOOS == "darwin" && caps.GhosttyAvailable {
		caps.GhosttyVersion = ghosttyVersion()
		caps.GhosttyAppleScript = ghosttyAppleScriptEnabled()
	}
	return caps
}

func ghosttyVersion() string {
	out, err := exec.Command("ghostty", "+version").Output()
	if err != nil {
		return ""
	}
	return strings.TrimSpace(string(out))
}

func ghosttyAppleScriptEnabled() bool {
	out, err := exec.Command("ghostty", "+show-config").Output()
	if err != nil {
		return true
	}
	for _, line := range strings.Split(string(out), "\n") {
		if strings.TrimSpace(line) == "macos-applescript = false" {
			return false
		}
	}
	return true
}

// launchKittySession opens one kitty window with a tab per entry. kitty is
// spawned fire-and-forget, so every failure happens before it could open
// anything and is safe to retry.
func launchKittySession(tabs []Tab) *TabLaunchFailure {
	dir, err := CreateSessionDir()
	if err != nil {
		return notStarted(err.Error())
	}
	session, err := WriteKittySession(dir, tabs)
	if err != nil {
		return notStarted(err.Error())
	}
	kitty, ok := adapters.ResolveKitty()
	if !ok {
		return notStarted("kitty not found")
	}
	if err := exec.Command(kitty, "--session="+session).Start(); err != nil {
		return notStarted("kitty launch failed: " + err.Error())
	}
	return nil
}

// launchGnomeTerminalLoadConfig opens one gnome-terminal window with a tab per
// entry and checks its exit status.
func launchGnomeTerminalLoadConfig(tabs []Tab) *TabLaunchFailure {
	dir, err := CreateSessionDir()
	if err != nil {
		return notStarted(err.Error())
	}
	conf, err := WriteGnomeSession(dir, tabs)
	if err != nil {
		return notStarted(err.Error())
	}
	resolved, ok := adapters.ResolveCommand("gnome-terminal")
	if !ok {
		return notStarted("gnome-terminal not found")
	}
	cmd := exec.Command(resolved, "--load-config="+conf)
	var stderr strings.Builder
	cmd.Stderr = &stderr
	// Start and Wait separately: only a failed start proves nothing ran.
	if err := cmd.Start(); err != nil {
		return notStarted("could not start gnome-terminal: " + err.Error())
	}
	if err := cmd.Wait(); err != nil {
		var exitErr *exec.ExitError
		if !errors.As(err, &exitErr) {
			return possiblyStarted("lost track of gnome-terminal: " + err.Error())
		}
		detail := strings.TrimSpace(stderr.String())
		if detail == "" {
			detail = "gnome-terminal --load-config reported an error"
		}
		return possiblyStarted(detail)
	}
	return nil
}

// runOsascript runs an AppleScript and checks its exit status, so a denied
// Automation permission is reported instead of phantom success.
func runOsascript(script, what string) *TabLaunchFailure {
	cmd := exec.Command("osascript", "-e", script)
	var stderr strings.Builder
	cmd.Stderr = &stderr
	if err := cmd.Start(); err != nil {
		return notStarted(fmt.Sprintf("%s failed: could not start osascript: %v", what, err))
	}
	if err := cmd.Wait(); err != nil {
		var exitErr *exec.ExitError
		if !errors.As(err, &exitErr) {
			return possiblyStarted(fmt.Sprintf("%s failed: lost track of osascript: %v", what, err))
		}
		// AppleScript is not transactional: earlier statements already opened
		// tabs and submitted their commands, so this is never safe to retry.
		detail := strings.TrimSpace(stderr.String())
		if detail == "" {
			detail = "osascript reported an error"
		}
		return possiblyStarted(fmt.Sprintf("%s failed: %s", what, detail))
	}
	return nil
}

// ReadTabbingPreference reads AppleWindowTabbingMode (macOS only).
func ReadTabbingPreference() TabbingPreference {
	if runtime.GOOS != "darwin" {
		return TabbingUnknown
	}
	for _, domain := range []string{"com.apple.Terminal", "NSGlobalDomain"} {
		out, err := exec.Command("defaults", "read", domain, "AppleWindowTabbingMode").Output()
		if err != nil {
			continue
		}
		if pref := ParseTabbingPreference(string(out)); pref != TabbingUnknown {
			return pref
		}
	}
	return TabbingUnknown
}

// PromptOutcome is the result of offering to enable Terminal.app tabs.
type PromptOutcome struct {
	Accepted, Declined bool
	WriteErr           error
}

// PromptToEnableTerminalTabbing asks (macOS, interactive only) whether to set
// AppleWindowTabbingMode=always.
func PromptToEnableTerminalTabbing(declined bool) PromptOutcome {
	if runtime.GOOS != "darwin" || declined || !term.IsTerminal(int(os.Stdout.Fd())) {
		return PromptOutcome{}
	}
	ui.Print(fmt.Sprintf("%s Enable Terminal.app tabs by setting AppleWindowTabbingMode=always? %s ",
		ui.Paint(ui.Cyan.With(ui.Bold), "?"), ui.Paint(ui.Dim, "[y/N]")))
	answer, err := bufio.NewReader(os.Stdin).ReadString('\n')
	if err != nil && answer == "" {
		return PromptOutcome{}
	}
	switch strings.ToLower(strings.TrimSpace(answer)) {
	case "y", "yes":
		err := exec.Command("defaults", "write", "com.apple.Terminal", "AppleWindowTabbingMode", "-string", "always").Run()
		if err != nil {
			return PromptOutcome{WriteErr: fmt.Errorf("defaults write failed: %v", err)}
		}
		return PromptOutcome{Accepted: true}
	}
	return PromptOutcome{Declined: true}
}

func userShell() string {
	if s := os.Getenv("SHELL"); s != "" {
		return s
	}
	return "sh"
}

func shellCommand(cwd, command string) string {
	escaped := shellSingleQuoteEscape(cwd)
	if command != "" {
		return fmt.Sprintf("cd '%s' && %s; exec %s", escaped, command, userShell())
	}
	return fmt.Sprintf("cd '%s' && exec %s", escaped, userShell())
}

func launchTerminal(resolvedPath, command string, tabIndex int, emulator string) error {
	if _, err := os.Stat(resolvedPath); err != nil {
		return fmt.Errorf("path not found: %s", resolvedPath)
	}
	switch emulator {
	case "ghostty":
		return tryGhostty(resolvedPath, command)
	case "terminal":
		return runInPlatformTerminal(resolvedPath, command, tabIndex, "")
	case "gnome-terminal", "ptyxis", "kitty":
		return runInPlatformTerminal(resolvedPath, command, tabIndex, emulator)
	case "":
	default:
		return fmt.Errorf("unknown emulator: %s", emulator)
	}
	// Auto-detect on macOS: kitty, then Ghostty, then Terminal.app.
	if runtime.GOOS == "darwin" {
		if tryKitty(resolvedPath, command) == nil || tryGhostty(resolvedPath, command) == nil {
			return nil
		}
	}
	return runInPlatformTerminal(resolvedPath, command, tabIndex, "")
}

// tryKitty opens one kitty window; kitty is not single-instance.
func tryKitty(cwd, command string) error {
	kitty, ok := adapters.ResolveKitty()
	if !ok {
		return errors.New("kitty not found")
	}
	if err := exec.Command(kitty, "-d", cwd, userShell(), "-lc", shellCommand(cwd, command)).Start(); err != nil {
		return fmt.Errorf("kitty launch failed: %v", err)
	}
	return nil
}

func tryGhostty(cwd, command string) error {
	ghostty, ok := adapters.ResolveCommand("ghostty")
	if !ok {
		return errors.New("ghostty not found")
	}
	if err := exec.Command(ghostty, "-e", userShell(), "-lc", shellCommand(cwd, command)).Start(); err != nil {
		return fmt.Errorf("ghostty launch failed: %v", err)
	}
	return nil
}

func start(name string, args ...string) error {
	path, ok := adapters.ResolveCommand(name)
	if !ok {
		path = name
	}
	return exec.Command(path, args...).Start()
}

func runInPlatformTerminal(cwd, command string, tabIndex int, forced string) error {
	switch runtime.GOOS {
	case "darwin":
		switch forced {
		case "kitty":
			return tryKitty(cwd, command)
		case "":
		default:
			return fmt.Errorf("%s is only available on Linux", forced)
		}
		script := BuildWindowScript(ResolvedTerminal{Cwd: cwd, Command: command})
		if f := runOsascript(script, "Terminal.app launch"); f != nil {
			return errors.New(f.Message)
		}
		return nil

	case "windows":
		if forced != "" {
			return fmt.Errorf("%s is only available on Linux", forced)
		}
		if adapters.CommandExists("wt") {
			args := []string{"-d", cwd}
			if tabIndex > 0 {
				args = []string{"-w", "0", "new-tab", "-d", cwd}
			}
			if command != "" {
				args = append(args, "cmd", "/K", command)
			}
			if err := start("wt", args...); err != nil {
				return fmt.Errorf("wt launch failed: %v", err)
			}
			return nil
		}
		if adapters.CommandExists("pwsh") {
			ps := fmt.Sprintf("Set-Location '%s'", EscapePowerShellSingleQuotes(cwd))
			if command != "" {
				ps += "; " + command
			}
			if err := start("pwsh", "-NoExit", "-Command", ps); err != nil {
				return fmt.Errorf("pwsh launch failed: %v", err)
			}
			return nil
		}
		full := fmt.Sprintf(`cd /d "%s"`, cwd)
		if command != "" {
			full += " && " + command
		}
		if err := exec.Command("cmd", "/C", "start", "cmd", "/K", full).Start(); err != nil {
			return fmt.Errorf("cmd launch failed: %v", err)
		}
		return nil
	}

	shell := userShell()
	sc := shellCommand(cwd, command)

	if forced == "" || forced == "kitty" {
		if kitty, ok := adapters.ResolveKitty(); ok {
			if exec.Command(kitty, "-d", cwd, shell, "-lc", sc).Start() == nil {
				return nil
			}
		}
	}
	if (forced == "" || forced == "ptyxis") && adapters.CommandExists("ptyxis") {
		var args []string
		if tabIndex > 0 {
			args = append(args, "--tab")
		}
		args = append(args, "-d", cwd, "--", shell, "-lc", sc)
		if start("ptyxis", args...) == nil {
			return nil
		}
	}
	if (forced == "" || forced == "gnome-terminal") && adapters.CommandExists("gnome-terminal") {
		var args []string
		if tabIndex > 0 {
			args = append(args, "--tab")
		}
		args = append(args, "--", shell, "-lc", sc)
		if start("gnome-terminal", args...) == nil {
			return nil
		}
	}
	if forced == "" {
		for _, bin := range []string{"konsole", "alacritty", "xterm"} {
			if adapters.CommandExists(bin) && start(bin, "-e", shell, "-lc", sc) == nil {
				return nil
			}
		}
		return errors.New("no terminal emulator found")
	}
	return fmt.Errorf("%s not found", forced)
}

func launchApplication(path string, args []string) error {
	normalized := adapters.NormalizePath(path)
	// Path-only inference: substituting the tool's CLI is only safe when the
	// configured path IS that tool. Wrapper paths (flatpak, Snap, Squirrel
	// Update.exe) fall through so their saved args run against the wrapper.
	if id := adapters.InferToolIDFromPath(normalized); id != "" {
		if cli, ok := adapters.LaunchCommandForTool(id); ok && adapters.CommandExists(cli) {
			if err := start(cli, args...); err != nil {
				return fmt.Errorf("launch failed: %v", err)
			}
			return nil
		}
	}
	return launchApplicationGeneric(normalized, args)
}

func launchApplicationGeneric(executable string, args []string) error {
	if runtime.GOOS == "darwin" {
		info, err := os.Stat(executable)
		if strings.HasSuffix(executable, ".app") || err == nil && info.IsDir() {
			openArgs := []string{"-a", executable}
			if len(args) > 0 {
				openArgs = append(append(openArgs, "--args"), args...)
			}
			if err := exec.Command("open", openArgs...).Start(); err != nil {
				return fmt.Errorf("launch failed: %v", err)
			}
			return nil
		}
	}
	if err := exec.Command(executable, args...).Start(); err != nil {
		return fmt.Errorf("launch failed: %v", err)
	}
	return nil
}
