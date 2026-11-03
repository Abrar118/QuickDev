package apps

import (
	"encoding/binary"
	"errors"
	"os"
	"path/filepath"
	"strings"
	"unicode/utf16"

	"github.com/Abrar118/QuickDev/internal/models"
	"github.com/Abrar118/QuickDev/internal/shellwords"
)

// windowsStartMenuApps collects Start Menu .lnk shortcuts whose target is an
// .exe. Only real directories are descended into, so a junction cycle cannot
// recurse forever.
func windowsStartMenuApps() []models.AppEntry {
	var dirs []string
	for _, env := range []string{"ProgramData", "AppData"} {
		if base := os.Getenv(env); base != "" {
			dirs = append(dirs, filepath.Join(base, `Microsoft\Windows\Start Menu\Programs`))
		}
	}
	var apps []models.AppEntry
	var walk func(dir string)
	walk = func(dir string) {
		entries, err := os.ReadDir(dir)
		if err != nil {
			return
		}
		for _, e := range entries {
			path := filepath.Join(dir, e.Name())
			if e.Type().IsDir() {
				walk(path)
				continue
			}
			if !strings.HasSuffix(strings.ToLower(e.Name()), ".lnk") {
				continue
			}
			data, err := os.ReadFile(path)
			if err != nil {
				continue
			}
			target, args, err := ParseShellLink(data)
			if err != nil || !strings.HasSuffix(strings.ToLower(target), ".exe") {
				continue
			}
			app := models.AppEntry{
				Name: strings.TrimSuffix(e.Name(), filepath.Ext(e.Name())),
				Path: target,
			}
			// POSIX tokenization is not a faithful Windows command-line parser,
			// but is correct for common args; fall back to whitespace splitting
			// on unbalanced quotes.
			if args != "" {
				split, err := shellwords.Split(args)
				if err != nil {
					split = strings.Fields(args)
				}
				app.Args = split
			}
			apps = append(apps, app)
		}
	}
	for _, d := range dirs {
		walk(d)
	}
	return apps
}

// Shell link flags (MS-SHLLINK 2.1.1).
const (
	hasLinkTargetIDList = 1 << 0
	hasLinkInfo         = 1 << 1
	hasName             = 1 << 2
	hasRelativePath     = 1 << 3
	hasWorkingDir       = 1 << 4
	hasArguments        = 1 << 5
	isUnicode           = 1 << 7
)

var errBadLink = errors.New("malformed shell link")

// ParseShellLink extracts the local target path and the argument string from
// a Windows .lnk file.
func ParseShellLink(data []byte) (target, args string, err error) {
	le := binary.LittleEndian
	if len(data) < 0x4C || le.Uint32(data) != 0x4C {
		return "", "", errBadLink
	}
	flags := le.Uint32(data[0x14:])
	pos := 0x4C

	if flags&hasLinkTargetIDList != 0 {
		if pos+2 > len(data) {
			return "", "", errBadLink
		}
		pos += 2 + int(le.Uint16(data[pos:]))
	}

	if flags&hasLinkInfo != 0 {
		if pos+28 > len(data) {
			return "", "", errBadLink
		}
		info := data[pos:]
		size := int(le.Uint32(info))
		if size > len(info) {
			return "", "", errBadLink
		}
		info = info[:size]
		headerSize := le.Uint32(info[4:])
		infoFlags := le.Uint32(info[8:])
		if infoFlags&1 != 0 { // VolumeIDAndLocalBasePath
			if headerSize >= 0x24 && len(info) >= 0x24 {
				if off := int(le.Uint32(info[0x1C:])); off > 0 && off < len(info) {
					target = utf16z(info[off:])
				}
			}
			if target == "" {
				if off := int(le.Uint32(info[16:])); off < len(info) {
					target = ansiz(info[off:])
				}
			}
		}
		pos += size
	}

	unicode := flags&isUnicode != 0
	for _, flag := range []uint32{hasName, hasRelativePath, hasWorkingDir, hasArguments} {
		if flags&flag == 0 {
			continue
		}
		if pos+2 > len(data) {
			return "", "", errBadLink
		}
		count := int(le.Uint16(data[pos:]))
		pos += 2
		n := count
		if unicode {
			n *= 2
		}
		if pos+n > len(data) {
			return "", "", errBadLink
		}
		raw := data[pos : pos+n]
		pos += n
		if flag == hasArguments {
			if unicode {
				args = decodeUTF16(raw)
			} else {
				args = string(raw)
			}
		}
	}
	if target == "" {
		return "", "", errBadLink
	}
	return target, args, nil
}

func ansiz(b []byte) string {
	if i := strings.IndexByte(string(b), 0); i >= 0 {
		b = b[:i]
	}
	return string(b)
}

func utf16z(b []byte) string {
	var units []uint16
	for i := 0; i+1 < len(b); i += 2 {
		u := binary.LittleEndian.Uint16(b[i:])
		if u == 0 {
			break
		}
		units = append(units, u)
	}
	return string(utf16.Decode(units))
}

func decodeUTF16(b []byte) string {
	units := make([]uint16, 0, len(b)/2)
	for i := 0; i+1 < len(b); i += 2 {
		units = append(units, binary.LittleEndian.Uint16(b[i:]))
	}
	return string(utf16.Decode(units))
}
