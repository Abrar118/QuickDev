// Package tomledit edits a TOML document in place, keeping its comments,
// formatting, and any keys the caller does not touch.
//
// It is deliberately small: it understands the document as a root region of
// key/value lines followed by `[table]` and `[[array]]` sections, which is the
// shape QuickDev's own files have. Callers check Supports before editing and
// regenerate the file instead when it uses a form this package does not model
// (dotted keys or inline tables for a managed table, for example).
package tomledit

import (
	"fmt"
	"reflect"
	"strconv"
	"strings"

	"github.com/pelletier/go-toml/v2"
)

// item is one logical entry: a comment/blank line (key == "") or a key/value
// pair, which may span several lines.
type item struct {
	lines  []string
	key    string // dotted key, segments joined with "."
	keyRaw string // source text before '=', right-trimmed (keeps indentation)
}

func (it *item) isTrivia() bool { return it.key == "" }

func (it *item) isBlank() bool {
	return it.isTrivia() && strings.TrimSpace(strings.Join(it.lines, "")) == ""
}

// Section is a `[name]` or `[[name]]` header with the comment lines directly
// above it and the entries below it.
type Section struct {
	decor  []string
	header string
	name   string
	dotted bool // header name has more than one segment
	array  bool
	body   []*item
}

// Doc is a parsed document.
type Doc struct {
	root     []*item
	sections []*Section
	crlf     bool
	broken   bool
}

// Value is something Set can write: string, bool, int64, or []string.
type Value any

// Parse splits content into editable pieces. content must already be valid
// TOML; Parse does not report syntax errors beyond what it needs to split.
func Parse(content string) *Doc {
	d := &Doc{crlf: strings.Contains(content, "\r\n")}
	content = strings.ReplaceAll(content, "\r\n", "\n")
	content = strings.TrimSuffix(content, "\n")
	var lines []string
	if content != "" {
		lines = strings.Split(content, "\n")
	}

	var current *Section
	body := func() *[]*item {
		if current == nil {
			return &d.root
		}
		return &current.body
	}

	for i := 0; i < len(lines); i++ {
		line := lines[i]
		trimmed := strings.TrimSpace(line)
		switch {
		case trimmed == "" || strings.HasPrefix(trimmed, "#"):
			*body() = append(*body(), &item{lines: []string{line}})
		case strings.HasPrefix(trimmed, "["):
			sec := parseHeader(line)
			if sec == nil {
				d.broken = true
				*body() = append(*body(), &item{lines: []string{line}})
				continue
			}
			// Comment lines directly above the header belong to it.
			items := body()
			cut := len(*items)
			for cut > 0 && (*items)[cut-1].isTrivia() && !(*items)[cut-1].isBlank() {
				cut--
			}
			for _, it := range (*items)[cut:] {
				sec.decor = append(sec.decor, it.lines...)
			}
			*items = (*items)[:cut]
			d.sections = append(d.sections, sec)
			current = sec
		default:
			end := i
			for ; end < len(lines); end++ {
				if parses(strings.Join(lines[i:end+1], "\n")) {
					break
				}
			}
			if end == len(lines) {
				d.broken = true
				end = len(lines) - 1
			}
			key, keyRaw := parseKeyLine(line)
			if key == "" {
				d.broken = true
				key = "\x00"
			}
			*body() = append(*body(), &item{lines: lines[i : end+1], key: key, keyRaw: keyRaw})
			i = end
		}
	}
	return d
}

func parses(s string) bool {
	var m map[string]any
	return toml.Unmarshal([]byte(s), &m) == nil
}

