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
	"fmt"
	"sort"

	"github.com/patsoffice/aliasman/internal/email"
	"github.com/patsoffice/aliasman/internal/storage"
	"github.com/spf13/cobra"
)

// listProvidersCmd represents the list-providers command
var listProvidersCmd = &cobra.Command{
	Use:   "list-providers",
	Short: "List the available providers",
	Run:   listProvidersRun,
}

func init() {
	rootCmd.AddCommand(listProvidersCmd)
}

func listProvidersRun(cobraCmd *cobra.Command, args []string) {
	fmt.Printf("Email providers:\n\n")
	for _, provider := range sort.StringSlice(email.ProviderFactories.Names()) {
		fmt.Println(provider)
	}

	fmt.Printf("\nStorage providers:\n\n")
	for _, provider := range sort.StringSlice(storage.ProviderFactories.Names()) {
		fmt.Println(provider)
	}
}
