package config

import (
	"os"
	"path/filepath"
	"runtime"
	"strings"
	"sync"
	"testing"
	"time"

	"github.com/Abrar118/QuickDev/internal/models"
)

func write(t *testing.T, path, content string) {
	t.Helper()
	if err := os.MkdirAll(filepath.Dir(path), 0o755); err != nil {
		t.Fatal(err)
	}
	if err := os.WriteFile(path, []byte(content), 0o644); err != nil {
		t.Fatal(err)
	}
}

func read(t *testing.T, path string) string {
	t.Helper()
	data, err := os.ReadFile(path)
	if err != nil {
		t.Fatal(err)
	}
	return string(data)
}

func TestSaveAndLoadProjectConfig(t *testing.T) {
	path := filepath.Join(t.TempDir(), ProjectFileName)
	cfg := &models.ProjectConfig{
		Project:   models.ProjectEntry{Name: "test-proj"},
		Terminals: []models.TerminalEntry{{Name: "dev", Path: ".", Command: models.Str("npm run dev")}},
		Applications: []models.AppEntry{
			{Name: "Cursor", Path: "/Applications/Cursor.app", Args: []string{"."}},
		},
	}
	if err := SaveProjectConfig(path, cfg); err != nil {
		t.Fatal(err)
	}
	loaded, err := LoadProjectConfig(path)
	if err != nil {
		t.Fatal(err)
	}
	if loaded.Project.Name != "test-proj" || len(loaded.Terminals) != 1 || loaded.Terminals[0].Name != "dev" {
		t.Fatalf("round trip lost data: %+v", loaded)
	}
	if models.Deref(loaded.Terminals[0].Command) != "npm run dev" || loaded.Terminals[0].Emulator != nil {
		t.Fatalf("optional fields wrong: %+v", loaded.Terminals[0])
	}
	if got := loaded.Applications[0].Args; len(got) != 1 || got[0] != "." {
		t.Fatalf("args lost: %v", got)
	}
}

func TestSaveAndLoadGlobalConfig(t *testing.T) {
	path := filepath.Join(t.TempDir(), "sub", "config.toml")
	ts := int64(1234)
	cfg := &models.GlobalConfig{Projects: []models.GlobalProjectEntry{{Name: "proj-a", Path: "/a", LastLaunched: &ts}}}
	if err := SaveGlobalConfig(path, cfg); err != nil {
		t.Fatal(err)
	}
	loaded, err := LoadGlobalConfig(path)
	if err != nil {
		t.Fatal(err)
	}
	if len(loaded.Projects) != 1 || loaded.Projects[0].Name != "proj-a" || *loaded.Projects[0].LastLaunched != 1234 {
		t.Fatalf("round trip lost data: %+v", loaded)
	}
}

func TestLoadGlobalConfigMissingIsEmpty(t *testing.T) {
	loaded, err := LoadGlobalConfig(filepath.Join(t.TempDir(), "absent.toml"))
	if err != nil || len(loaded.Projects) != 0 {
		t.Fatalf("got %+v, %v", loaded, err)
	}
}

func TestProjectConfigRequiresProjectTable(t *testing.T) {
	path := filepath.Join(t.TempDir(), ProjectFileName)
	write(t, path, "[[terminals]]\nname = \"a\"\npath = \".\"\n")
	if _, err := LoadProjectConfig(path); err == nil {
		t.Fatal("a config without [project] must not load")
	}
	write(t, path, "[project]\nname = \"p\"\n[[terminals]]\nname = \"a\"\n")
	if _, err := LoadProjectConfig(path); err == nil {
		t.Fatal("a terminal without a path must not load")
	}
}