// parseHeader reads `[a.b]` or `[[a.b]]` with an optional trailing comment.
func parseHeader(line string) *Section {
	t := strings.TrimSpace(line)
	array := strings.HasPrefix(t, "[[")
	open := 1
	if array {
		open = 2
	}
	segments, rest, ok := parseKey(t[open:])
	if !ok {
		return nil
	}
	rest = strings.TrimLeft(rest, " \t")
	closing := "]"
	if array {
		closing = "]]"
	}
	if !strings.HasPrefix(rest, closing) {
		return nil
	}
	return &Section{
		header: line,
		name:   strings.Join(segments, "."),
		dotted: len(segments) > 1,
		array:  array,
	}
}

// parseKeyLine returns the dotted key of a `key = value` line and the source
// text before the '='.
func parseKeyLine(line string) (key, raw string) {
	segments, rest, ok := parseKey(line)
	if !ok {
		return "", ""
	}
	rest = strings.TrimLeft(rest, " \t")
	if !strings.HasPrefix(rest, "=") {
		return "", ""
	}
	raw = strings.TrimRight(line[:len(line)-len(rest)], " \t")
	return strings.Join(segments, "."), raw
}

// parseKey reads a (possibly dotted, possibly quoted) key from the start of s
// and returns its segments and the unconsumed remainder.
func parseKey(s string) ([]string, string, bool) {
	var segments []string
	for {
		s = strings.TrimLeft(s, " \t")
		if s == "" {
			return nil, "", false
		}
		var seg string
		switch s[0] {
		case '"':
			end := 1
			for end < len(s) && s[end] != '"' {
				if s[end] == '\\' {
					end++
				}
				end++
			}
			if end >= len(s) {
				return nil, "", false
			}
			unq, err := strconv.Unquote(s[:end+1])
			if err != nil {
				unq = s[1:end]
			}
			seg, s = unq, s[end+1:]
		case '\'':
			end := strings.IndexByte(s[1:], '\'')
			if end < 0 {
				return nil, "", false
			}
			seg, s = s[1:end+1], s[end+2:]
		default:
			end := 0
			for end < len(s) && isBareKeyChar(s[end]) {
				end++
			}
			if end == 0 {
				return nil, "", false
			}
			seg, s = s[:end], s[end:]
		}
		segments = append(segments, seg)
		trimmed := strings.TrimLeft(s, " \t")
		if strings.HasPrefix(trimmed, ".") {
			s = trimmed[1:]
			continue
		}
		return segments, s, true
	}
}

func isBareKeyChar(c byte) bool {
	return c >= 'a' && c <= 'z' || c >= 'A' && c <= 'Z' || c >= '0' && c <= '9' || c == '_' || c == '-'
}

// Supports reports whether this package can safely edit the given managed
// tables and arrays: each must appear only as plain `[table]` / `[[array]]`
// sections, never through dotted keys, inline tables, or sub-tables.
func (d *Doc) Supports(tables, arrays []string) bool {
	if d.broken {
		return false
	}
	managed := map[string]bool{}
	for _, n := range append(append([]string{}, tables...), arrays...) {
		managed[n] = true
	}
	for _, it := range d.root {
		if !it.isTrivia() && managed[strings.SplitN(it.key, ".", 2)[0]] {
			return false
		}
	}
	isArray := map[string]bool{}
	for _, n := range arrays {
		isArray[n] = true
	}
	seen := map[string]int{}
	for _, s := range d.sections {
		first := strings.SplitN(s.name, ".", 2)[0]
		if !managed[first] {
			continue
		}
		if s.dotted || s.array != isArray[s.name] {
			return false
		}
		seen[s.name]++
	}
	for _, t := range tables {
		if seen[t] > 1 {
			return false
		}
	}
	return true
}

// String renders the document. Sections are separated by exactly one blank
// line; everything inside a section is reproduced as written.
func (d *Doc) String() string {
	var chunks [][]string
	if root := renderItems(d.root); len(root) > 0 {
		chunks = append(chunks, root)
	}
	for _, s := range d.sections {
		chunk := append(append([]string{}, s.decor...), s.header)
		chunk = append(chunk, renderItems(s.body)...)
		chunks = append(chunks, chunk)
	}
	var out []string
	for i, c := range chunks {
		if i > 0 {
			out = append(out, "")
		}
		out = append(out, c...)
	}
	nl := "\n"
	if d.crlf {
		nl = "\r\n"
	}
	if len(out) == 0 {
		return ""
	}
	return strings.Join(out, nl) + nl
}

