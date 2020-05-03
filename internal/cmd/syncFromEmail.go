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
	"regexp"

	"github.com/patsoffice/aliasman/internal/alias"
	"github.com/patsoffice/toolbox"
	"github.com/spf13/cobra"
	"github.com/spf13/viper"
)

// syncFromEmailCmd represents the sync command
var (
	syncFromEmailCmd = &cobra.Command{
		Use:   "sync-from-email",
		Short: "sync aliases from email provider to the storage provider",
		Run:   syncFromEmailCmdRun,
	}
	syncFromEmailFlags = struct {
		domain    string
		addresses []string
		yes       bool
	}{}
)

func init() {
	rootCmd.AddCommand(syncFromEmailCmd)

	syncFromEmailCmd.Flags().StringVarP(&syncFromEmailFlags.domain, "domain", "d", "", "Domain to attach the alias to")
	syncFromEmailCmd.Flags().StringSliceVarP(&syncFromEmailFlags.addresses, "email-address", "e", []string{}, "Address(es) for alias to send email to (fully qualified)")
	syncFromEmailCmd.Flags().BoolVarP(&syncFromEmailFlags.yes, "yes", "y", false, "Answer 'yes' to sync confirmation")
	syncFromEmailCmd.Flags().SortFlags = false
}

func syncFromEmailCmdRun(cmd *cobra.Command, args []string) {
	// Validate inputs
	err := ValidateInputs(&syncFromEmailFlags.domain, nil, nil)
	if err != nil {
		ErrorExit(err, cmd)
	}

	sp, err := StorageProvider()
	if err != nil {
		ErrorExit(err, nil)
	}
	ep, err := EmailProvider()
	if err != nil {
		ErrorExit(err, nil)
	}

	sourceAliases, err := ep.AliasList(syncFromEmailFlags.domain, syncFromEmailFlags.addresses...)
	if err != nil {
		ErrorExit(err, nil)
	}
	sourceMap := sourceAliases.ToMap()

	readOnly := viper.GetBool("readonly")
	if err := sp.Open(readOnly); err != nil {
		ErrorExit(err, nil)
	}

	filter := alias.Filter{
		Alias:  regexp.MustCompile(`.*`),
		Domain: regexp.MustCompile(syncFromEmailFlags.domain),
	}
	destAliases, err := sp.Search(filter, false)
	if err != nil {
		ErrorExit(err, nil)
	}
	destMap := destAliases.ToMap()

	scanner := bufio.NewScanner(os.Stdin)
	for k, v := range sourceMap {
		if _, ok := destMap[k]; !ok {
			yes := false
			if !syncFromEmailFlags.yes {
				yes = toolbox.CheckYes(scanner, fmt.Sprintf("Add alias for %s@%s to %s?", v.Alias, v.Domain, sp.Type()), true)
			}

			if yes || syncFromEmailFlags.yes {
				if err := sp.Put(v, true); err != nil {
					ErrorExit(err, nil)
				}
			}
		}
	}

	sp.Close()
}