func TestFindProjectConfigWalksParents(t *testing.T) {
	root := t.TempDir()
	write(t, filepath.Join(root, ProjectFileName), "[project]\nname = \"p\"\n")
	nested := filepath.Join(root, "a", "b")
	if err := os.MkdirAll(nested, 0o755); err != nil {
		t.Fatal(err)
	}
	configPath, found, err := FindProjectConfig(nested)
	if err != nil || configPath != filepath.Join(root, ProjectFileName) || found != root {
		t.Fatalf("got %s %s %v", configPath, found, err)
	}
	if _, _, err := FindProjectConfig(t.TempDir()); err == nil {
		t.Fatal("expected not found")
	}
}

func TestUniqueProjectNameAppendsSuffix(t *testing.T) {
	cfg := &models.GlobalConfig{Projects: []models.GlobalProjectEntry{{Name: "my-app"}, {Name: "my-app-2"}}}
	if got := UniqueProjectName("my-app", cfg); got != "my-app-3" {
		t.Fatal(got)
	}
	if got := UniqueProjectName("new-proj", cfg); got != "new-proj" {
		t.Fatal(got)
	}
}

func TestNewFilesGetCommentHeaders(t *testing.T) {
	dir := t.TempDir()
	projectPath := filepath.Join(dir, ProjectFileName)
	cfg := &models.ProjectConfig{Project: models.ProjectEntry{Name: "test-proj"}}
	if err := SaveProjectConfig(projectPath, cfg); err != nil {
		t.Fatal(err)
	}
	content := read(t, projectPath)
	if !strings.HasPrefix(content, "# QuickDev project configuration") || !strings.Contains(content, "[project]\nname = \"test-proj\"") {
		t.Fatalf("unexpected content:\n%s", content)
	}

	cfg.Terminals = []models.TerminalEntry{{Name: "dev", Path: "."}}
	if err := SaveProjectConfig(projectPath, cfg); err != nil {
		t.Fatal(err)
	}
	content = read(t, projectPath)
	if strings.Count(content, "# QuickDev project configuration") != 1 || !strings.Contains(content, "[[terminals]]") {
		t.Fatalf("header must be kept once after re-save:\n%s", content)
	}

	globalPath := filepath.Join(dir, "global.toml")
	if err := SaveGlobalConfig(globalPath, &models.GlobalConfig{Emulator: models.Str("ghostty")}); err != nil {
		t.Fatal(err)
	}
	content = read(t, globalPath)
	if !strings.HasPrefix(content, "# QuickDev global configuration") || !strings.Contains(content, `emulator = "ghostty"`) {
		t.Fatalf("unexpected global:\n%s", content)
	}
}

func TestRegisterExistingProjectConfigSyncsName(t *testing.T) {
	dir := t.TempDir()
	path := filepath.Join(dir, ProjectFileName)
	write(t, path, "[project]\nname = \"api\"\n")
	global := &models.GlobalConfig{Projects: []models.GlobalProjectEntry{{Name: "api", Path: "/other"}}}
	name, err := RegisterExistingProjectConfig(path, dir, global)
	if err != nil || name != "api-2" {
		t.Fatalf("got %q %v", name, err)
	}
	if last := global.Projects[len(global.Projects)-1]; last.Name != "api-2" || last.Path != dir {
		t.Fatalf("index not updated: %+v", last)
	}
	reloaded, _ := LoadProjectConfig(path)
	if reloaded.Project.Name != "api-2" {
		t.Fatal("local name not synced")
	}
}

func TestSettings(t *testing.T) {
	cfg := &models.GlobalConfig{}
	for _, e := range []string{"ghostty", "terminal", "kitty", "gnome-terminal", "ptyxis"} {
		msg, err := SetGlobalSetting(cfg, "emulator", e)
		if err != nil || models.Deref(cfg.Emulator) != e || !strings.Contains(msg, e) {
			t.Fatalf("%s: %q %v", e, msg, err)
		}
	}
	if got, _ := GetGlobalSetting(cfg, "emulator"); got != "emulator = ptyxis" {
		t.Fatal(got)
	}
	if _, err := UnsetGlobalSetting(cfg, "emulator"); err != nil || cfg.Emulator != nil {
		t.Fatal("unset failed")
	}
	if got, _ := GetGlobalSetting(cfg, "emulator"); got != "emulator is not set (auto-detect)" {
		t.Fatal(got)
	}
	if _, err := SetGlobalSetting(cfg, "emulator", "nonexistent-terminal"); err == nil {
		t.Fatal("bad emulator accepted")
	}
	if _, err := SetGlobalSetting(cfg, "theme", "dark"); err == nil {
		t.Fatal("unknown key accepted")
	}
	if cfg.Emulator != nil {
		t.Fatal("failed set must not change config")
	}
}

