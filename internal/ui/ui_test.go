package ui

import (
	"os"
	"path/filepath"
	"testing"
)

func TestPadIgnoresANSICodes(t *testing.T) {
	if got := Strip(Pad(Bold, "api", 6)); got != "api   " {
		t.Fatalf("%q", got)
	}
}

func TestTilde(t *testing.T) {
	home, err := os.UserHomeDir()
	if err != nil {
		t.Skip("no home")
	}
	sep := string(filepath.Separator)
	if Tilde(home) != "~" || Tilde(filepath.Join(home, "code")) != "~"+sep+"code" || Tilde("/elsewhere/code") != "/elsewhere/code" {
		t.Fatal("tilde wrong")
	}
}

func TestCount(t *testing.T) {
	if Count(1, "item") != "1 item" || Count(0, "item") != "0 items" {
		t.Fatal("pluralization wrong")
	}
}

func TestClosest(t *testing.T) {
	names := []string{"my-api", "web", "inventory"}
	if s, ok := Closest("my-ap", names); !ok || s != "my-api" {
		t.Fatal(s)
	}
	if s, ok := Closest("invetory", names); !ok || s != "inventory" {
		t.Fatal(s)
	}
	if _, ok := Closest("zzz", names); ok {
		t.Fatal("unexpected suggestion")
	}
}

func TestJaroMatchesReferenceValues(t *testing.T) {
	// Reference values from strsim::jaro.
	cases := []struct {
		a, b string
		want float64
	}{
		{"", "", 1}, {"foo", "", 0}, {"martha", "marhta", 0.944}, {"dwayne", "duane", 0.822},
	}
	for _, c := range cases {
		if got := Jaro(c.a, c.b); got < c.want-0.001 || got > c.want+0.001 {
			t.Errorf("Jaro(%q,%q)=%f want %f", c.a, c.b, got, c.want)
		}
	}
}

func TestAgo(t *testing.T) {
	cases := map[[2]int64]string{
		{1000, 1030}:             "just now",
		{1000, 1000 + 5*60 + 59}: "5m ago",
		{1000, 1000 + 3*3600}:    "3h ago",
		{1000, 1000 + 2*86400}:   "2d ago",
		{2000, 1000}:             "just now",
	}
	for in, want := range cases {
		if got := Ago(in[0], in[1]); got != want {
			t.Errorf("Ago(%v)=%q want %q", in, got, want)
		}
	}
}
