package tomledit

import (
	"strings"
	"testing"
)

func TestRoundTripIsVerbatim(t *testing.T) {
	src := "# header\n\nemulator = \"kitty\" # pinned\n\n[[projects]]\n# about a\nname = \"a\"\npath = \"/a\"\nextra = [\n  1,\n  2,\n]\n\n[future]\nx = \"\"\"\n[not a header]\n\"\"\"\n"
	if got := Parse(src).String(); got != src {
		t.Fatalf("got:\n%q\nwant:\n%q", got, src)
	}
}

func TestMultilineValuesAreOneEntry(t *testing.T) {
	d := Parse("[t]\nlist = [\n  \"a\",\n  \"b\",\n]\nafter = 1\n")
	s := d.Table("t")
	v, ok := s.Get("list")
	if !ok || len(v.([]any)) != 2 {
		t.Fatalf("%v", v)
	}
	s.Remove("list")
	if got := d.String(); got != "[t]\nafter = 1\n" {
		t.Fatalf("%q", got)
	}
}

func TestSetKeepsUnchangedValueFormatting(t *testing.T) {
	d := Parse("[t]\nname = 'x'   # note\n")
	d.Table("t").Set("name", "x")
	if !strings.Contains(d.String(), "name = 'x'   # note") {
		t.Fatal(d.String())
	}
	d.Table("t").Set("name", "y")
	if !strings.Contains(d.String(), `name = "y"`) {
		t.Fatal(d.String())
	}
}

func TestSetRootInsertsBeforeTables(t *testing.T) {
	d := Parse("# header\n\n[[projects]]\nname = \"a\"\n")
	d.SetRoot("emulator", "kitty")
	want := "# header\n\nemulator = \"kitty\"\n\n[[projects]]\nname = \"a\"\n"
	if got := d.String(); got != want {
		t.Fatalf("got:\n%q\nwant:\n%q", got, want)
	}
}

func TestReplaceArrayKeepsPositionAndOtherSections(t *testing.T) {
	d := Parse("[project]\nname = \"p\"\n\n[[terminals]]\nname = \"a\"\n\n[other]\nk = 1\n\n[[terminals]]\nname = \"b\"\n")
	old := d.ArrayTables("terminals")
	b := old[1].Clone()
	b.Set("path", ".")
	fresh := NewArrayTable("terminals")
	fresh.Set("name", "c")
	d.ReplaceArray("terminals", []*Section{b, fresh})
	want := "[project]\nname = \"p\"\n\n[[terminals]]\nname = \"b\"\npath = \".\"\n\n[[terminals]]\nname = \"c\"\n\n[other]\nk = 1\n"
	if got := d.String(); got != want {
		t.Fatalf("got:\n%q\nwant:\n%q", got, want)
	}
}

func TestSupports(t *testing.T) {
	tables, arrays := []string{"project"}, []string{"terminals"}
	if !Parse("[project]\nname = \"p\"\n[[terminals]]\nname = \"a\"\n").Supports(tables, arrays) {
		t.Fatal("plain layout rejected")
	}
	for _, src := range []string{
		"project.name = \"p\"\n",
		"project = { name = \"p\" }\n",
		"terminals = [{ name = \"a\" }]\n",
		"[project]\nname = \"p\"\n[[terminals]]\nname = \"a\"\n[terminals.env]\nX = \"1\"\n",
	} {
		if Parse(src).Supports(tables, arrays) {
			t.Errorf("accepted %q", src)
		}
	}
}

func TestEncodeString(t *testing.T) {
	cases := map[string]string{
		"plain":       `"plain"`,
		`C:\Users\me`: `'C:\Users\me'`,
		`say "hi"`:    `'say "hi"'`,
		`it's "x"`:    `"it's \"x\""`,
		"line\nbreak": `"line\nbreak"`,
	}
	for in, want := range cases {
		if got := EncodeString(in); got != want {
			t.Errorf("EncodeString(%q)=%s want %s", in, got, want)
		}
	}
}

func TestCRLFIsPreserved(t *testing.T) {
	d := Parse("[t]\r\nname = \"a\"\r\n")
	d.Table("t").Set("path", ".")
	if got := d.String(); got != "[t]\r\nname = \"a\"\r\npath = \".\"\r\n" {
		t.Fatalf("%q", got)
	}
}
