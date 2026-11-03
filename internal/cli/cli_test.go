package cli

import (
	"bytes"
	"os"
	"os/exec"
	"path/filepath"
	"reflect"
	"strings"
	"testing"
)

// The test binary re-executes itself as quickdev when this variable is set, so
// these tests drive the real command line, exit codes and all.
const asCLI = "QUICKDEV_TEST_RUN_CLI"

func TestMain(m *testing.M) {
	if os.Getenv(asCLI) == "1" {
		os.Exit(Main(os.Args[1:]))
	}
	os.Exit(m.Run())
}

type result struct {
	stdout, stderr string
	code           int
}

// quickdev runs the CLI in dir with an isolated global config.
func quickdev(t *testing.T, dir string, env []string, args ...string) result {
	t.Helper()
	cmd := exec.Command(os.Args[0], args...)
	cmd.Dir = dir
	cmd.Env = append(os.Environ(), asCLI+"=1", "QUICKDEV_CONFIG="+filepath.Join(dir, "global.toml"), "NO_COLOR=", "CLICOLOR_FORCE=")
	cmd.Env = append(cmd.Env, env...)
	var stdout, stderr bytes.Buffer
	cmd.Stdout, cmd.Stderr = &stdout, &stderr
	err := cmd.Run()
	code := 0
	if exitErr, ok := err.(*exec.ExitError); ok {
		code = exitErr.ExitCode()
	} else if err != nil {
		t.Fatal(err)
	}
	return result{stdout.String(), stderr.String(), code}
}

func TestVersion(t *testing.T) {
	dir := t.TempDir()
	long := quickdev(t, dir, nil, "--version")
	short := quickdev(t, dir, nil, "-V")
	if long.code != 0 || strings.TrimSpace(long.stdout) != "quickdev "+Version || short.stdout != long.stdout {
		t.Fatalf("%+v %+v", long, short)
	}
	if bare := quickdev(t, dir, nil); bare.code != 2 {
		t.Fatalf("bare invocation must fail with a usage error: %+v", bare)
	}
}

func TestValidateJSON(t *testing.T) {
	dir := t.TempDir()
	missing := quickdev(t, dir, nil, "validate", "--json")
	if missing.code == 0 || !strings.Contains(missing.stdout, `"path": null`) ||
		!strings.Contains(missing.stdout, `"valid": false`) || !strings.Contains(missing.stdout, "no .quickdev.toml found") {
		t.Fatalf("%+v", missing)
	}
	os.WriteFile(filepath.Join(dir, ".quickdev.toml"), []byte("[project\n"), 0o644)
	broken := quickdev(t, dir, nil, "validate", "--json")
	if broken.code == 0 || !strings.Contains(broken.stdout, `"valid": false`) || !strings.Contains(broken.stdout, "failed to parse project config") {
		t.Fatalf("%+v", broken)
	}
}

func TestColorFlagBeatsEnvironment(t *testing.T) {
	dir := t.TempDir()
	esc := func(r result) bool { return strings.Contains(r.stdout, "\x1b") }
	if esc(quickdev(t, dir, []string{"CLICOLOR_FORCE=1"}, "--color", "never", "prune")) {
		t.Error("--color never must win over CLICOLOR_FORCE")
	}
	if !esc(quickdev(t, dir, []string{"NO_COLOR=1"}, "--color", "always", "prune")) {
		t.Error("--color always must win over NO_COLOR")
	}
	if !esc(quickdev(t, dir, []string{"CLICOLOR_FORCE=1"}, "prune")) {
		t.Error("without the flag, CLICOLOR_FORCE applies")
	}
	if esc(quickdev(t, dir, nil, "prune")) {
		t.Error("piped output must be plain")
	}
}

