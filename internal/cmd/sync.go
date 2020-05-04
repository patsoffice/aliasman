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
	"github.com/patsoffice/aliasman/internal/storage"
	toolbox "github.com/patsoffice/go.toolbox"
	"github.com/spf13/cobra"
	"github.com/spf13/viper"
)

// syncCmd represents the sync command
var (
	syncCmd = &cobra.Command{
		Use:   "sync",
		Short: "sync aliases from one storage type to another",
		Run:   syncCmdRun,
	}
	syncFlags = struct {
		sourceProvider string
		destProvider   string
		yes            bool
	}{}
)

func init() {
	rootCmd.AddCommand(syncCmd)

	syncCmd.Flags().StringVarP(&syncFlags.sourceProvider, "source-storage", "s", "", "Source storage system (required)")
	syncCmd.Flags().StringVarP(&syncFlags.destProvider, "destination-storage", "d", "", "Destination storage system (required)")
	syncCmd.Flags().BoolVarP(&syncFlags.yes, "yes", "y", false, "Answer 'yes' to sync confirmation")
	syncCmd.Flags().SortFlags = false
	syncCmd.MarkFlagRequired("source-storage")
	syncCmd.MarkFlagRequired("destination-storage")
}

func provider(storageType string) (storage.Provider, error) {
	factory := storage.ProviderFactories.Lookup(storageType)
	if factory == nil {
		return nil, fmt.Errorf("Unknown storage type: %v", storageType)
	}
	sp, err := factory.New()
	if err != nil {
		return nil, err
	}

	return sp, nil
}

func getAliasesMap(sp storage.Provider, readOnly bool) (alias.AliasesMap, error) {
	f := alias.Filter{
		Alias:  regexp.MustCompile(`.*`),
		Domain: regexp.MustCompile(`.*`),
	}

	if err := sp.Open(readOnly); err != nil {
		return nil, fmt.Errorf("Failure opening %s storage: %v", sp.Type(), err)
	}
	aliases, err := sp.Search(f, false)
	if err != nil {
		return nil, fmt.Errorf("Failure filtering aliases from %s storage: %v", sp.Type(), err)
	}
	return aliases.ToMap(), nil
}

func syncCmdRun(cmd *cobra.Command, args []string) {
	var (
		err error
	)

	sourceProvider, err := provider(syncFlags.sourceProvider)
	if err != nil {
		ErrorExit(err, nil)
	}
	destProvider, err := provider(syncFlags.destProvider)
	if err != nil {
		ErrorExit(err, nil)
	}

	readOnly := viper.GetBool("readonly")
	sourceMap, err := getAliasesMap(sourceProvider, readOnly)
	if err != nil {
		ErrorExit(err, nil)
	}
	destMap, err := getAliasesMap(destProvider, readOnly)
	if err != nil {
		ErrorExit(err, nil)
	}

	scanner := bufio.NewScanner(os.Stdin)

	for k, v := range sourceMap {
		doCopy := false
		doUpdate := false
		uDiff := ""
		if destAlias, ok := destMap[k]; ok {
			if !v.Equal(destAlias) {
				doUpdate = true
				uDiff = v.UnifiedDiff(destAlias)
			}
		} else {
			doCopy = true
		}

		if doCopy || doUpdate {
			ok := false
			if !syncFlags.yes {
				if doCopy {
					ok = toolbox.CheckYes(scanner, fmt.Sprintf("Add alias for %s@%s to %s?", v.Alias, v.Domain, destProvider.Type()), true)
				} else {
					ok = toolbox.CheckYes(scanner, fmt.Sprintf("Update alias for %s@%s to %s: %s?", v.Alias, v.Domain, destProvider.Type(), uDiff), true)
				}
			}

			if ok || syncFlags.yes {
				if err := destProvider.Put(v, false); err != nil {
					ErrorExit(err, nil)
				}
			}
		}
	}

	sourceProvider.Close()
	destProvider.Close()
}
