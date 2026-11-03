package shellwords

import (
	"reflect"
	"strings"
	"testing"
)

func TestSplit(t *testing.T) {
	cases := map[string][]string{
		`--profile "Dev User"`:   {"--profile", "Dev User"},
		"--flag value":           {"--flag", "value"},
		`a 'b c' "d\"e" f\ g`:    {"a", "b c", `d"e`, "f g"},
		`''`:                     {""},
		`"a\nb"`:                 {`a\nb`},
		"cmd # trailing comment": {"cmd"},
		"a#b":                    {"a#b"},
		"  spaced   out  ":       {"spaced", "out"},
	}
	for input, want := range cases {
		got, err := Split(input)
		if err != nil || !reflect.DeepEqual(got, want) {
			t.Errorf("Split(%q) = %q, %v; want %q", input, got, err, want)
		}
	}
}

func TestParseArgsErrorsOnUnbalancedQuote(t *testing.T) {
	_, err := ParseArgs(`--name "unterminated`)
	if err == nil || !strings.Contains(err.Error(), "invalid") {
		t.Fatalf("got %v", err)
	}
}
