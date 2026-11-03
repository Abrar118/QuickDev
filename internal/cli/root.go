// Package cli wires QuickDev's subcommands together.
package cli

import (
	"errors"
	"fmt"
	"os"
	"regexp"
	"runtime/debug"
	"strings"

	"github.com/spf13/cobra"

	"github.com/Abrar118/QuickDev/internal/config"
	"github.com/Abrar118/QuickDev/internal/fzf"
	"github.com/Abrar118/QuickDev/internal/ui"
)

// Version is the release this build reports. Release builds may override it
// with -ldflags "-X github.com/Abrar118/QuickDev/internal/cli.Version=…";
// `go install …@vX.Y.Z` builds report the module version instead.
var Version = "0.5.0"

var pseudoVersion = regexp.MustCompile(`\d{14}-[0-9a-f]{12}`)

func version() string {
	if info, ok := debug.ReadBuildInfo(); ok {
		v := info.Main.Version
		// Tagged module versions only: local builds get "(devel)" or a
		// pseudo-version derived from the commit, neither of which is a release.
		if v != "" && v != "(devel)" && !pseudoVersion.MatchString(v) && !strings.Contains(v, "+") {
			return strings.TrimPrefix(v, "v")
		}
	}
	return Version
}

// exitError ends the process with code, printing message unless it is empty.
type exitError struct {
	code    int
	message string
}

func (e *exitError) Error() string { return e.message }

// silentExit exits with code without printing anything more.
func silentExit(code int) error { return &exitError{code: code} }

// runError marks errors produced by a command's own logic, as opposed to
// cobra's argument parsing (which exit with status 2).
type runError struct{ err error }

func (e *runError) Error() string { return e.err.Error() }
func (e *runError) Unwrap() error { return e.err }

// run adapts a command body so its errors are told apart from usage errors.
func run(fn func(cmd *cobra.Command, args []string) error) func(*cobra.Command, []string) error {
	return func(cmd *cobra.Command, args []string) error {
		if err := fn(cmd, args); err != nil {
			var exit *exitError
			if errors.As(err, &exit) {
				return err
			}
			return &runError{err}
		}
		return nil
	}
}

const rootExamples = `Examples:
  quickdev init                                         Initialize a project
  quickdev init --from my-api                           Clone config from another project
  quickdev launch                                       Select items to launch
  quickdev launch --all                                 Launch everything
  quickdev launch my-api                                Interactive picker for a named project
  quickdev add                                          Interactive add
  quickdev remove                                       Interactive removal picker
  quickdev list                                         Show all projects
  quickdev edit                                         Edit project config
  quickdev edit --global                                Edit global config
  quickdev deregister                                   Unregister project
  quickdev completions zsh                              Print shell completion setup`

func newRoot() *cobra.Command {
	var color string
	root := &cobra.Command{
		Use:           "quickdev",
		Short:         "Manage and launch project terminal/app configurations",
		Version:       version(),
		SilenceErrors: true,
		SilenceUsage:  true,
		Args:          cobra.NoArgs,
		PersistentPreRunE: func(cmd *cobra.Command, args []string) error {
			choice, err := ui.ParseColorChoice(color)
			if err != nil {
				return err
			}
			ui.SetColorChoice(choice)
			return nil
		},
		RunE: func(cmd *cobra.Command, args []string) error {
			return errors.New("a subcommand is required")
		},
	}
	root.SetVersionTemplate("quickdev {{.Version}}\n")
	root.Flags().BoolP("version", "V", false, "Print version")
	root.PersistentFlags().StringVar(&color, "color", "auto",
		"When to use colors: auto (default; off when piped or NO_COLOR is set), always, never")
	_ = root.RegisterFlagCompletionFunc("color", fixed("auto", "always", "never"))
	root.CompletionOptions.DisableDefaultCmd = true
	root.SetHelpTemplate(root.HelpTemplate() + "\n" + rootExamples + "\n")

	root.AddCommand(
		initCmd(), launchCmd(), listCmd(), addCmd(), removeCmd(), editCmd(),
		deregisterCmd(), configCmd(), pruneCmd(), validateCmd(), doctorCmd(),
		captureCmd(), completionsCmd(),
	)
	return root
}

// Main runs QuickDev with args (excluding the program name) and returns the
// process exit code.
func Main(args []string) int {
	root := newRoot()
	root.SetArgs(expandMultiValueFlags(args))
	err := root.Execute()
	if err == nil {
		return 0
	}

	var exit *exitError
	if errors.As(err, &exit) {
		if exit.message != "" {
			printError(exit.message)
		}
		return exit.code
	}
	if fzf.IsCancellation(err) {
		ui.Println(ui.Paint(ui.Dim, "Cancelled."))
		return 0
	}
	var runErr *runError
	if errors.As(err, &runErr) {
		printError(err.Error())
		return 1
	}
	// Argument parsing errors.
	printError(err.Error() + "\n\nFor more information, try '--help'.")
	return 2
}

func printError(msg string) {
	ui.Eprintln(ui.Paint(ui.Red.With(ui.Bold), "error:") + " " + msg)
}

