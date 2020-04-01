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
	"os"
	"path/filepath"

	homedir "github.com/mitchellh/go-homedir"
	"github.com/patsoffice/aliasman/internal/email"
	"github.com/patsoffice/aliasman/internal/storage"
	"github.com/spf13/cobra"
	"github.com/spf13/viper"
)

// RootCmd returns a pointer to the root command so that other modules can
// manipulate it. This primarily so providers can add their options to
// the global flags via their init() funtions.
func RootCmd() *cobra.Command {
	return rootCmd
}

// DefaultDir returns the standard directory for the aliasman configuration
// data.
//
// BUG(patsoffice) this is very UNIX centric.
// BUG(patsoffice) this function should return an eror and not exit from here
func DefaultDir() string {
	// If a config dir was passed on the command line
	if rootFlags.cfgDir != "" {
		return rootFlags.cfgDir
	}

	// Determine the home directory to set the default config path
	home, err := homedir.Dir()
	if err != nil {
		ErrorNoExit(err)
	}
	// BUG(patsoffice) this is very UNIX centric. Different for different
	// platforms?
	p := filepath.Join(home, ".config", "aliasman")

	return p
}

// DefaultConfigFile returns the specified config file name or the default
// if one has not beeen specified.
func DefaultConfigFile() string {
	// If a config file was passed on the command line
	if rootFlags.cfgFile != "" {
		return rootFlags.cfgFile
	}
	return "config.yaml"
}

// StorageProvider returns a storage provider of the type specified in a
// Viper string. If a provider that is specified does not exist or there
// was a problem creating the provider, an error is returned.
func StorageProvider() (storage.Provider, error) {
	storageType := viper.GetString("storage_type")

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

// EmailProvider returns an email provider of the type specified in a
// Viper string. If a provider that is specified does not exist or there
// was a problem creating the provider, an error is returned.
func EmailProvider() (email.Provider, error) {
	emailType := viper.GetString("email_type")

	factory := email.ProviderFactories.Lookup(emailType)
	if factory == nil {
		return nil, fmt.Errorf("Unknown email type: %v", emailType)
	}
	ep, err := factory.New()
	if err != nil {
		return nil, err
	}

	return ep, nil
}

// ValidateInputs ensures that valid input is provided for domain, alias and
// email addresses. If empty input is given for domain and email addresses,
// we attempt to retrieve defaults from the Viper configuration.
func ValidateInputs(domain, alias *string, emailAddresses *[]string) error {
	if domain != nil && *domain == "" {
		*domain = viper.GetString("default_domain")
	}
	if *domain == "" {
		return fmt.Errorf("domain needed")
	}

	if alias != nil && *alias == "" {
		return fmt.Errorf("alias needed")
	}

	if emailAddresses != nil && len(*emailAddresses) == 0 {
		defaultAddress := viper.GetStringSlice("default_addresses")

		if defaultAddress == nil {
			return fmt.Errorf("email address(es) needed")
		}
		*emailAddresses = defaultAddress
	}

	return nil
}

func pathExists(path string) bool {
	if path == "" {
		return false
	}
	_, err := os.Stat(path)
	if err == nil {
		return true
	}
	if !os.IsNotExist(err) {
		ErrorNoExit(err)
	}
	return false
}

// ErrorExit takes an error value and possibly pointer to a Cobra command.
// The error value is printed to stderr. If the pointer to the Cobra command
// is non-nil, it's Help() method is run. THe program is then run with a
// non-zero exit value.
//
// BUG(patsoffice) allow exit values to be passed? Necessary?
// BUG(patsoffice) allow setting of a Writer so that something other than stderr can be used?
func ErrorExit(err error, cmd *cobra.Command) {
	fmt.Fprintln(os.Stderr, err)
	if cmd != nil {
		fmt.Fprintln(os.Stderr)
		cmd.Help()
	}
	os.Exit(1)
}

// ErrorNoExit takes and error value and outputs it to stderr.
// BUG(patsoffice) allow setting of a Writer so that something other than stderr can be used?
func ErrorNoExit(err error) {
	fmt.Fprintln(os.Stderr, err)
}