func TestRewriteKeepsCommentsAndUnmodelledKeys(t *testing.T) {
	path := filepath.Join(t.TempDir(), ProjectFileName)
	write(t, path, `# my own notes about this project
[project]
name = "demo"

[[terminals]]
# the API server, do not remove
name = "api"
path = "./api"
written_by_a_newer_quickdev = "keep me"
`)
	cfg, err := LoadProjectConfig(path)
	if err != nil {
		t.Fatal(err)
	}
	cfg.Terminals = append(cfg.Terminals, models.TerminalEntry{Name: "web", Path: "./web", Command: models.Str("npm run dev")})
	if err := SaveProjectConfig(path, cfg); err != nil {
		t.Fatal(err)
	}
	content := read(t, path)
	for _, want := range []string{"# my own notes about this project", "# the API server, do not remove", `written_by_a_newer_quickdev = "keep me"`} {
		if !strings.Contains(content, want) {
			t.Fatalf("lost %q:\n%s", want, content)
		}
	}
	if !strings.HasPrefix(content, "# my own notes") {
		t.Fatalf("header moved:\n%s", content)
	}
	reloaded, _ := LoadProjectConfig(path)
	if len(reloaded.Terminals) != 2 || reloaded.Terminals[1].Name != "web" || models.Deref(reloaded.Terminals[1].Command) != "npm run dev" {
		t.Fatalf("new terminal missing: %+v", reloaded.Terminals)
	}
}

func TestRewriteKeepsUnknownTablesAndValueFormatting(t *testing.T) {
	path := filepath.Join(t.TempDir(), ProjectFileName)
	write(t, path, `[project]
name = "demo"   # trailing comment

[future]
setting = 1

[[terminals]]
name = 'api'
path = '.'
`)
	cfg, err := LoadProjectConfig(path)
	if err != nil {
		t.Fatal(err)
	}
	cfg.Applications = append(cfg.Applications, models.AppEntry{Name: "Zed", Path: "zed", Args: []string{"{root}"}})
	if err := SaveProjectConfig(path, cfg); err != nil {
		t.Fatal(err)
	}
	content := read(t, path)
	for _, want := range []string{`name = "demo"   # trailing comment`, "[future]\nsetting = 1", "name = 'api'", `args = ["{root}"]`} {
		if !strings.Contains(content, want) {
			t.Fatalf("missing %q:\n%s", want, content)
		}
	}
}

func TestRemovingAnOptionalFieldClearsIt(t *testing.T) {
	path := filepath.Join(t.TempDir(), ProjectFileName)
	write(t, path, "[project]\nname = \"demo\"\n\n[[terminals]]\nname = \"api\"\npath = \".\"\ncommand = \"echo hi\"\n")
	cfg, _ := LoadProjectConfig(path)
	cfg.Terminals[0].Command = nil
	if err := SaveProjectConfig(path, cfg); err != nil {
		t.Fatal(err)
	}
	if content := read(t, path); strings.Contains(content, "command") {
		t.Fatalf("stale key left behind:\n%s", content)
	}
}

