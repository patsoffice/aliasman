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

// aliasSearchCmd represents the search command
var (
	aliasSearchCmd = &cobra.Command{
		Use:   "search",
		Short: "Search for aliases via regular expressions",
		Run:   aliasSearchRun,
	}
	aliasSearchFlags = struct {
		searchInputRE    string
		checkSuspended   bool
		excludeSuspended bool
		excludeEnabled   bool
	}{}
)

func checkRegexp(input string) (*regexp.Regexp, error) {
	if input != "" {
		// Add case insensitivity "(?i)" to the regexp
		re, err := regexp.Compile("(?i)" + input)
		if err != nil {
			err = fmt.Errorf("Invalid regular expression: %v", err)
			return nil, err
		}
		return re, nil
	}
	return nil, nil
}

func init() {
	aliasCmd := cmd.AliasCmd()
	aliasCmd.AddCommand(aliasSearchCmd)

	aliasSearchCmd.Flags().StringVarP(&aliasSearchFlags.searchInputRE, "search-regexp", "s", "", "Regular expression for matching aliases (via email addresses, domains, aliases and descriptions)")
	aliasSearchCmd.Flags().BoolVarP(&aliasSearchFlags.excludeSuspended, "exclude-suspended", "e", false, "Exclude susprended aliases")
	aliasSearchCmd.Flags().BoolVarP(&aliasSearchFlags.excludeEnabled, "exclude-enabled", "E", false, "Exclude enabled aliases")
}

func aliasSearchRun(cobraCmd *cobra.Command, args []string) {
	sp, err := cmd.StorageProvider()
	if err != nil {
		cmd.ErrorExit(err, nil)
	}

	searchRegexp, err := checkRegexp(aliasSearchFlags.searchInputRE)
	if err != nil {
		cmd.ErrorExit(err, nil)
	}

	f := alias.Filter{
		Alias:            searchRegexp,
		Domain:           searchRegexp,
		EmailAddress:     searchRegexp,
		Description:      searchRegexp,
		ExcludeEnabled:   aliasSearchFlags.excludeEnabled,
		ExcludeSuspended: aliasSearchFlags.excludeSuspended,
	}

	readOnly := viper.GetBool("readonly")
	if err := sp.Open(readOnly); err != nil {
		cmd.ErrorExit(fmt.Errorf("Failure opening %s storage: %v", sp.Type(), err), nil)
	}
	aliases, err := sp.Search(f, true)
	if err != nil {
		cmd.ErrorExit(fmt.Errorf("Failure filtering aliases from %s storage: %v", sp.Type(), err), nil)
	}
	sort.Sort(aliases)

	// tableRender(aliases, os.Stdout)
	t, err := alias.NewTable(alias.SetAliases(aliases))
	if err != nil {
		cmd.ErrorExit(err, nil)
	}
	t.Render()

	if err := sp.Close(); err != nil {
		cmd.ErrorExit(fmt.Errorf("Failure closing %s storage: %v", sp.Type(), err), nil)
	}
}
