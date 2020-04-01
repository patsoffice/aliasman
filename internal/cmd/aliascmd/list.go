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
	"sort"

	"github.com/patsoffice/aliasman/internal/alias"
	"github.com/patsoffice/aliasman/internal/cmd"
	"github.com/spf13/cobra"
	"github.com/spf13/viper"
)

// listCmd represents the list command
var (
	aliasListCmd = &cobra.Command{
		Use:   "list",
		Short: "List all aliases",
		Run:   aliasListRun,
	}
	aliasListFlags = struct {
		columns []string
	}{}
)

func init() {
	aliasCmd := cmd.AliasCmd()
	aliasCmd.AddCommand(aliasListCmd)

	fields := alias.Alias{}.Fields()
	aliasListCmd.Flags().StringSliceVarP(&aliasListFlags.columns, "columns", "c", fields, "Columns to include in output")
	aliasListCmd.Flags().SortFlags = false
}

func aliasListRun(cobraCmd *cobra.Command, args []string) {
	sp, err := cmd.StorageProvider()
	if err != nil {
		cmd.ErrorExit(err, nil)
	}

	f := alias.Filter{
		Alias:  regexp.MustCompile(`.*`),
		Domain: regexp.MustCompile(`.*`),
	}

	readOnly := viper.GetBool("readonly")
	if err := sp.Open(readOnly); err != nil {
		cmd.ErrorExit(fmt.Errorf("Failure opening %s storage: %v", sp.Type(), err), nil)
	}
	aliases, err := sp.Search(f, false)
	if err != nil {
		cmd.ErrorExit(fmt.Errorf("Failure filtering aliases from %s storage: %v", sp.Type(), err), nil)
	}
	sort.Sort(aliases)

	t, err := alias.NewTable(alias.SetAliases(aliases), alias.SetColumns(aliasListFlags.columns))
	if err != nil {
		cmd.ErrorExit(err, nil)
	}
	t.Render()

	if err := sp.Close(); err != nil {
		cmd.ErrorExit(fmt.Errorf("Failure closing %s storage: %v", sp.Type(), err), nil)
	}
}