func TestRemovingAnEntryDropsItsTable(t *testing.T) {
	path := filepath.Join(t.TempDir(), ProjectFileName)
	write(t, path, "[project]\nname = \"demo\"\n\n[[terminals]]\nname = \"api\"\npath = \".\"\n\n[[terminals]]\nname = \"web\"\npath = \".\"\n\n[[applications]]\nname = \"Zed\"\npath = \"zed\"\n")
	cfg, _ := LoadProjectConfig(path)
	cfg.Terminals = cfg.Terminals[1:]
	cfg.Applications = nil
	if err := SaveProjectConfig(path, cfg); err != nil {
		t.Fatal(err)
	}
	want := "[project]\nname = \"demo\"\n\n[[terminals]]\nname = \"web\"\npath = \".\"\n"
	if got := read(t, path); got != want {
		t.Fatalf("got:\n%q\nwant:\n%q", got, want)
	}
}

func TestAddingTopLevelKeyToGlobalWithProjectsStaysValid(t *testing.T) {
	path := filepath.Join(t.TempDir(), "config.toml")
	write(t, path, "terminal_app_tabbing_prompt_declined = false\n\n[[projects]]\nname = \"demo\"\npath = \"/tmp/demo\"\n")
	global, _ := LoadGlobalConfig(path)
	global.Emulator = models.Str("kitty")
	if err := SaveGlobalConfig(path, global); err != nil {
		t.Fatal(err)
	}
	reloaded, err := LoadGlobalConfig(path)
	if err != nil || models.Deref(reloaded.Emulator) != "kitty" || len(reloaded.Projects) != 1 || reloaded.Projects[0].Name != "demo" {
		t.Fatalf("got %+v %v\n%s", reloaded, err, read(t, path))
	}
}

func TestUnsupportedLayoutsAreRegeneratedNotCorrupted(t *testing.T) {
	path := filepath.Join(t.TempDir(), ProjectFileName)
	write(t, path, "# header\nproject = { name = \"demo\" }\nterminals = [{ name = \"api\", path = \".\" }]\n")
	cfg, err := LoadProjectConfig(path)
	if err != nil {
		t.Fatal(err)
	}
	cfg.Terminals = append(cfg.Terminals, models.TerminalEntry{Name: "web", Path: "."})
	if err := SaveProjectConfig(path, cfg); err != nil {
		t.Fatal(err)
	}
	content := read(t, path)
	if !strings.HasPrefix(content, "# header\n") {
		t.Fatalf("header lost:\n%s", content)
	}
	reloaded, err := LoadProjectConfig(path)
	if err != nil || len(reloaded.Terminals) != 2 {
		t.Fatalf("got %+v %v\n%s", reloaded, err, content)
	}
}

func TestSavingOverUnparseableConfigRefuses(t *testing.T) {
	path := filepath.Join(t.TempDir(), ProjectFileName)
	broken := "[project\nname = \"demo\"\n"
	write(t, path, broken)
	remember(path, &broken)
	if err := SaveProjectConfig(path, &models.ProjectConfig{Project: models.ProjectEntry{Name: "demo"}}); err == nil {
		t.Fatal("expected an error")
	}
	if read(t, path) != broken {
		t.Fatal("broken file was overwritten")
	}
}

func TestSavingKeepsFilePermissions(t *testing.T) {
	if runtime.GOOS == "windows" {
		t.Skip("unix permissions")
	}
	path := filepath.Join(t.TempDir(), ProjectFileName)
	write(t, path, "[project]\nname = \"demo\"\n")
	if err := os.Chmod(path, 0o644); err != nil {
		t.Fatal(err)
	}
	cfg, _ := LoadProjectConfig(path)
	if err := SaveProjectConfig(path, cfg); err != nil {
		t.Fatal(err)
	}
	info, _ := os.Stat(path)
	if info.Mode().Perm() != 0o644 {
		t.Fatalf("mode changed to %o", info.Mode().Perm())
	}
}

func TestSavingRefusesWhenFileChangedSinceRead(t *testing.T) {
	path := filepath.Join(t.TempDir(), ProjectFileName)
	write(t, path, "[project]\nname = \"demo\"\n")
	cfg, _ := LoadProjectConfig(path)
	write(t, path, "[project]\nname = \"demo\"\n\n[[terminals]]\nname = \"added-elsewhere\"\npath = \".\"\n")
	err := SaveProjectConfig(path, cfg)
	if err == nil || !strings.Contains(err.Error(), "changed on disk") {
		t.Fatalf("got %v", err)
	}
	if !strings.Contains(read(t, path), "added-elsewhere") {
		t.Fatal("the concurrent write must survive")
	}
}

