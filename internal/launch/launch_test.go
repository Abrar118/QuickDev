package launch

import (
	"os"
	"path/filepath"
	"reflect"
	"runtime"
	"strings"
	"testing"
	"time"

	"github.com/Abrar118/QuickDev/internal/models"
	"github.com/Abrar118/QuickDev/internal/ui"
)

func TestEmulatorWatchProcess(t *testing.T) {
	cases := []struct {
		emulator               string
		ghostty, ptyxis, kitty bool
		goos, want             string
	}{
		{"", true, false, false, "darwin", "ghostty"},
		{"", true, false, false, "linux", "gnome-terminal-server"},
		{"", true, true, false, "linux", "ptyxis"},
		{"", false, false, false, "darwin", "Terminal"},
		{"", false, false, false, "windows", "WindowsTerminal.exe"},
		{"ghostty", false, false, false, "linux", "ghostty"},
		{"terminal", false, false, false, "darwin", "Terminal"},
		{"terminal", false, true, false, "linux", "ptyxis"},
		{"kitty", true, false, false, "linux", ""},
		{"kitty", false, false, true, "darwin", ""},
		{"gnome-terminal", false, false, false, "linux", "gnome-terminal-server"},
		{"gnome-terminal", false, false, false, "darwin", ""},
		{"ptyxis", false, true, false, "linux", "ptyxis"},
		{"ptyxis", false, false, false, "windows", ""},
		{"", true, true, true, "linux", ""},
		{"", true, false, true, "darwin", ""},
	}
	for _, c := range cases {
		if got := EmulatorWatchProcess(c.emulator, c.ghostty, c.ptyxis, c.kitty, c.goos); got != c.want {
			t.Errorf("%+v: got %q", c, got)
		}
	}
}

func TestPollUntil(t *testing.T) {
	n := 0
	if !PollUntil(func() bool { n++; return n >= 3 }, 5, 0) || n != 3 {
		t.Fatal("should succeed on the third try")
	}
	if PollUntil(func() bool { return false }, 3, time.Duration(0)) {
		t.Fatal("never satisfied")
	}
}

func TestPgrepArgs(t *testing.T) {
	if !reflect.DeepEqual(PgrepArgsForProcess("gnome-terminal-server"), []string{"-f", "gnome-terminal-server"}) ||
		!reflect.DeepEqual(PgrepArgsForProcess("ghostty"), []string{"-x", "ghostty"}) {
		t.Fatal("pgrep args wrong")
	}
}

func TestEscaping(t *testing.T) {
	if EscapeAppleScriptString(`a"b\c`) != `a\"b\\c` || EscapeAppleScriptString("plain") != "plain" {
		t.Fatal("applescript")
	}
	if EscapePowerShellSingleQuotes("it's") != "it''s" {
		t.Fatal("powershell")
	}
}

func TestRenderResults(t *testing.T) {
	out := RenderResults("Launched 2/3 items", []Result{
		{Label: "dev", Kind: "terminal", Success: true, Detail: "/home/user/p · npm run dev"},
		{Label: "Cursor", Kind: "app", Success: true, Detail: "/Applications/Cursor.app"},
		{Label: "logs", Kind: "terminal", Error: "bad path"},
	})
	want := "Launched 2/3 items\n\n  ✓ terminal  dev     /home/user/p · npm run dev\n  ✓ app       Cursor  /Applications/Cursor.app\n  ✗ terminal  logs    bad path\n"
	if got := ui.Strip(out); got != want {
		t.Fatalf("got:\n%q\nwant:\n%q", got, want)
	}
}

