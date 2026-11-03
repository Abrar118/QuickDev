package fzf

import (
	"errors"
	"reflect"
	"strings"
	"testing"
)

func TestInstallHint(t *testing.T) {
	if h := InstallHint(); !strings.Contains(h, "fzf") || !strings.Contains(h, "install") {
		t.Fatal(h)
	}
}

func TestIsCancellation(t *testing.T) {
	if !IsCancellation(ErrCancelled) || IsCancellation(errors.New("cancelled")) {
		t.Fatal("cancellation must match only the sentinel")
	}
}

func TestIndexedSelectionRecoversPositions(t *testing.T) {
	got, err := ParseIndexedSelection([]string{"2\t[app] a — b", "0\t[terminal] x — y"}, 3)
	if err != nil || !reflect.DeepEqual(got, []int{2, 0}) {
		t.Fatalf("%v %v", got, err)
	}
}

func TestIndexedSelectionRejectsUnmappableRows(t *testing.T) {
	for _, row := range []string{"9\tout of range", "no index", "x\tbad"} {
		if _, err := ParseIndexedSelection([]string{row}, 3); err == nil {
			t.Errorf("accepted %q", row)
		}
	}
}

func TestSanitizeRow(t *testing.T) {
	if got := SanitizeRow("api\nlaunch\tevil\x1b[31m"); got != "apilaunchevil[31m" {
		t.Fatalf("%q", got)
	}
}