func TestSavingRefusesToOverwriteUnreadFile(t *testing.T) {
	path := filepath.Join(t.TempDir(), ProjectFileName)
	existing := "[project]\nname = \"someone-elses\"\n"
	write(t, path, existing)
	if err := SaveProjectConfig(path, &models.ProjectConfig{Project: models.ProjectEntry{Name: "mine"}}); err == nil {
		t.Fatal("expected refusal")
	}
	if read(t, path) != existing {
		t.Fatal("file was clobbered")
	}
}

func TestRepeatedSavesInOneProcessAreAllowed(t *testing.T) {
	path := filepath.Join(t.TempDir(), ProjectFileName)
	cfg := &models.ProjectConfig{Project: models.ProjectEntry{Name: "demo"}}
	if err := SaveProjectConfig(path, cfg); err != nil {
		t.Fatal(err)
	}
	cfg.Terminals = append(cfg.Terminals, models.TerminalEntry{Name: "api", Path: "."})
	if err := SaveProjectConfig(path, cfg); err != nil {
		t.Fatal(err)
	}
	if loaded, _ := LoadProjectConfig(path); len(loaded.Terminals) != 1 {
		t.Fatal("second save lost")
	}
}

func seededConfig(t *testing.T, dir string) string {
	path := filepath.Join(dir, ProjectFileName)
	write(t, path, "[project]\nname = \"demo\"\n")
	return path
}

func stagedLeftovers(t *testing.T, dir string) []string {
	entries, err := os.ReadDir(dir)
	if err != nil {
		t.Fatal(err)
	}
	var out []string
	for _, e := range entries {
		if strings.HasPrefix(e.Name(), ".quickdev-deregister-") {
			out = append(out, filepath.Join(dir, e.Name()))
		}
	}
	return out
}

func ok() (struct{}, error) { return struct{}{}, nil }

func TestRemoveConfigWithCommits(t *testing.T) {
	dir := t.TempDir()
	path := seededConfig(t, dir)
	if _, err := RemoveConfigWith(path, ok); err != nil {
		t.Fatal(err)
	}
	if exists(path) || len(stagedLeftovers(t, dir)) != 0 {
		t.Fatal("config or staging file left behind")
	}
}

func TestRemoveConfigWithKeepsEarlierRecoveryCopy(t *testing.T) {
	dir := t.TempDir()
	path := seededConfig(t, dir)
	earlier := filepath.Join(dir, ".quickdev-deregister-earlier")
	write(t, earlier, "an earlier recovery copy")
	if _, err := RemoveConfigWith(path, ok); err != nil {
		t.Fatal(err)
	}
	if read(t, earlier) != "an earlier recovery copy" {
		t.Fatal("recovery copy clobbered")
	}
}

func TestRemoveConfigWithRestoresOnFailedCommit(t *testing.T) {
	dir := t.TempDir()
	path := seededConfig(t, dir)
	_, err := RemoveConfigWith(path, func() (struct{}, error) {
		return struct{}{}, errString("index write failed")
	})
	if err == nil || err.Error() != "index write failed" {
		t.Fatalf("got %v", err)
	}
	if read(t, path) != "[project]\nname = \"demo\"\n" || len(stagedLeftovers(t, dir)) != 0 {
		t.Fatal("config not restored exactly")
	}
}