func TestPlanLaunch(t *testing.T) {
	root := t.TempDir()
	os.Mkdir(filepath.Join(root, "src"), 0o755)
	app := filepath.Join(root, "Cursor.app")
	os.Mkdir(app, 0o755)
	cfg := &models.ProjectConfig{
		Project: models.ProjectEntry{Name: "p"},
		Terminals: []models.TerminalEntry{
			{Name: "dev", Path: "./src", Command: models.Str("npm run dev")},
			{Name: "gone", Path: "./missing"},
			{Name: "bad", Path: "../escape"},
		},
		Applications: []models.AppEntry{
			{Name: "Cursor", Path: app},
			{Name: "Cursor2", Path: app, Args: []string{".", "--flag"}},
			{Name: "Nope", Path: "/definitely/not/installed.app"},
		},
	}
	plan := PlanLaunch(cfg, root)
	if !plan[0].Success || plan[0].Detail != filepath.Join(root, "src")+" · npm run dev" {
		t.Errorf("terminal: %+v", plan[0])
	}
	if plan[1].Success || plan[2].Success || plan[2].Error == "" {
		t.Errorf("missing/escaping terminals must fail: %+v %+v", plan[1], plan[2])
	}
	if !plan[3].Success || plan[3].Detail != app+" · args: "+root {
		t.Errorf("editor with no args should open the root: %+v", plan[3])
	}
	if plan[4].Detail != app+" · args: "+root+" --flag" {
		t.Errorf("args not resolved: %+v", plan[4])
	}
	if plan[5].Success {
		t.Errorf("missing app planned as success: %+v", plan[5])
	}
	if runtime.GOOS != "windows" {
		onPath := PlanLaunch(&models.ProjectConfig{Applications: []models.AppEntry{{Name: "shell", Path: "sh"}}}, root)
		if !onPath[0].Success {
			t.Error("an app found on PATH is launchable")
		}
	}
}

func TestTabLaunchFailureRetry(t *testing.T) {
	if !notStarted("kitty not found").MayRetry() || possiblyStarted("osascript error").MayRetry() {
		t.Fatal("only pre-start failures may be retried")
	}
}

func TestSelectTabStrategy(t *testing.T) {
	supported := TabCapabilities{GhosttyAvailable: true, GhosttyVersion: "Ghostty 1.3.0", GhosttyAppleScript: true}
	cases := []struct {
		goos, emulator string
		caps           TabCapabilities
		want           TabStrategy
	}{
		{"darwin", "ghostty", supported, AppleScriptTab},
		{"darwin", "", supported, AppleScriptTab},
		{"darwin", "ghostty", TabCapabilities{GhosttyAvailable: true, GhosttyVersion: "1.2.9", GhosttyAppleScript: true}, WindowOnly},
		{"darwin", "ghostty", TabCapabilities{GhosttyAvailable: true, GhosttyVersion: "1.3.0"}, WindowOnly},
		{"darwin", "", TabCapabilities{GhosttyAvailable: true, GhosttyVersion: "1.2.0"}, WindowOnly},
		{"darwin", "", TabCapabilities{}, TerminalAppTab},
		{"darwin", "terminal", TabCapabilities{KittyAvailable: true}, TerminalAppTab},
		{"darwin", "ghostty", TabCapabilities{KittyAvailable: true}, WindowOnly},
		{"darwin", "kitty", TabCapabilities{KittyAvailable: true}, KittySession},
		{"darwin", "kitty", TabCapabilities{}, WindowOnly},
		{"darwin", "", TabCapabilities{KittyAvailable: true, GhosttyAvailable: true, GhosttyVersion: "1.3", GhosttyAppleScript: true}, KittySession},
		{"linux", "", TabCapabilities{PtyxisAvailable: true, GnomeTerminalAvailable: true}, WindowOnly},
		{"linux", "", TabCapabilities{GnomeTerminalAvailable: true}, GnomeTerminalLoadConfig},
		{"linux", "gnome-terminal", TabCapabilities{GnomeTerminalAvailable: true, PtyxisAvailable: true}, GnomeTerminalLoadConfig},
		{"linux", "ptyxis", TabCapabilities{PtyxisAvailable: true}, WindowOnly},
		{"linux", "ghostty", TabCapabilities{}, WindowOnly},
		{"linux", "", TabCapabilities{}, WindowOnly},
		{"linux", "kitty", TabCapabilities{KittyAvailable: true}, KittySession},
		{"linux", "kitty", TabCapabilities{}, WindowOnly},
		{"linux", "", TabCapabilities{KittyAvailable: true, PtyxisAvailable: true, GnomeTerminalAvailable: true}, KittySession},
		{"windows", "", TabCapabilities{}, CliTab},
	}
	for _, c := range cases {
		if got := SelectTabStrategy(c.goos, c.emulator, c.caps); got != c.want {
			t.Errorf("%s/%q %+v: got %v want %v", c.goos, c.emulator, c.caps, got, c.want)
		}
	}
}

