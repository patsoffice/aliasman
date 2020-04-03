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
	"sort"

	"github.com/patsoffice/aliasman/internal/alias"
	"github.com/patsoffice/aliasman/internal/cmd"
	"github.com/spf13/cobra"
)

// listMailCmd represents the list-mail command
var (
	aliasListMailCmd = &cobra.Command{
		Use:   "list-mail",
		Short: "List all aliases on the email provider",
		Run:   aliasListMailRun,
	}
	aliasListMailFlags = struct {
		domain    string
		addresses []string
		columns   []string
	}{}
)

func init() {
	aliasCmd := cmd.AliasCmd()
	aliasCmd.AddCommand(aliasListMailCmd)

	fields := []string{"Alias", "Domain", "EmailAddresses"}

	aliasListMailCmd.Flags().StringVarP(&aliasListMailFlags.domain, "domain", "d", "", "Domain to use")
	aliasListMailCmd.Flags().StringSliceVarP(&aliasListMailFlags.addresses, "email-address", "e", []string{}, "Address(es) for alias to send email to (fully qualified)")
	aliasListMailCmd.Flags().StringSliceVarP(&aliasListMailFlags.columns, "columns", "c", fields, "Columns to include in output")
	aliasListMailCmd.Flags().SortFlags = false
}

func aliasListMailRun(cobraCmd *cobra.Command, args []string) {
	ep, err := cmd.EmailProvider()
	if err != nil {
		cmd.ErrorExit(err, nil)
	}

	aliases, err := ep.AliasList(aliasListMailFlags.domain, aliasListMailFlags.addresses...)
	if err != nil {
		cmd.ErrorExit(fmt.Errorf("Failure listing aliases from %s email provider: %v", ep.Type(), err), nil)
	}
	sort.Sort(aliases)

	t, err := alias.NewTable(alias.SetAliases(aliases), alias.SetColumns(aliasListMailFlags.columns))
	if err != nil {
		cmd.ErrorExit(err, nil)
	}
	t.Render()
}
