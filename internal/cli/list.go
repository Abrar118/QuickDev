package cli

import (
	"fmt"
	"path/filepath"
	"strconv"
	"strings"
	"time"

	"github.com/spf13/cobra"

	"github.com/Abrar118/QuickDev/internal/adapters"
	"github.com/Abrar118/QuickDev/internal/config"
	"github.com/Abrar118/QuickDev/internal/fzf"
	"github.com/Abrar118/QuickDev/internal/launch"
	"github.com/Abrar118/QuickDev/internal/models"
	"github.com/Abrar118/QuickDev/internal/ui"
)

func listCmd() *cobra.Command {
	var missing, asJSON bool
	cmd := &cobra.Command{
		Use:   "list",
		Short: "List all indexed projects",
		Args:  cobra.NoArgs,
		RunE: run(func(cmd *cobra.Command, args []string) error {
			path, err := config.GlobalConfigPath()
			if err != nil {
				return err
			}
			global, err := config.LoadGlobalConfig(path)
			if err != nil {
				return err
			}
			selected := config.ProjectStatuses(global)
			if missing {
				selected = config.MissingStatuses(selected)
			}

			if asJSON {
				ui.Println(config.ProjectsJSON(selected))
				return nil
			}
			if len(global.Projects) == 0 {
				ui.Println("No projects registered yet.")
				ui.Println(ui.Paint(ui.Dim, "Run 'quickdev init' in a project directory to add one."))
				return nil
			}
			if missing && len(selected) == 0 {
				ui.Println(ui.OK("All registered projects are healthy."))
				return nil
			}

			title := "Projects"
			if missing {
				title = "Missing projects"
			}
			ui.Println(ui.Heading(title, strconv.Itoa(len(selected))))

			type block struct {
				healthy bool
				lines   []string
			}
			blocks := make([]block, len(selected))
			unhealthy, width := 0, 0
			for i, status := range selected {
				healthy, lines := projectBlock(status, models.Deref(global.Emulator))
				blocks[i] = block{healthy, lines}
				if !healthy {
					unhealthy++
				}
				for _, l := range lines {
					width = max(width, ui.Width(ui.Strip(l)))
				}
			}
			width = min(max(width, 40), 100)
			rule := ui.Paint(ui.Dim, strings.Repeat("─", width))

			ui.Println(rule)
			for _, b := range blocks {
				for _, l := range b.lines {
					ui.Println(l)
				}
				ui.Println(rule)
			}
			ui.Println()
			if unhealthy > 0 {
				ui.Println(ui.Paint(ui.Dim, "Run 'quickdev doctor' for details or 'quickdev prune' to drop missing projects."))
			}
			return nil
		}),
	}
	cmd.Flags().BoolVar(&missing, "missing", false, "Show only projects whose path or .quickdev.toml is missing")
	cmd.Flags().BoolVar(&asJSON, "json", false, "Output as a JSON array")
	return cmd
}

// projectBlock describes one project: a header, then every terminal and app
// with the details a launch would use. Returns whether the project is healthy.
func projectBlock(status config.ProjectStatus, globalEmulator string) (bool, []string) {
	issue := status.Issue()
	var cfg *models.ProjectConfig
	if issue == "" {
		var err error
		if cfg, err = config.LoadProjectConfig(filepath.Join(status.Path, config.ProjectFileName)); err != nil {
			issue = "config error"
		}
	}
	marker := ui.Paint(ui.Green, "●")
	if issue != "" {
		marker = ui.Paint(ui.Red, "✗")
	}
	header := fmt.Sprintf("%s %s  %s", marker,
		ui.Paint(ui.Bold, fzf.SanitizeRow(status.Name)),
		ui.Paint(ui.Dim, ui.Tilde(fzf.SanitizeRow(status.Path))))
	if status.LastLaunched != nil {
		header += "  " + ui.Paint(ui.Dim, "· launched "+ui.Ago(*status.LastLaunched, time.Now().Unix()))
	}
	lines := []string{header}

	if issue != "" {
		return false, append(lines, "  "+ui.Paint(ui.Red, issue))
	}
	if len(cfg.Terminals) == 0 && len(cfg.Applications) == 0 {
		return true, append(lines, "  "+ui.Paint(ui.Dim, "nothing configured — run 'quickdev add'"))
	}

	if len(cfg.Terminals) > 0 {
		lines = append(lines, "", section("Terminals", len(cfg.Terminals)))
		var rows [][]cell
		for _, t := range cfg.Terminals {
			emulator := newCell(ui.Dim, "auto")
			if e := models.Deref(t.Emulator); e != "" {
				emulator = newCell(ui.Cyan, e)
			} else if globalEmulator != "" {
				emulator = newCell(ui.Cyan, globalEmulator)
			}
			command := newCell(ui.Plain, "")
			if c := strings.TrimSpace(models.Deref(t.Command)); c != "" {
				command = newCell(ui.Yellow, "$ "+c)
			}
			rows = append(rows, []cell{newCell(ui.Bold, t.Name), newCell(ui.Plain, ui.Tilde(t.Path)), emulator, command})
		}
		lines = append(lines, table(rows, "    ")...)
	}

	if len(cfg.Applications) > 0 {
		lines = append(lines, "", section("Apps", len(cfg.Applications)))
		// Same substitution launch performs, so {root} etc. show real paths.
		ctx := launch.MakePlaceholderContext(cfg, status.Path)
		var rows [][]cell
		for _, a := range cfg.Applications {
			args := newCell(ui.Plain, "")
			if len(a.Args) > 0 {
				shown := adapters.ResolveAppArgs(a.Args, ctx)
				for i := range shown {
					shown[i] = ui.Tilde(shown[i])
				}
				args = newCell(ui.Plain, "→ "+strings.Join(shown, " "))
			}
			rows = append(rows, []cell{newCell(ui.Bold, a.Name), args, newCell(ui.Dim, ui.Tilde(a.Path))})
		}
		lines = append(lines, table(rows, "    ")...)
	}
	return true, lines
}

func section(title string, n int) string {
	return "  " + ui.Paint(ui.Cyan, title) + " " + ui.Paint(ui.Dim, n)
}

// cell is a styled table cell. Text is stripped of control characters: it
// comes from .quickdev.toml, and a stray escape would garble the terminal.
type cell struct {
	style ui.Style
	text  string
}

func newCell(style ui.Style, text string) cell { return cell{style, fzf.SanitizeRow(text)} }

// table renders rows with aligned columns, dropping trailing empty cells so no
// row ends in padding.
func table(rows [][]cell, indent string) []string {
	columns := 0
	for _, r := range rows {
		columns = max(columns, len(r))
	}
	widths := make([]int, columns)
	for _, r := range rows {
		for c, cl := range r {
			widths[c] = max(widths[c], ui.Width(cl.text))
		}
	}
	out := make([]string, len(rows))
	for i, r := range rows {
		last := 0
		for c := len(r) - 1; c >= 0; c-- {
			if r[c].text != "" {
				last = c
				break
			}
		}
		parts := make([]string, 0, last+1)
		for c := 0; c <= last; c++ {
			if c == last {
				parts = append(parts, ui.Paint(r[c].style, r[c].text))
			} else {
				parts = append(parts, ui.Pad(r[c].style, r[c].text, widths[c]))
			}
		}
		out[i] = indent + strings.Join(parts, "  ")
	}
	return out
}
