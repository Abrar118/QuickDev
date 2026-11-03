// Command quickdev saves and launches a project's terminal tabs and apps.
package main

import (
	"os"

	"github.com/Abrar118/QuickDev/internal/cli"
)

func main() {
	os.Exit(cli.Main(os.Args[1:]))
}
