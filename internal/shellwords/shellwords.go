// Package shellwords splits strings into arguments with POSIX shell quoting.
package shellwords

import (
	"errors"
	"strings"
)

// ErrMismatchedQuotes is returned for an unterminated quote or a trailing
// backslash.
var ErrMismatchedQuotes = errors.New("missing closing quote")

// Split splits input into words the way a POSIX shell would, without
// expansions: single quotes are literal, double quotes allow \" \\ \$ \` and
// line continuations, a backslash outside quotes escapes the next character,
// and an unquoted # at the start of a word starts a comment.
func Split(input string) ([]string, error) {
	const (
		delimiter = iota
		unquoted
		unquotedBackslash
		singleQuoted
		doubleQuoted
		doubleQuotedBackslash
		comment
	)

	var words []string
	var word strings.Builder
	state := delimiter
	for _, c := range input {
		switch state {
		case delimiter:
			switch c {
			case '\'':
				state = singleQuoted
			case '"':
				state = doubleQuoted
			case '\\':
				state = unquotedBackslash
			case '\t', ' ', '\n':
			case '#':
				state = comment
			default:
				word.WriteRune(c)
				state = unquoted
			}
		case unquoted:
			switch c {
			case '\'':
				state = singleQuoted
			case '"':
				state = doubleQuoted
			case '\\':
				state = unquotedBackslash
			case '\t', ' ', '\n':
				words = append(words, word.String())
				word.Reset()
				state = delimiter
			default:
				word.WriteRune(c)
			}
		case unquotedBackslash:
			if c != '\n' {
				word.WriteRune(c)
			}
			state = unquoted
		case singleQuoted:
			if c == '\'' {
				state = unquoted
			} else {
				word.WriteRune(c)
			}
		case doubleQuoted:
			switch c {
			case '"':
				state = unquoted
			case '\\':
				state = doubleQuotedBackslash
			default:
				word.WriteRune(c)
			}
		case doubleQuotedBackslash:
			switch c {
			case '\n':
			case '$', '`', '"', '\\':
				word.WriteRune(c)
			default:
				word.WriteRune('\\')
				word.WriteRune(c)
			}
			state = doubleQuoted
		case comment:
			if c == '\n' {
				state = delimiter
			}
		}
	}

	switch state {
	case unquotedBackslash, singleQuoted, doubleQuoted, doubleQuotedBackslash:
		return nil, ErrMismatchedQuotes
	case unquoted:
		words = append(words, word.String())
	}
	return words, nil
}

// ParseArgs is Split with QuickDev's error wording.
func ParseArgs(input string) ([]string, error) {
	words, err := Split(input)
	if err != nil {
		return nil, errors.New("invalid shell string: " + err.Error())
	}
	return words, nil
}