func TestRemoveConfigWithNamesRecoveryPathWhenRestoreFails(t *testing.T) {
	dir := t.TempDir()
	path := seededConfig(t, dir)
	_, err := RemoveConfigWith(path, func() (struct{}, error) {
		// Occupying the path with a non-empty directory makes the restoring
		// rename fail.
		if err := os.MkdirAll(filepath.Join(path, "x"), 0o755); err != nil {
			t.Fatal(err)
		}
		return struct{}{}, errString("index write failed")
	})
	if err == nil || !strings.Contains(err.Error(), "index write failed") || !strings.Contains(err.Error(), "could not be put back") {
		t.Fatalf("got %v", err)
	}
	leftovers := stagedLeftovers(t, dir)
	if len(leftovers) != 1 || !strings.Contains(err.Error(), leftovers[0]) {
		t.Fatalf("recovery path not named: %v %v", leftovers, err)
	}
	if read(t, leftovers[0]) != "[project]\nname = \"demo\"\n" {
		t.Fatal("staged copy damaged")
	}
}

func TestRemoveConfigWithReportsFailedCleanup(t *testing.T) {
	dir := t.TempDir()
	path := seededConfig(t, dir)
	_, err := RemoveConfigWith(path, func() (struct{}, error) {
		staged := stagedLeftovers(t, dir)[0]
		os.Remove(staged)
		os.MkdirAll(filepath.Join(staged, "x"), 0o755)
		return struct{}{}, nil
	})
	if err == nil || !strings.Contains(err.Error(), "could not be deleted") {
		t.Fatalf("got %v", err)
	}
	if exists(path) {
		t.Fatal("the commit still happened, so the config must be gone")
	}
}

func TestConcurrentSaveCannotResurrectDeletedConfig(t *testing.T) {
	dir := t.TempDir()
	path := seededConfig(t, dir)
	pending, err := LoadProjectConfig(path)
	if err != nil {
		t.Fatal(err)
	}
	const held = 300 * time.Millisecond
	gate := make(chan struct{})
	var wg sync.WaitGroup
	var took time.Duration
	wg.Add(1)
	go func() {
		defer wg.Done()
		<-gate
		start := time.Now()
		_ = SaveProjectConfig(path, pending)
		took = time.Since(start)
	}()
	_, err = RemoveConfigWith(path, func() (struct{}, error) {
		close(gate)
		time.Sleep(held)
		return struct{}{}, nil
	})
	wg.Wait()
	if err != nil {
		t.Fatal(err)
	}
	if exists(path) {
		t.Fatal("a config is back on disk after a successful delete")
	}
	if took < held/2 {
		t.Fatalf("the save was not blocked by the delete's lock (took %v)", took)
	}
}

func TestMigrationMovesAndRetiresLegacyFile(t *testing.T) {
	dir := t.TempDir()
	legacy := filepath.Join(dir, "Documents/quickdev/config.toml")
	path := filepath.Join(dir, ".config/quickdev/config.toml")
	content := "# keep my comment\n[[projects]]\nname = \"api\"\npath = \"/p/api\"\n"
	write(t, legacy, content)
	if err := MigrateGlobalConfig(legacy, path); err != nil {
		t.Fatal(err)
	}
	if read(t, path) != content || exists(legacy) || !exists(legacy+".migrated") {
		t.Fatal("migration incomplete")
	}
}

func TestMigrationNeverOverwritesExistingConfig(t *testing.T) {
	dir := t.TempDir()
	legacy := filepath.Join(dir, "old.toml")
	path := filepath.Join(dir, "new/config.toml")
	write(t, legacy, "old")
	write(t, path, "new")
	if err := MigrateGlobalConfig(legacy, path); err != nil {
		t.Fatal(err)
	}
	if read(t, path) != "new" || read(t, legacy) != "old" {
		t.Fatal("existing config touched")
	}
}

func TestMigratingAgainKeepsEarlierBackup(t *testing.T) {
	dir := t.TempDir()
	legacy := filepath.Join(dir, "Documents/config.toml")
	path := filepath.Join(dir, ".config/quickdev/config.toml")
	write(t, legacy+".migrated", "earlier index")
	write(t, legacy, "newer index")
	if err := MigrateGlobalConfig(legacy, path); err != nil {
		t.Fatal(err)
	}
	if read(t, path) != "newer index" || read(t, legacy+".migrated") != "earlier index" ||
		exists(legacy) || read(t, legacy+".migrated.2") != "newer index" {
		t.Fatal("backup handling wrong")
	}
}

