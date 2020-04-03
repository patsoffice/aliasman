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
	"crypto/md5"
	"crypto/rand"
	"encoding/base64"
	"fmt"
	"strings"

	"github.com/patsoffice/aliasman/internal/alias"
	"github.com/patsoffice/aliasman/internal/cmd"
	"github.com/spf13/cobra"
	"github.com/spf13/viper"
)

// aliasCreateCmd represents the create command
var (
	aliasCreateCmd = &cobra.Command{
		Use:   "create",
		Short: "Create an email alias",
		Run:   aliasCreateRun,
	}
	aliasCreateFlags = struct {
		alias             string
		domain            string
		addresses         []string
		description       string
		randomAlias       bool
		randomAliasLength int
		useBase64Encoding bool
	}{}
)

func init() {
	aliasCmd := cmd.AliasCmd()
	aliasCmd.AddCommand(aliasCreateCmd)

	aliasCreateCmd.Flags().StringSliceVarP(&aliasCreateFlags.addresses, "email-address", "e", []string{}, "Address(es) for alias to send email to (fully qualified)")
	aliasCreateCmd.Flags().StringVarP(&aliasCreateFlags.domain, "domain", "d", "", "Domain to attach the alias to")
	aliasCreateCmd.Flags().StringVarP(&aliasCreateFlags.alias, "alias", "a", "", "Alias name (minus domain)")
	aliasCreateCmd.Flags().StringVarP(&aliasCreateFlags.description, "description", "D", "", "Description")
	aliasCreateCmd.Flags().BoolVarP(&aliasCreateFlags.randomAlias, "random-alias", "r", false, "Create a random alias")
	aliasCreateCmd.Flags().IntVarP(&aliasCreateFlags.randomAliasLength, "random-length", "l", 16, "Length of the randomly generated alias")
	aliasCreateCmd.Flags().BoolVarP(&aliasCreateFlags.useBase64Encoding, "use-base64", "b", false, "Use RFC 4648 encoding for the alias")
	aliasCreateCmd.Flags().SortFlags = false
}

type randReader interface {
	Read([]byte) (int, error)
}

type realRandReader struct{}

func (r realRandReader) Read(b []byte) (int, error) {
	return rand.Read(b)
}

func randomAlias(r randReader, randomAliasLength int, useBase64Encoding bool) (string, error) {
	// Make a bas64 encoding set that only consists of a-z0-9 -- not an even distribution
	// but should not matter for our use case. Using lowercase letters since most email
	// systems don't distinguish between upper and lowercase latters.
	const encodeSet = "abcdefghijklmnopqrstuvwxyz0123456789abcdefghijklmnopqrstuvwxyz01"

	b := make([]byte, 256)
	_, err := r.Read(b)
	if err != nil {
		return "", fmt.Errorf("generating random data")
	}

	var alias string
	if useBase64Encoding {
		fmt.Println("HERE")
		alias = base64.NewEncoding(encodeSet).WithPadding(base64.NoPadding).EncodeToString(b)
		alias = alias[:randomAliasLength]
	} else {
		alias = fmt.Sprintf("%x", md5.Sum(b))
		alias = alias[:randomAliasLength]
	}

	return alias, nil
}

func aliasCreateRun(cobraCmd *cobra.Command, args []string) {
	// Validate inputs
	err := cmd.ValidateInputs(&aliasCreateFlags.domain, nil, &aliasCreateFlags.addresses)
	if err != nil {
		cmd.ErrorExit(err, cobraCmd)
	}

	if aliasCreateFlags.randomAliasLength > 32 {
		cmd.ErrorExit(fmt.Errorf("max random alias length of 32 bytes"), cobraCmd)
	}

	if !aliasCreateFlags.randomAlias && aliasCreateFlags.alias == "" {
		cmd.ErrorExit(fmt.Errorf("alias needed or specify generating a random alias"), cobraCmd)
	}

	// If --random-alias is set, we generate a random set of bytes to feed to
	// the encoding generators.
	if aliasCreateFlags.randomAlias && aliasCreateFlags.alias == "" {
		var err error
		aliasCreateFlags.alias, err = randomAlias(realRandReader{}, aliasCreateFlags.randomAliasLength, aliasCreateFlags.useBase64Encoding)
		if err != nil {
			cmd.ErrorExit(err, nil)
		}
	}

	sp, err := cmd.StorageProvider()
	if err != nil {
		cmd.ErrorExit(err, nil)
	}
	ep, err := cmd.EmailProvider()
	if err != nil {
		cmd.ErrorExit(err, nil)
	}

	a := alias.Alias{
		Alias:          aliasCreateFlags.alias,
		Domain:         aliasCreateFlags.domain,
		EmailAddresses: aliasCreateFlags.addresses,
		Description:    aliasCreateFlags.description,
	}

	readOnly := viper.GetBool("readonly")
	if readOnly {
		cmd.ErrorExit(fmt.Errorf("alias create requires %s to not be readonly", sp.Type()), nil)
	}
	if err := sp.Open(readOnly); err != nil {
		cmd.ErrorExit(fmt.Errorf("failure opening %s storage: %v", sp.Type(), err), nil)
	}

	// Check if the alias already exists
	alias, err := sp.Get(aliasCreateFlags.alias, aliasCreateFlags.domain)
	if err != nil {
		cmd.ErrorExit(err, nil)
	}
	if alias != nil {
		cmd.ErrorExit(fmt.Errorf("alias %s for domain %s already exists", aliasCreateFlags.alias, aliasCreateFlags.domain), nil)
	}

	err = ep.AliasCreate(aliasCreateFlags.alias, aliasCreateFlags.domain, aliasCreateFlags.addresses...)
	if err != nil {
		cmd.ErrorExit(err, nil)
	}
	fmt.Printf("Created alias %s that points to %s\n", fmt.Sprintf("%s@%s", aliasCreateFlags.alias, aliasCreateFlags.domain), strings.Join(aliasCreateFlags.addresses, ", "))

	if err := sp.Put(a, true); err != nil {
		cmd.ErrorExit(fmt.Errorf("failure adding to %s storage: %v", sp.Type(), err), nil)
	}
	if err := sp.Close(); err != nil {
		cmd.ErrorExit(fmt.Errorf("failure closing %s storage: %v", sp.Type(), err), nil)
	}
}
