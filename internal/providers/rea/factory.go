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

package rea

import (
	"bufio"
	"context"
	"fmt"
	"os"

	"github.com/patsoffice/aliasman/internal/cmd"
	"github.com/patsoffice/aliasman/internal/email"
	"github.com/patsoffice/reago"
	"github.com/patsoffice/toolbox"
	"github.com/spf13/viper"
)

func init() {
	var (
		rackspaceAPIUserKey, rackspaceAPISecretKey string
	)

	rootCmd := cmd.RootCmd()
	rootCmd.PersistentFlags().StringVar(&rackspaceAPIUserKey, "rackspace-api-user-key", "", "Rackspace API user key")
	rootCmd.PersistentFlags().StringVar(&rackspaceAPISecretKey, "rackspace-api-secret-key", "", "Rackspace API secret key")

	viper.BindPFlag("rackspace_api_user_key", rootCmd.PersistentFlags().Lookup("rackspace-api-user-key"))
	viper.BindPFlag("rackspace_api_secret_key", rootCmd.PersistentFlags().Lookup("rackspace-api-secret-key"))
}

// Config takes input from the user to configure the rackspace_email_api
// provider.
func (cn *ConfigerNewer) Config() error {
	scanner := bufio.NewScanner(os.Stdin)

	if ok := toolbox.CheckYes(scanner, "Configure rackspace_email_api provider", false); ok {
		userKey := toolbox.GetInputString(scanner, "Rackspace API user key", viper.GetString("rackspace_api_user_key"))
		viper.Set("rackspace_api_user_key,", userKey)

		secretKey := toolbox.GetInputString(scanner, "Rackspace API secret key", viper.GetString("rackspace_api_secret_key"))
		viper.Set("rackspace_api_secret_key,", secretKey)

		if ok := toolbox.CheckYes(scanner, "Make rackspace_email_api the default email provider?", true); ok {
			viper.Set("email_type", "rackspace_email_api")
		}
	}
	return nil
}

// New returns a usable instance of the rackspace_email_api provider.
func (cn *ConfigerNewer) New() (email.Provider, error) {
	var err error

	userKey := viper.GetString("rackspace_api_user_key")
	secretKey := viper.GetString("rackspace_api_secret_key")
	if userKey == "" || secretKey == "" {
		return nil, fmt.Errorf("rackspace_email_api provider is not properly configured")
	}

	emailer := Emailer{
		// BUG(patsoffice) need a different context type?
		ctx: context.TODO(),
	}
	emailer.client, err = reago.New(nil, reago.SetUserKey(userKey), reago.SetSecretKey(secretKey))
	if err != nil {
		return nil, err
	}

	return &emailer, nil
}