func TestGhosttyScript(t *testing.T) {
	script, err := BuildGhosttyScript([]ResolvedTerminal{{Cwd: "/Users/me/project", Command: "npm run dev"}, {Cwd: "/Users/me/project/logs"}})
	if err != nil {
		t.Fatal(err)
	}
	for _, want := range []string{`tell application "Ghostty"`, "set w to new window with configuration c0", "new tab in w with configuration c1",
		`initial working directory:"/Users/me/project"`, `initial input:"npm run dev" & return`} {
		if !strings.Contains(script, want) {
			t.Errorf("missing %q in\n%s", want, script)
		}
	}
	escaped, _ := BuildGhosttyScript([]ResolvedTerminal{{Cwd: `/Users/me/Quote "Project"/a\b`, Command: `printf "ok\done"`}})
	if !strings.Contains(escaped, `Quote \"Project\"/a\\b`) || !strings.Contains(escaped, `initial input:"printf \"ok\\done\"" & return`) {
		t.Errorf("escaping wrong:\n%s", escaped)
	}
	if _, err := BuildGhosttyScript(nil); err == nil {
		t.Error("empty script accepted")
	}
}

func TestTerminalAppScripts(t *testing.T) {
	if got := BuildTerminalCommand("/Users/o'brien", "ls"); got != `cd '/Users/o'\''brien' && ls` {
		t.Fatal(got)
	}
	script := BuildAutoTabScript([]ResolvedTerminal{{Cwd: "/Users/me/project", Command: "npm run dev"}, {Cwd: "/Users/me/project/logs"}})
	if !strings.Contains(script, `do script "cd '/Users/me/project' && npm run dev"`) ||
		!strings.Contains(script, `do script "cd '/Users/me/project/logs'"`) || strings.Contains(script, "System Events") {
		t.Fatal(script)
	}
	if !strings.Contains(BuildSystemEventsTabScript([]ResolvedTerminal{{Cwd: "/a"}, {Cwd: "/b"}}), "in selected tab of front window") {
		t.Fatal("system events script")
	}
	if ParseTabbingPreference("always\n") != TabbingAlways || ParseTabbingPreference("manual") != TabbingManual || ParseTabbingPreference("") != TabbingUnknown {
		t.Fatal("tabbing preference")
	}
}

func TestWrapperAndLoadConfig(t *testing.T) {
	s := BuildWrapperScript("/home/me/api", "npm run dev")
	if !strings.HasPrefix(s, "#!/bin/sh\n") || !strings.Contains(s, "cd '/home/me/api' || exit 1\n") ||
		!strings.Contains(s, "npm run dev\n") || !strings.HasSuffix(strings.TrimSpace(s), `exec "${SHELL:-/bin/sh}" -l`) {
		t.Fatal(s)
	}
	if !strings.Contains(BuildWrapperScript("/home/o'brien", ""), `cd '/home/o'\''brien' || exit 1`) {
		t.Fatal("cwd quoting")
	}
	conf := BuildLoadConfig([]SessionTab{{"api", "/tmp/qd/tab0.sh"}, {"web", "/tmp/my path/tab1.sh"}})
	for _, want := range []string{"[GNOME Terminal Configuration]", "Windows=Window0;", "Terminals=Terminal0;Terminal1;",
		"[Terminal0]", "Title=api", "Command=/bin/sh '/tmp/qd/tab0.sh'", "Command=/bin/sh '/tmp/my path/tab1.sh'"} {
		if !strings.Contains(conf, want) {
			t.Errorf("missing %q", want)
		}
	}
	if EscapeGKeyFileValue("a\\b\nc") != `a\\b\nc` {
		t.Error("gkeyfile escape")
	}
}

func TestKittySession(t *testing.T) {
	session := BuildKittySession([]SessionTab{{"api", "/tmp/qd/tab0.sh"}, {"web", "/tmp/qd/tab1.sh"}, {"logs", "/tmp/qd/tab2.sh"}})
	if !strings.HasPrefix(session, "launch --title 'api' /bin/sh '/tmp/qd/tab0.sh'") || strings.Count(session, "new_tab") != 2 ||
		!strings.Contains(session, "new_tab web\nlaunch /bin/sh '/tmp/qd/tab1.sh'") {
		t.Fatal(session)
	}
	if !strings.Contains(BuildKittySession([]SessionTab{{"o'brien", "/x"}}), `--title 'o'\''brien'`) {
		t.Fatal("title quoting")
	}
	hostile := BuildKittySession([]SessionTab{{"first\nlaunch /bin/sh '/evil.sh'", "/a"}, {"second\nlaunch /bin/sh '/evil.sh'", "/b"}})
	lines := strings.Split(strings.TrimSuffix(hostile, "\n"), "\n")
	if len(lines) != 3 {
		t.Fatalf("title injected lines: %q", lines)
	}
	if SanitizeTitle("a\r\nb\tc") != "abc" {
		t.Fatal("sanitize")
	}
}