func TestRecordedLaunchesOrderMostRecentFirst(t *testing.T) {
	path := filepath.Join(t.TempDir(), "config.toml")
	write(t, path, "# my notes\nterminal_app_tabbing_prompt_declined = false\n\n[[projects]]\nname = \"a\"\npath = \"/a\"\n\n[[projects]]\nname = \"b\"\npath = \"/b\"\n\n[[projects]]\nname = \"c\"\npath = \"/c\"\nlast_launched = 100\n")
	if err := RecordLaunch(path, "/b"); err != nil {
		t.Fatal(err)
	}
	if err := RecordLaunch(path, "/not-registered"); err != nil {
		t.Fatal(err)
	}
	global, _ := LoadGlobalConfig(path)
	var order []string
	for _, p := range ProjectsByRecency(global) {
		order = append(order, p.Name)
	}
	if strings.Join(order, ",") != "b,c,a" {
		t.Fatalf("order %v", order)
	}
	if !strings.HasPrefix(read(t, path), "# my notes") {
		t.Fatal("comment lost")
	}
}

func TestProjectStatusAndPrune(t *testing.T) {
	healthy := t.TempDir()
	write(t, filepath.Join(healthy, ProjectFileName), "[project]\nname = \"h\"\n")
	noConfig := t.TempDir()
	global := &models.GlobalConfig{Projects: []models.GlobalProjectEntry{
		{Name: "gone", Path: filepath.Join(healthy, "missing")},
		{Name: "healthy", Path: healthy},
		{Name: "noconfig", Path: noConfig},
	}}
	statuses := ProjectStatuses(global)
	if statuses[0].Issue() != "path missing" || !statuses[1].IsHealthy() || statuses[2].Issue() != ".quickdev.toml missing" {
		t.Fatalf("statuses %+v", statuses)
	}
	if missing := MissingStatuses(statuses); len(missing) != 2 {
		t.Fatalf("missing %+v", missing)
	}
	removed := PruneProjects(global)
	if strings.Join(removed, ",") != "gone,noconfig" || len(global.Projects) != 1 {
		t.Fatalf("removed %v, left %+v", removed, global.Projects)
	}
}

func TestProjectsJSON(t *testing.T) {
	if ProjectsJSON(nil) != "[]" {
		t.Fatal("empty must be []")
	}
	ts := int64(42)
	out := ProjectsJSON([]ProjectStatus{{Name: "a\"b\n", Path: `C:\x`, PathExists: true, LastLaunched: &ts}})
	for _, want := range []string{`"name": "a\"b\u000a"`, `"path": "C:\\x"`, `"healthy": false`, `"config_exists": false`, `"last_launched": 42`} {
		if !strings.Contains(out, want) {
			t.Fatalf("missing %s in %s", want, out)
		}
	}
}

func TestGlobalConfigPathHonorsOverrideAndXDG(t *testing.T) {
	t.Setenv("QUICKDEV_CONFIG", "/custom/path.toml")
	if p, _ := GlobalConfigPath(); p != "/custom/path.toml" {
		t.Fatal(p)
	}
	if runtime.GOOS == "windows" {
		return
	}
	home := t.TempDir()
	t.Setenv("QUICKDEV_CONFIG", "")
	t.Setenv("HOME", home)
	t.Setenv("XDG_CONFIG_HOME", "relative/ignored")
	if p, _ := GlobalConfigPath(); p != filepath.Join(home, ".config/quickdev/config.toml") {
		t.Fatal(p)
	}
	xdg := t.TempDir()
	t.Setenv("XDG_CONFIG_HOME", xdg)
	if p, _ := GlobalConfigPath(); p != filepath.Join(xdg, "quickdev/config.toml") {
		t.Fatal(p)
	}
}

type errString string

func (e errString) Error() string { return string(e) }
