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

	"github.com/patsoffice/aliasman/internal/cmd"
	"github.com/spf13/cobra"
	"github.com/spf13/viper"
)

// aliassuspendCmd represents the suspend command
var (
	aliasSuspendCmd = &cobra.Command{
		Use:   "suspend",
		Short: "Suspend an email alias",
		Run:   aliasSuspendRun,
	}
	aliasSuspendFlags = struct {
		alias  string
		domain string
	}{}
)

func init() {
	aliasCmd := cmd.AliasCmd()
	aliasCmd.AddCommand(aliasSuspendCmd)

	aliasSuspendCmd.Flags().StringVarP(&aliasSuspendFlags.domain, "domain", "d", "", "Domain to attach the alias to")
	aliasSuspendCmd.Flags().StringVarP(&aliasSuspendFlags.alias, "alias", "a", "", "Alias name (minus domain)")
	aliasSuspendCmd.Flags().SortFlags = false
}

func aliasSuspendRun(cobraCmd *cobra.Command, args []string) {
	// Validate inputs
	err := cmd.ValidateInputs(&aliasSuspendFlags.domain, &aliasSuspendFlags.alias, nil)
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

	readOnly := viper.GetBool("readonly")
	if err := sp.Open(readOnly); err != nil {
		cmd.ErrorExit(fmt.Errorf("failure opening %s storage: %v", sp.Type(), err), nil)
	}
	if readOnly {
		cmd.ErrorExit(fmt.Errorf("alias suspend requires %s to not be readonly", sp.Type()), nil)
	}

	err = ep.AliasDelete(aliasSuspendFlags.alias, aliasSuspendFlags.domain)
	if err != nil {
		cmd.ErrorExit(err, nil)
	}
	fmt.Printf("Suspended alias %s\n", fmt.Sprintf("%s@%s", aliasSuspendFlags.alias, aliasSuspendFlags.domain))

	if err := sp.Suspend(aliasSuspendFlags.alias, aliasSuspendFlags.domain); err != nil {
		cmd.ErrorExit(fmt.Errorf("failure adding to %s storage: %v", sp.Type(), err), nil)
	}
	if err := sp.Close(); err != nil {
		cmd.ErrorExit(fmt.Errorf("failure closing %s storage: %v", sp.Type(), err), nil)
	}
}