func TestSessionFiles(t *testing.T) {
	if runtime.GOOS == "windows" {
		t.Skip("unix permissions")
	}
	dir := t.TempDir()
	tabs := []Tab{{Title: "api", Cwd: "/tmp", Command: "echo hi"}, {Title: "web", Cwd: "/"}}
	session, err := WriteKittySession(dir, tabs)
	if err != nil {
		t.Fatal(err)
	}
	body, _ := os.ReadFile(session)
	if !strings.Contains(string(body), "launch --title 'api' /bin/sh '") || !strings.Contains(string(body), "new_tab web") {
		t.Fatal(string(body))
	}
	info, _ := os.Stat(filepath.Join(dir, "tab0.sh"))
	sinfo, _ := os.Stat(session)
	if info.Mode().Perm() != 0o700 || sinfo.Mode().Perm() != 0o600 {
		t.Fatalf("modes %o %o", info.Mode().Perm(), sinfo.Mode().Perm())
	}
	// A second write into the same directory must refuse the existing paths.
	if _, err := WriteGnomeSession(dir, tabs); err == nil {
		t.Fatal("existing wrapper overwritten")
	}

	gdir := t.TempDir()
	conf, err := WriteGnomeSession(gdir, tabs)
	if err != nil {
		t.Fatal(err)
	}
	cbody, _ := os.ReadFile(conf)
	if !strings.Contains(string(cbody), "Terminals=Terminal0;Terminal1;") {
		t.Fatal(string(cbody))
	}
}

func TestWriteNewRefusesSymlinks(t *testing.T) {
	if runtime.GOOS == "windows" {
		t.Skip("symlinks")
	}
	dir := t.TempDir()
	victim := filepath.Join(dir, "victim")
	os.WriteFile(victim, []byte("original"), 0o644)
	link := filepath.Join(dir, "wrapper.sh")
	os.Symlink(victim, link)
	if WriteNew(link, "planted", 0o700) == nil {
		t.Fatal("followed a symlink")
	}
	if data, _ := os.ReadFile(victim); string(data) != "original" {
		t.Fatal("victim clobbered")
	}
}

func TestSessionDirAndReaper(t *testing.T) {
	if runtime.GOOS == "windows" {
		t.Skip("unix permissions")
	}
	t.Setenv("TMPDIR", t.TempDir())
	a, err := CreateSessionDir()
	if err != nil {
		t.Fatal(err)
	}
	b, _ := CreateSessionDir()
	info, _ := os.Stat(a)
	if a == b || info.Mode().Perm() != 0o700 || !strings.HasPrefix(filepath.Base(a), sessionPrefix) {
		t.Fatalf("%s %s %o", a, b, info.Mode().Perm())
	}
	later := time.Now().Add(2 * time.Hour)
	if !IsReapable(a, later) || IsReapable(a, time.Now()) {
		t.Fatal("age rule wrong")
	}
	unmarked := filepath.Join(os.TempDir(), sessionPrefix+"unmarked")
	os.Mkdir(unmarked, 0o700)
	other := filepath.Join(os.TempDir(), "quickdev-other")
	os.Mkdir(other, 0o700)
	os.WriteFile(filepath.Join(other, sessionMarker), nil, 0o600)
	if IsReapable(unmarked, later) || IsReapable(other, later) {
		t.Fatal("reaper must only target marked session directories")
	}
}

func TestVersionParsing(t *testing.T) {
	cases := map[string][2]int{"Ghostty 1.3.0": {1, 3}, "2": {2, 0}, "v1.10.2-dev": {1, 10}}
	for in, want := range cases {
		major, minor, ok := parseVersionMajorMinor(in)
		if !ok || major != want[0] || minor != want[1] {
			t.Errorf("%q -> %d.%d %v", in, major, minor, ok)
		}
	}
	if _, _, ok := parseVersionMajorMinor("none"); ok {
		t.Error("no version parsed as ok")
	}
}
