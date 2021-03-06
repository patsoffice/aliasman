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

// aliasUpdDescCmd represents the update command
var (
	aliasUpdDescCmd = &cobra.Command{
		Use:   "update-description",
		Short: "Update an email alias description",
		Run:   aliasUpdDescRun,
	}
	aliasUpdDescFlags = struct {
		alias       string
		domain      string
		addresses   []string
		description string
	}{}
)

func init() {
	aliasCmd := cmd.AliasCmd()
	aliasCmd.AddCommand(aliasUpdDescCmd)

	aliasUpdDescCmd.Flags().StringSliceVarP(&aliasUpdDescFlags.addresses, "email-address", "e", []string{}, "Address(es) for alias to send email to (fully qualified)")
	aliasUpdDescCmd.Flags().StringVarP(&aliasUpdDescFlags.domain, "domain", "d", "", "Domain to attach the alias to")
	aliasUpdDescCmd.Flags().StringVarP(&aliasUpdDescFlags.alias, "alias", "a", "", "Alias name (minus domain)")
	aliasUpdDescCmd.Flags().StringVarP(&aliasUpdDescFlags.description, "description", "D", "", "Description")
	aliasUpdDescCmd.Flags().SortFlags = false
}

func aliasUpdDescRun(cobraCmd *cobra.Command, args []string) {
	// Validate inputs
	err := cmd.ValidateInputs(&aliasUpdDescFlags.domain, &aliasUpdDescFlags.alias, &aliasUpdDescFlags.addresses)
	if err != nil {
		cmd.ErrorExit(err, cobraCmd)
	}

	sp, err := cmd.StorageProvider()
	if err != nil {
		cmd.ErrorExit(err, nil)
	}

	readOnly := viper.GetBool("readonly")
	if readOnly {
		cmd.ErrorExit(fmt.Errorf("alias update requires %s to not be readonly", sp.Type()), nil)
	}
	if err := sp.Open(readOnly); err != nil {
		cmd.ErrorExit(fmt.Errorf("failure opening %s storage: %v", sp.Type(), err), nil)
	}

	// Check if the alias already exists
	a, err := sp.Get(aliasUpdDescFlags.alias, aliasUpdDescFlags.domain)
	if err != nil {
		cmd.ErrorExit(err, nil)
	}
	if a == nil {
		cmd.ErrorExit(fmt.Errorf("alias %s for domain %s doesn't exist", aliasUpdDescFlags.alias, aliasUpdDescFlags.domain), nil)
	}
	a.Description = aliasUpdDescFlags.description

	if err := sp.Update(*a, true); err != nil {
		cmd.ErrorExit(fmt.Errorf("failure adding to %s storage: %v", sp.Type(), err), nil)
	}
	if err := sp.Close(); err != nil {
		cmd.ErrorExit(fmt.Errorf("failure closing %s storage: %v", sp.Type(), err), nil)
	}
}