// expandMultiValueFlags rewrites `add app … --args a b c` into repeated
// `--args=a --args=b --args=c`, so `--args` takes several values as the Rust
// CLI's did. Values stop at the next token starting with '-'.
func expandMultiValueFlags(args []string) []string {
	var positional []string
	for i := 0; i < len(args) && len(positional) < 2; i++ {
		a := args[i]
		if a == "--color" {
			i++
			continue
		}
		if strings.HasPrefix(a, "-") {
			continue
		}
		positional = append(positional, a)
	}
	if len(positional) < 2 || positional[0] != "add" || positional[1] != "app" {
		return args
	}

	out := make([]string, 0, len(args))
	for i := 0; i < len(args); i++ {
		if args[i] == "--" {
			out = append(out, args[i:]...)
			break
		}
		if args[i] != "--args" {
			out = append(out, args[i])
			continue
		}
		taken := 0
		for i+1 < len(args) && (!strings.HasPrefix(args[i+1], "-") || args[i+1] == "-") {
			i++
			out = append(out, "--args="+args[i])
			taken++
		}
		if taken == 0 {
			out = append(out, "--args") // let the parser report the missing value
		}
	}
	return out
}

// ---------------------------------------------------------------------------
// Completion candidates. These run on every TAB press, so they only read:
// never migrate, prompt, or print. An unreadable config means no suggestions.

type completeFunc = func(*cobra.Command, []string, string) ([]string, cobra.ShellCompDirective)

func fixed(values ...string) completeFunc {
	return func(*cobra.Command, []string, string) ([]string, cobra.ShellCompDirective) {
		return values, cobra.ShellCompDirectiveNoFileComp
	}
}

func projectCandidates(*cobra.Command, []string, string) ([]string, cobra.ShellCompDirective) {
	path, err := config.GlobalConfigPathReadonly()
	if err != nil {
		return nil, cobra.ShellCompDirectiveNoFileComp
	}
	global, err := config.LoadGlobalConfig(path)
	if err != nil {
		return nil, cobra.ShellCompDirectiveNoFileComp
	}
	var out []string
	for _, p := range global.Projects {
		out = append(out, p.Name+"\t"+p.Path)
	}
	return out, cobra.ShellCompDirectiveNoFileComp
}

// currentProjectItems lists names from the .quickdev.toml governing the
// current directory.
func currentProjectItems(apps bool) completeFunc {
	return func(_ *cobra.Command, args []string, _ string) ([]string, cobra.ShellCompDirective) {
		if len(args) > 0 {
			return nil, cobra.ShellCompDirectiveNoFileComp
		}
		cwd, err := os.Getwd()
		if err != nil {
			return nil, cobra.ShellCompDirectiveNoFileComp
		}
		configPath, _, err := config.FindProjectConfig(cwd)
		if err != nil {
			return nil, cobra.ShellCompDirectiveNoFileComp
		}
		cfg, err := config.LoadProjectConfig(configPath)
		if err != nil {
			return nil, cobra.ShellCompDirectiveNoFileComp
		}
		var out []string
		if apps {
			for _, a := range cfg.Applications {
				out = append(out, a.Name)
			}
		} else {
			for _, t := range cfg.Terminals {
				out = append(out, t.Name)
			}
		}
		return out, cobra.ShellCompDirectiveNoFileComp
	}
}

func onlyFirstArg(f completeFunc) completeFunc {
	return func(cmd *cobra.Command, args []string, toComplete string) ([]string, cobra.ShellCompDirective) {
		if len(args) > 0 {
			return nil, cobra.ShellCompDirectiveNoFileComp
		}
		return f(cmd, args, toComplete)
	}
}

func completionsCmd() *cobra.Command {
	return &cobra.Command{
		Use:       "completions <shell>",
		Short:     "Print the shell setup that enables tab completion",
		ValidArgs: []string{"bash", "zsh", "fish", "powershell"},
		Args:      cobra.MatchAll(cobra.ExactArgs(1), cobra.OnlyValidArgs),
		Long: `Print the shell setup that enables tab completion.

Add one line to your shell's startup file:
  zsh         echo 'source <(quickdev completions zsh)' >> ~/.zshrc
  bash        echo 'source <(quickdev completions bash)' >> ~/.bashrc
  fish        echo 'quickdev completions fish | source' >> ~/.config/fish/config.fish
  powershell  Add-Content $PROFILE 'quickdev completions powershell | Out-String | Invoke-Expression'

Completions call back into quickdev, so project and item names stay current.`,
		RunE: run(func(cmd *cobra.Command, args []string) error {
			root := cmd.Root()
			out := os.Stdout
			var err error
			switch args[0] {
			case "bash":
				err = root.GenBashCompletionV2(out, true)
			case "zsh":
				err = root.GenZshCompletion(out)
			case "fish":
				err = root.GenFishCompletion(out, true)
			case "powershell":
				err = root.GenPowerShellCompletionWithDesc(out)
			}
			if err != nil {
				return fmt.Errorf("failed to write completions: %w", err)
			}
			return nil
		}),
	}
}
