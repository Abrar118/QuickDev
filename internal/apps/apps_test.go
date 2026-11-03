package apps

import (
	"encoding/binary"
	"reflect"
	"strings"
	"testing"
	"unicode/utf16"

	"github.com/Abrar118/QuickDev/internal/models"
)

func TestParseExec(t *testing.T) {
	cases := []struct {
		in   string
		path string
		args []string
	}{
		{"code %F", "code", nil},
		{"flatpak run com.example.App", "flatpak", []string{"run", "com.example.App"}},
		{"foo %% bar", "foo", []string{"%", "bar"}},
		{"myapp --flag %U", "myapp", []string{"--flag"}},
		{`env "VAR=a b" /snap/bin/app`, "env", []string{"VAR=a b", "/snap/bin/app"}},
		{"app http://x/%foo", "app", []string{"http://x/%foo"}},
	}
	for _, c := range cases {
		path, args := ParseExec(c.in)
		if path != c.path || !reflect.DeepEqual(args, c.args) {
			t.Errorf("ParseExec(%q) = %q %q", c.in, path, args)
		}
	}
}

func TestParseDesktopEntry(t *testing.T) {
	yes := func(string) bool { return true }
	e, ok := ParseDesktopEntry("[Desktop Entry]\nType=Application\nName=Cursor\nExec=cursor %F\n", yes)
	if !ok || e.Name != "Cursor" || e.Path != "cursor" || e.Args != nil {
		t.Fatalf("%+v", e)
	}
	e, ok = ParseDesktopEntry("[Desktop Entry]\nType=Application\nName=App\nExec=flatpak run com.example.App %U\n", yes)
	if !ok || e.Path != "flatpak" || !reflect.DeepEqual(e.Args, []string{"run", "com.example.App"}) {
		t.Fatalf("%+v", e)
	}
	e, ok = ParseDesktopEntry("[Desktop Entry]\nType=Application\nName=Main\nExec=main\n[Desktop Action New]\nName=New\nExec=other\n", yes)
	if !ok || e.Name != "Main" || e.Path != "main" {
		t.Fatalf("%+v", e)
	}
	skipped := []string{
		"[Desktop Entry]\nType=Application\nName=X\nExec=x\nNoDisplay=true\n",
		"[Desktop Entry]\nType=Application\nName=X\nExec=x\nHidden=true\n",
		"[Desktop Entry]\nType=Directory\nName=X\nExec=x\n",
		"[Desktop Entry]\nType=Application\nExec=x\n",
		"[Desktop Entry]\nType=Application\nName=X\n",
	}
	for _, c := range skipped {
		if _, ok := ParseDesktopEntry(c, yes); ok {
			t.Errorf("accepted %q", c)
		}
	}
	if _, ok := ParseDesktopEntry("[Desktop Entry]\nType=Application\nName=X\nExec=x\nTryExec=/nope/x\n", func(string) bool { return false }); ok {
		t.Error("unresolvable TryExec accepted")
	}
}

func TestCombineAppArgs(t *testing.T) {
	if CombineAppArgs(nil, nil) != nil {
		t.Fatal("empty must be nil")
	}
	if got := CombineAppArgs([]string{"run", "app"}, []string{"."}); !reflect.DeepEqual(got, []string{"run", "app", "."}) {
		t.Fatal(got)
	}
}

func installed() []NamePath {
	return []NamePath{
		{"Cursor", "/Applications/Cursor.app"},
		{"Zed", "/Applications/Zed.app"},
		{"arc", "/Applications/Arc.app"},
	}
}

func names(apps []models.AppEntry) string {
	var n []string
	for _, a := range apps {
		n = append(n, a.Name)
	}
	return strings.Join(n, ",")
}

func TestDetectedToApps(t *testing.T) {
	got := DetectedToApps([]string{"/Applications/Zed.app/", "/Applications/Cursor.app", "/Applications/Zed.app", "/System/Finder.app", "/Applications/Arc.app"}, installed())
	if names(got) != "arc,Cursor,Zed" {
		t.Fatal(names(got))
	}
	if got[2].Path != "/Applications/Zed.app" {
		t.Fatal("installed path must be used")
	}
	if len(DetectedToApps(nil, installed())) != 0 {
		t.Fatal("empty running must be empty")
	}
	if NormalizeBundlePath("/a.app/") != "/a.app" || NormalizeBundlePath("/a.app") != "/a.app" {
		t.Fatal("normalize")
	}
}

func TestMergeApps(t *testing.T) {
	existing := []models.AppEntry{{Name: "Renamed", Path: "/Applications/Cursor.app/"}}
	captured := []models.AppEntry{{Name: "Zed", Path: "/Applications/Zed.app"}, {Name: "Cursor", Path: "/Applications/Cursor.app"}, {Name: "Arc", Path: "/Applications/Arc.app"}}
	if got := MergeApps(existing, captured); names(got) != "Zed,Arc" {
		t.Fatal(names(got))
	}
}

func TestDiscoverAppsIsSortedAndUnique(t *testing.T) {
	apps := DiscoverApps()
	for i := 1; i < len(apps); i++ {
		if strings.ToLower(apps[i-1].Name) > strings.ToLower(apps[i].Name) {
			t.Fatal("not sorted")
		}
	}
	seen := map[string]bool{}
	for _, a := range DiscoverAppsUniqueByPath() {
		if seen[a.Path] {
			t.Fatalf("duplicate %s", a.Path)
		}
		seen[a.Path] = true
	}
}

// buildLink assembles a minimal Unicode .lnk with LinkInfo and arguments.
func buildLink(target, args string) []byte {
	le := binary.LittleEndian
	header := make([]byte, 0x4C)
	le.PutUint32(header, 0x4C)
	le.PutUint32(header[0x14:], hasLinkInfo|hasArguments|isUnicode)

	info := make([]byte, 0x1C)
	info = append(info, []byte(target)...)
	info = append(info, 0)
	le.PutUint32(info[0:], uint32(len(info)))
	le.PutUint32(info[4:], 0x1C)
	le.PutUint32(info[8:], 1)
	le.PutUint32(info[16:], 0x1C)

	units := utf16.Encode([]rune(args))
	str := make([]byte, 2+2*len(units))
	le.PutUint16(str, uint16(len(units)))
	for i, u := range units {
		le.PutUint16(str[2+2*i:], u)
	}
	return append(append(header, info...), str...)
}

func TestParseShellLink(t *testing.T) {
	target, args, err := ParseShellLink(buildLink(`C:\Apps\Discord\Update.exe`, `--processStart "Discord.exe"`))
	if err != nil || target != `C:\Apps\Discord\Update.exe` || args != `--processStart "Discord.exe"` {
		t.Fatalf("%q %q %v", target, args, err)
	}
	if _, _, err := ParseShellLink([]byte("not a link")); err == nil {
		t.Fatal("garbage accepted")
	}
}