func renderItems(items []*item) []string {
	end := len(items)
	for end > 0 && items[end-1].isBlank() {
		end--
	}
	var out []string
	for _, it := range items[:end] {
		out = append(out, it.lines...)
	}
	return out
}

// SetRoot sets a top-level key, adding it after the last top-level key when
// absent so it never lands inside a table.
func (d *Doc) SetRoot(key string, v Value) { setIn(&d.root, key, v) }

// RemoveRoot removes a top-level key. Comments above it stay.
func (d *Doc) RemoveRoot(key string) { removeFrom(&d.root, key) }

// Table returns the `[name]` section, creating it at the end when missing.
func (d *Doc) Table(name string) *Section {
	for _, s := range d.sections {
		if !s.array && s.name == name {
			return s
		}
	}
	s := newSection(name, false)
	d.sections = append(d.sections, s)
	return s
}

// ArrayTables returns the `[[name]]` sections in document order.
func (d *Doc) ArrayTables(name string) []*Section {
	var out []*Section
	for _, s := range d.sections {
		if s.array && s.name == name {
			out = append(out, s)
		}
	}
	return out
}

// ReplaceArray swaps every `[[name]]` section for tables, placing them where
// the first old one was (or at the end when there were none).
func (d *Doc) ReplaceArray(name string, tables []*Section) {
	at := -1
	var kept []*Section
	for _, s := range d.sections {
		if s.array && s.name == name {
			if at < 0 {
				at = len(kept)
			}
			continue
		}
		kept = append(kept, s)
	}
	if at < 0 {
		at = len(kept)
	}
	out := append(append(append([]*Section{}, kept[:at]...), tables...), kept[at:]...)
	d.sections = out
}

// NewArrayTable makes an empty `[[name]]` section.
func NewArrayTable(name string) *Section { return newSection(name, true) }

func newSection(name string, array bool) *Section {
	header := "[" + name + "]"
	if array {
		header = "[[" + name + "]]"
	}
	return &Section{header: header, name: name, array: array}
}

// Get returns the decoded value of key in the section.
func (s *Section) Get(key string) (any, bool) {
	for _, it := range s.body {
		if it.key == key {
			return decodeItem(it)
		}
	}
	return nil, false
}

// GetString returns key's value when it is a string.
func (s *Section) GetString(key string) (string, bool) {
	v, ok := s.Get(key)
	if !ok {
		return "", false
	}
	str, ok := v.(string)
	return str, ok
}

// Set sets key, adding it after the section's last key when absent.
func (s *Section) Set(key string, v Value) { setIn(&s.body, key, v) }

// Remove deletes key from the section.
func (s *Section) Remove(key string) { removeFrom(&s.body, key) }

func setIn(items *[]*item, key string, v Value) {
	encoded := Encode(v)
	for _, it := range *items {
		if it.key != key {
			continue
		}
		if old, ok := decodeItem(it); ok && sameValue(old, v) {
			return // unchanged: keep the user's formatting and trailing comment
		}
		it.lines = []string{it.keyRaw + " = " + encoded}
		return
	}
	newItem := &item{lines: []string{encodeKey(key) + " = " + encoded}, key: key, keyRaw: encodeKey(key)}
	at := len(*items)
	for i := len(*items) - 1; i >= 0; i-- {
		if !(*items)[i].isTrivia() {
			at = i + 1
			break
		}
	}
	*items = append((*items)[:at], append([]*item{newItem}, (*items)[at:]...)...)
}

func removeFrom(items *[]*item, key string) {
	out := (*items)[:0]
	for _, it := range *items {
		if it.key != key {
			out = append(out, it)
		}
	}
	*items = out
}

