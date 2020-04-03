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
	aliasUnsuspendCmd = &cobra.Command{
		Use:   "unsuspend",
		Short: "Unsuspend an email alias",
		Run:   aliasUnsuspendRun,
	}
	aliasUnsuspendFlags = struct {
		alias  string
		domain string
	}{}
)

func init() {
	aliasCmd := cmd.AliasCmd()
	aliasCmd.AddCommand(aliasUnsuspendCmd)

	aliasUnsuspendCmd.Flags().StringVarP(&aliasUnsuspendFlags.domain, "domain", "d", "", "Domain to attach the alias to")
	aliasUnsuspendCmd.Flags().StringVarP(&aliasUnsuspendFlags.alias, "alias", "a", "", "Alias name (minus domain)")
	aliasUnsuspendCmd.Flags().SortFlags = false
}

func aliasUnsuspendRun(cobraCmd *cobra.Command, args []string) {
	// Validate inputs
	err := cmd.ValidateInputs(&aliasUnsuspendFlags.domain, &aliasUnsuspendFlags.alias, nil)
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
		cmd.ErrorExit(fmt.Errorf("Failure opening %s storage: %v", sp.Type(), err), nil)
	}
	if readOnly {
		cmd.ErrorExit(fmt.Errorf("alias unsuspend requires %s to not be readonly", sp.Type()), nil)
	}

	a, err := sp.Get(aliasUnsuspendFlags.alias, aliasUnsuspendFlags.domain)
	if err != nil {
		cmd.ErrorExit(fmt.Errorf("Failure getting alias from %s storage: %v", sp.Type(), err), nil)
	}

	err = ep.AliasCreate(aliasUnsuspendFlags.alias, aliasUnsuspendFlags.domain, a.EmailAddresses...)
	if err != nil {
		cmd.ErrorExit(err, nil)
	}
	fmt.Printf("Unsuspended alias %s\n", fmt.Sprintf("%s@%s", aliasUnsuspendFlags.alias, aliasUnsuspendFlags.domain))

	if err := sp.Unsuspend(aliasUnsuspendFlags.alias, aliasUnsuspendFlags.domain); err != nil {
		cmd.ErrorExit(fmt.Errorf("Failure adding to %s storage: %v", sp.Type(), err), nil)
	}
	if err := sp.Close(); err != nil {
		cmd.ErrorExit(fmt.Errorf("Failure closing %s storage: %v", sp.Type(), err), nil)
	}
}
