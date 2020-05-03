// Copyright © 2020 Patrick Lawrence <patrick.lawrence@gmail.com>
//
// Permission is hereby granted, free of charge, to any person obtaining a copy
// of this software and associated documentation files (the "Software"), to deal
// in the Software without restriction, including without limitation the rights
// to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
// copies of the Software, and to permit persons to whom the Software is
// furnished to do so, subject to the following conditions:
//
// The above copyright notice and this permission notice shall be included in
// all copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN
// THE SOFTWARE.

package cmd

import (
	"bufio"
	"fmt"
	"os"
	"path/filepath"
	"strings"

	"github.com/patsoffice/aliasman/internal/email"
	"github.com/patsoffice/aliasman/internal/storage"
	"github.com/patsoffice/toolbox"
	"github.com/spf13/cobra"
	"github.com/spf13/viper"
)

// configCmd represents the config command
var configCmd = &cobra.Command{
	Use:   "config",
	Short: "Configure aliasman for use",
	Long:  `Config information from the user needed to utilize mail and storage APIs.`,
	Run:   runInit,
}

func init() {
	rootCmd.AddCommand(configCmd)
}

func runInit(cmd *cobra.Command, args []string) {
	for _, f := range storage.ProviderFactories.All() {
		if err := f.Config(); err != nil {
			ErrorExit(err, nil)
		}
	}

	for _, f := range email.ProviderFactories.All() {
		if err := f.Config(); err != nil {
			ErrorExit(err, nil)
		}
	}

	scanner := bufio.NewScanner(os.Stdin)

	if ok := toolbox.CheckYes(scanner, "Set a default domain?", false); ok {
		defaultDomain := toolbox.GetInputString(scanner, "Domain", viper.GetString("default_domain"))
		viper.Set("default_domain", defaultDomain)
	}

	if ok := toolbox.CheckYes(scanner, "Set default email address(es)?", false); ok {
		defaultEmails := strings.Join(viper.GetStringSlice("default_addresses"), ", ")
		for {
			defaultEmails = toolbox.GetInputString(scanner, "Email addresses (comma separated)", defaultEmails)
			defaultEmails = strings.ReplaceAll(defaultEmails, " ", "")
			emails := strings.Split(defaultEmails, ",")
			if len(emails) == 0 {
				fmt.Fprintf(os.Stderr, "Please specify one or more email addresses.")
			} else {
				viper.Set("default_addresses", emails)
				break
			}
		}
	}

	cfgDir := DefaultDir()
	if !pathExists(cfgDir) {
		if ok := toolbox.CheckYes(scanner, fmt.Sprintf("Create configuration directory: %s?", cfgDir), true); ok {
			if err := os.MkdirAll(rootFlags.cfgDir, os.ModeDir|0700); err != nil {
				ErrorExit(err, nil)
			}
		}
	}

	cfgPath := filepath.Join(cfgDir, DefaultConfigFile())
	if ok := toolbox.CheckYes(scanner, fmt.Sprintf("Write config file: %s?", cfgPath), true); ok {
		viper.WriteConfigAs(cfgPath)
	}
}