func decodeItem(it *item) (any, bool) {
	var m map[string]any
	if err := toml.Unmarshal([]byte(strings.Join(it.lines, "\n")), &m); err != nil {
		return nil, false
	}
	var cur any = m
	for _, seg := range strings.Split(it.key, ".") {
		mm, ok := cur.(map[string]any)
		if !ok {
			return nil, false
		}
		cur, ok = mm[seg]
		if !ok {
			return nil, false
		}
	}
	return cur, true
}

func sameValue(old any, v Value) bool {
	switch want := v.(type) {
	case []string:
		list, ok := old.([]any)
		if !ok || len(list) != len(want) {
			return false
		}
		for i, x := range list {
			if s, ok := x.(string); !ok || s != want[i] {
				return false
			}
		}
		return true
	case int:
		return reflect.DeepEqual(old, int64(want))
	default:
		return reflect.DeepEqual(old, v)
	}
}

func encodeKey(key string) string {
	for i := 0; i < len(key); i++ {
		if !isBareKeyChar(key[i]) {
			return EncodeString(key)
		}
	}
	return key
}

// Encode renders v as a TOML value.
func Encode(v Value) string {
	switch x := v.(type) {
	case string:
		return EncodeString(x)
	case bool:
		return strconv.FormatBool(x)
	case int:
		return strconv.Itoa(x)
	case int64:
		return strconv.FormatInt(x, 10)
	case []string:
		parts := make([]string, len(x))
		for i, s := range x {
			parts[i] = EncodeString(s)
		}
		return "[" + strings.Join(parts, ", ") + "]"
	}
	panic(fmt.Sprintf("tomledit: unsupported value %T", v))
}

// EncodeString renders s as a TOML string: a literal 'string' when that avoids
// escaping (Windows paths, embedded quotes), otherwise a basic "string".
func EncodeString(s string) string {
	hasControl := false
	for _, r := range s {
		if r < 0x20 && r != '\t' || r == 0x7f {
			hasControl = true
		}
	}
	if !hasControl && strings.ContainsAny(s, `\"`) && !strings.ContainsAny(s, "'\t") {
		return "'" + s + "'"
	}
	var b strings.Builder
	b.WriteByte('"')
	for _, r := range s {
		switch r {
		case '"':
			b.WriteString(`\"`)
		case '\\':
			b.WriteString(`\\`)
		case '\b':
			b.WriteString(`\b`)
		case '\t':
			b.WriteString(`\t`)
		case '\n':
			b.WriteString(`\n`)
		case '\f':
			b.WriteString(`\f`)
		case '\r':
			b.WriteString(`\r`)
		default:
			if r < 0x20 || r == 0x7f {
				fmt.Fprintf(&b, `\u%04X`, r)
			} else {
				b.WriteRune(r)
			}
		}
	}
	b.WriteByte('"')
	return b.String()
}

// LeadingComments is the run of comment and blank lines at the top of content,
// used to keep a file's header when it has to be regenerated.
func LeadingComments(content string) string {
	var out []string
	for _, line := range strings.Split(strings.ReplaceAll(content, "\r\n", "\n"), "\n") {
		t := strings.TrimSpace(line)
		if t != "" && !strings.HasPrefix(t, "#") {
			break
		}
		out = append(out, line)
	}
	for len(out) > 0 && strings.TrimSpace(out[len(out)-1]) == "" {
		out = out[:len(out)-1]
	}
	if len(out) == 0 {
		return ""
	}
	return strings.Join(out, "\n") + "\n"
}

// Clone returns a deep copy of the section.
func (s *Section) Clone() *Section {
	c := *s
	c.decor = append([]string(nil), s.decor...)
	c.body = make([]*item, len(s.body))
	for i, it := range s.body {
		copied := *it
		copied.lines = append([]string(nil), it.lines...)
		c.body[i] = &copied
	}
	return &c
}
