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
	"path/filepath"

	"github.com/spf13/cobra"
	"github.com/spf13/viper"
)

// rootCmd represents the base command when called without any subcommands
var (
	rootCmd = &cobra.Command{
		Use:   "aliasman",
		Short: "aliasman is a tool for managing email aliases",
		Long: `Rackspace email, GSuite and others allow for a large number of email aliases.
The aliasman tool allows for the creation, read, update and delete of
 aliases with various service providers.
`,
	}
	rootFlags = struct {
		cfgDir      string
		cfgFile     string
		emailType   string
		storageType string
		readOnly    bool
	}{}
)

// Execute adds all child commands to the root command and sets flags appropriately.
// This is called by main.main(). It only needs to happen once to the rootCmd.
func Execute() {
	if err := rootCmd.Execute(); err != nil {
		ErrorExit(err, nil)
	}
}

func init() {
	cobra.OnInitialize(initConfig)

	dir := DefaultDir()

	rootCmd.PersistentFlags().SortFlags = false
	rootCmd.PersistentFlags().StringVar(&rootFlags.cfgDir, "config-dir", dir, "Config directory")
	rootCmd.PersistentFlags().StringVar(&rootFlags.cfgFile, "config-file", "config.yaml", "Config file name")
	rootCmd.PersistentFlags().BoolVar(&rootFlags.readOnly, "readonly", false, "Perform read-only operations")
	rootCmd.PersistentFlags().StringVar(&rootFlags.emailType, "email-type", "", "Specify the type of email (see output of 'list-providers')")
	rootCmd.PersistentFlags().StringVar(&rootFlags.storageType, "storage-type", "", fmt.Sprintf("Specify the type of storage (see output of 'list-providers')"))

	viper.BindPFlag("readonly", rootCmd.PersistentFlags().Lookup("readonly"))
	viper.BindPFlag("email_type", rootCmd.PersistentFlags().Lookup("email-type"))
	viper.BindPFlag("storage_type", rootCmd.PersistentFlags().Lookup("storage-type"))
}

// initConfig reads in config file and ENV variables if set.
func initConfig() {
	cfgPath := filepath.Join(rootFlags.cfgDir, rootFlags.cfgFile)
	viper.SetConfigFile(cfgPath)
	viper.AutomaticEnv()

	if err := viper.ReadInConfig(); err != nil {
		ErrorNoExit(err)
	}
}
