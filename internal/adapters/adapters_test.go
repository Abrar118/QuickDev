package adapters

import (
	"os"
	"path/filepath"
	"reflect"
	"runtime"
	"strings"
	"testing"
)

func TestInferToolID(t *testing.T) {
	cases := []struct{ name, path, want string }{
		{"Cursor", "/Applications/Cursor.app", "cursor"},
		{"VS Code", "/Applications/Visual Studio Code.app", "vscode"},
		{"code", "/usr/bin/code", "vscode"},
		{"Zed", "/Applications/Zed.app", "zed"},
		{"Firefox", "/usr/bin/firefox", ""},
		{"Codex", "/usr/bin/codex", ""},
		{"Visual Studio Code", "flatpak", "vscode"},
	}
	for _, c := range cases {
		if got := InferToolID(c.name, c.path); got != c.want {
			t.Errorf("InferToolID(%q,%q)=%q want %q", c.name, c.path, got, c.want)
		}
	}
}

func TestInferToolIDFromPathIgnoresWrappers(t *testing.T) {
	for _, p := range []string{"flatpak", "/usr/bin/snap", `C:\Users\me\AppData\Local\Discord\Update.exe`} {
		if got := InferToolIDFromPath(p); got != "" {
			t.Errorf("%q -> %q", p, got)
		}
	}
	if InferToolIDFromPath(`C:\Programs\Microsoft VS Code\Code.exe`) != "vscode" {
		t.Error("Code.exe not detected")
	}
}

func TestToolTable(t *testing.T) {
	if !IsEditorTool("vscode") || !IsEditorTool("zed") || IsEditorTool("ghostty") {
		t.Fatal("editor tools wrong")
	}
	if c, ok := LaunchCommandForTool("cursor"); !ok || c != "cursor" {
		t.Fatal(c)
	}
	if _, ok := LaunchCommandForTool("unknown"); ok {
		t.Fatal("unknown tool resolved")
	}
}

func TestMacOSKittyBundlePaths(t *testing.T) {
	if got := MacOSKittyBundlePaths(""); !reflect.DeepEqual(got, []string{"/Applications/kitty.app/Contents/MacOS/kitty"}) {
		t.Fatal(got)
	}
	if got := MacOSKittyBundlePaths("/Users/me"); len(got) != 3 || got[2] != "/Users/me/.local/kitty.app/bin/kitty" {
		t.Fatal(got)
	}
}

func TestEmulatorInstalledAgreesWithProbes(t *testing.T) {
	_, kitty := ResolveKitty()
	if EmulatorInstalled("kitty") != (runtime.GOOS != "windows" && kitty) {
		t.Fatal("kitty")
	}
	if EmulatorInstalled("ghostty") != CommandExists("ghostty") || EmulatorInstalled("nonexistent-terminal") {
		t.Fatal("probe mismatch")
	}
}

func TestResolveTerminalPath(t *testing.T) {
	root := "/home/user/my-project"
	if got, _ := ResolveTerminalPath(root, "."); filepath.ToSlash(got) != root {
		t.Fatal(got)
	}
	if got, _ := ResolveTerminalPath(root, "./src/server"); filepath.ToSlash(got) != root+"/src/server" {
		t.Fatal(got)
	}
	for _, bad := range []string{"../../outside", "/etc/passwd", "src/../../escape"} {
		_, err := ResolveTerminalPath(root, bad)
		if err == nil || !strings.Contains(err.Error(), "must stay inside the project root") {
			t.Errorf("%q accepted: %v", bad, err)
		}
	}
	if runtime.GOOS == "windows" {
		for _, bad := range []string{`\Users\me`, `C:Users\me`} {
			if _, err := ResolveTerminalPath(`C:\Users\me\p`, bad); err == nil {
				t.Errorf("%q accepted", bad)
			}
		}
	}
}

func TestResolveExistingTerminalPath(t *testing.T) {
	root := t.TempDir()
	os.Mkdir(filepath.Join(root, "src"), 0o755)
	os.WriteFile(filepath.Join(root, "notes.txt"), []byte("hi"), 0o644)
	if _, err := ResolveExistingTerminalPath(root, "src"); err != nil {
		t.Fatal(err)
	}
	if _, err := ResolveExistingTerminalPath(root, "absent"); err == nil || !strings.HasPrefix(err.Error(), "path not found:") {
		t.Fatal(err)
	}
	if _, err := ResolveExistingTerminalPath(root, "notes.txt"); err == nil || !strings.HasPrefix(err.Error(), "path is not a directory:") {
		t.Fatal(err)
	}
	if _, err := ResolveExistingTerminalPath(root, "/etc"); err == nil || !strings.Contains(err.Error(), "must stay inside") {
		t.Fatal(err)
	}
}

var ctx = PlaceholderContext{
	Root: "/home/user/project", Config: "/home/user/project/.quickdev.toml",
	Name: "myproj", Cwd: "/home/user/elsewhere",
}

func TestResolveAppArgs(t *testing.T) {
	got := ResolveAppArgs([]string{".", "{root}/src", "{config}", "--name={name}", "{cwd}", "plain", "{unknown}", "{open"}, ctx)
	want := []string{"/home/user/project", "/home/user/project/src", "/home/user/project/.quickdev.toml",
		"--name=myproj", "/home/user/elsewhere", "plain", "{unknown}", "{open"}
	if !reflect.DeepEqual(got, want) {
		t.Fatalf("%q", got)
	}
}

func TestResolveAppArgsDoesNotDoubleExpand(t *testing.T) {
	c := PlaceholderContext{Root: "/r", Config: "/r/.quickdev.toml", Name: "{cwd}", Cwd: "/the/cwd"}
	if got := ResolveAppArgs([]string{"{name}"}, c); got[0] != "{cwd}" {
		t.Fatal(got)
	}
}

func TestUnknownPlaceholders(t *testing.T) {
	if got := UnknownPlaceholders("{root}/{bogus}/{}/{name}{x}"); !reflect.DeepEqual(got, []string{"bogus", "x"}) {
		t.Fatal(got)
	}
}

func TestNormalizePath(t *testing.T) {
	home, _ := os.UserHomeDir()
	if got := NormalizePath("~/code"); got != home+"/code" {
		t.Fatal(got)
	}
	if NormalizePath("/abs/path") != "/abs/path" {
		t.Fatal("absolute path changed")
	}
}

func TestEditorArgs(t *testing.T) {
	if got := EditorArgs([]string{"/p", "--reuse-window"}, "/p"); len(got) != 2 {
		t.Fatal(got)
	}
	if got := EditorArgs(nil, "/root"); !reflect.DeepEqual(got, []string{"/root"}) {
		t.Fatal(got)
	}
	if got := EffectiveAppArgs("Cursor", "/Applications/Cursor.app", nil, "/project"); !reflect.DeepEqual(got, []string{"/project"}) {
		t.Fatal(got)
	}
	if got := EffectiveAppArgs("Firefox", "/usr/bin/firefox", nil, "/project"); got != nil {
		t.Fatal(got)
	}
}

func TestAppTargetResolvable(t *testing.T) {
	if runtime.GOOS != "windows" && !AppTargetResolvable("sh") {
		t.Fatal("sh is on PATH")
	}
	if AppTargetResolvable("/definitely/not/installed.app") {
		t.Fatal("missing path resolvable")
	}
}
