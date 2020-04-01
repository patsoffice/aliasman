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

package aliascmd

import (
	"fmt"
	"regexp"

	"github.com/patsoffice/aliasman/internal/alias"
	"github.com/patsoffice/aliasman/internal/cmd"
	"github.com/spf13/cobra"
	"github.com/spf13/viper"
)

// aliasAuditCmd represents the audit command
var (
	aliasAuditCmd = &cobra.Command{
		Use:   "audit",
		Short: "Audit aliases for differences between storage and email providers",
		Run:   aliasAuditRun,
	}
	aliasAuditFlags = struct {
		domain           string
		columns          []string
		excludeSuspended bool
	}{}
)

func init() {
	aliasCmd := cmd.AliasCmd()
	aliasCmd.AddCommand(aliasAuditCmd)

	fields := []string{"Alias", "Domain", "EmailAddresses"}
	aliasAuditCmd.Flags().StringSliceVarP(&aliasAuditFlags.columns, "columns", "c", fields, "Columns to include in output")
	aliasAuditCmd.Flags().StringVarP(&aliasAuditFlags.domain, "domain", "d", "", "Domain to list aliases for")
	aliasAuditCmd.Flags().BoolVarP(&aliasAuditFlags.excludeSuspended, "exclude-suspended", "S", false, "Exclude susprended aliases")
	aliasAuditCmd.Flags().SortFlags = false
}

func aliasAuditRun(cobraCmd *cobra.Command, args []string) {
	// Validate inputs
	err := cmd.ValidateInputs(&aliasAuditFlags.domain, nil, nil)
	if err != nil {
		cmd.ErrorExit(err, cobraCmd)
	}

	sp, err := cmd.StorageProvider()
	if err != nil {
		cmd.ErrorExit(err, nil)
	}

	ep, err := cmd.EmailProvider()
	if err != nil {
		cmd.ErrorExit(err, nil)
	}

	aliasesEmail, err := ep.AliasList(aliasAuditFlags.domain)
	if err != nil {
		cmd.ErrorExit(err, nil)
	}

	readOnly := viper.GetBool("readonly")
	if err := sp.Open(readOnly); err != nil {
		cmd.ErrorExit(fmt.Errorf("Failure opening %s storage: %v", sp.Type(), err), nil)
	}

	filter := alias.Filter{
		Domain:           regexp.MustCompile(aliasAuditFlags.domain),
		ExcludeSuspended: aliasAuditFlags.excludeSuspended,
	}
	aliasesStorage, err := sp.Search(filter, false)
	if err != nil {
		cmd.ErrorExit(err, nil)
	}

	stripped, err := aliasesStorage.StripData([]string{"Alias", "Domain", "EmailAddresses"})
	if err != nil {
		cmd.ErrorExit(err, nil)
	}
	diff1 := stripped.Diff(aliasesEmail)
	diff2 := aliasesEmail.Diff(stripped)

	fmt.Printf("Aliases in %s but not in %s:\n\n", sp.Type(), ep.Type())
	t, err := alias.NewTable(alias.SetAliases(diff1), alias.SetColumns(aliasAuditFlags.columns))
	if err != nil {
		cmd.ErrorExit(err, nil)
	}
	t.Render()

	fmt.Printf("\nAliases in %s but not in %s:\n\n", ep.Type(), sp.Type())
	t, err = alias.NewTable(alias.SetAliases(diff2), alias.SetColumns(aliasAuditFlags.columns))
	if err != nil {
		cmd.ErrorExit(err, nil)
	}
	t.Render()

	if err := sp.Close(); err != nil {
		cmd.ErrorExit(fmt.Errorf("failure closing %s storage: %v", sp.Type(), err), nil)
	}
}