func TestShellCompletionOffersProjects(t *testing.T) {
	dir := t.TempDir()
	os.WriteFile(filepath.Join(dir, "global.toml"), []byte("[[projects]]\nname = \"my-api\"\npath = \"/p/my-api\"\n"), 0o644)
	r := quickdev(t, dir, nil, "__complete", "launch", "my")
	if r.code != 0 || !strings.Contains(r.stdout, "my-api\t/p/my-api") {
		t.Fatalf("%+v", r)
	}
	script := quickdev(t, dir, nil, "completions", "zsh")
	if script.code != 0 || !strings.Contains(script.stdout, "compdef _quickdev quickdev") {
		t.Fatalf("%+v", script)
	}
	if bad := quickdev(t, dir, nil, "completions", "tcsh"); bad.code != 2 {
		t.Fatalf("%+v", bad)
	}
}

func TestInitAddRemoveFlow(t *testing.T) {
	dir := t.TempDir()
	project := filepath.Join(dir, "proj")
	os.MkdirAll(filepath.Join(project, "api"), 0o755)

	if r := quickdev(t, project, []string{"QUICKDEV_CONFIG=" + filepath.Join(dir, "global.toml")}, "init"); r.code != 0 {
		t.Fatalf("%+v", r)
	}
	env := []string{"QUICKDEV_CONFIG=" + filepath.Join(dir, "global.toml")}
	if r := quickdev(t, project, env, "add", "terminal", "api", "./api", "--command", "npm run dev"); r.code != 0 {
		t.Fatalf("%+v", r)
	}
	if r := quickdev(t, project, env, "add", "app", "Zed", "zed", "--args", "{root}", "-n"); r.code != 2 {
		t.Fatalf("unknown flag must be a usage error: %+v", r)
	}
	if r := quickdev(t, project, env, "add", "app", "Zed", "zed", "--args", "{root}", "."); r.code != 0 {
		t.Fatalf("%+v", r)
	}
	if r := quickdev(t, project, env, "add", "terminal", "api", "."); r.code != 1 || !strings.Contains(r.stderr, "already exists") {
		t.Fatalf("%+v", r)
	}
	content, _ := os.ReadFile(filepath.Join(project, ".quickdev.toml"))
	for _, want := range []string{`command = "npm run dev"`, `args = ["{root}", "."]`} {
		if !strings.Contains(string(content), want) {
			t.Fatalf("missing %s:\n%s", want, content)
		}
	}

	dry := quickdev(t, project, env, "launch", "--all", "--dry-run")
	if dry.code != 0 || !strings.Contains(dry.stdout, "Dry run — would launch") || !strings.Contains(dry.stdout, filepath.Join(project, "api")+" · npm run dev") {
		t.Fatalf("%+v", dry)
	}

	typo := quickdev(t, project, env, "remove", "terminal", "ap")
	if typo.code != 1 || !strings.Contains(typo.stderr, "did you mean 'api'?") {
		t.Fatalf("%+v", typo)
	}
	if r := quickdev(t, project, env, "remove", "terminal", "api"); r.code != 0 {
		t.Fatalf("%+v", r)
	}
	if r := quickdev(t, project, env, "deregister", "--delete"); r.code != 0 {
		t.Fatalf("%+v", r)
	}
	if _, err := os.Stat(filepath.Join(project, ".quickdev.toml")); err == nil {
		t.Fatal("config not deleted")
	}
	list := quickdev(t, project, env, "list", "--json")
	if strings.TrimSpace(list.stdout) != "[]" {
		t.Fatalf("%+v", list)
	}
}

func TestExpandMultiValueFlags(t *testing.T) {
	cases := []struct{ in, want []string }{
		{[]string{"add", "app", "A", "/a", "--args", ".", "x", "--other"}, []string{"add", "app", "A", "/a", "--args=.", "--args=x", "--other"}},
		{[]string{"--color", "never", "add", "app", "A", "/a", "--args", "-"}, []string{"--color", "never", "add", "app", "A", "/a", "--args=-"}},
		{[]string{"add", "terminal", "t", ".", "--args", "x"}, []string{"add", "terminal", "t", ".", "--args", "x"}},
		{[]string{"add", "app", "A", "/a", "--", "--args", "x"}, []string{"add", "app", "A", "/a", "--", "--args", "x"}},
	}
	for _, c := range cases {
		if got := expandMultiValueFlags(c.in); !reflect.DeepEqual(got, c.want) {
			t.Errorf("%q -> %q, want %q", c.in, got, c.want)
		}
	}
}
