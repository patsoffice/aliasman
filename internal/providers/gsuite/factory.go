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

package gsuite

import (
	"bufio"
	"context"
	"encoding/json"
	"fmt"
	"io/ioutil"
	"net/http"
	"os"
	"path/filepath"

	"github.com/patsoffice/aliasman/internal/cmd"
	"github.com/patsoffice/aliasman/internal/email"
	"github.com/patsoffice/aliasman/internal/util"
	"github.com/spf13/viper"
	"golang.org/x/oauth2"
	"golang.org/x/oauth2/google"
	admin "google.golang.org/api/admin/directory/v1"
)

func init() {
	var (
		credentialsPath, tokenPath string
	)
	rootCmd := cmd.RootCmd()
	dir := cmd.DefaultDir()

	rootCmd.PersistentFlags().StringVar(&credentialsPath, "gsuite-credentials", filepath.Join(dir, "gsuite-credentials.json"), "GSuite credentials path")
	rootCmd.PersistentFlags().StringVar(&tokenPath, "gsuite-token", filepath.Join(dir, "gsuite-token.json"), "GSuite token path")

	viper.BindPFlag("gsuite_credentials", rootCmd.PersistentFlags().Lookup("gsuite-credentials"))
	viper.BindPFlag("gsuite_token", rootCmd.PersistentFlags().Lookup("gsuite-token"))
}

// Request a token from the web, then returns the retrieved token. Returns an
// error if there is a problem retrieving the token from the web.
func (cn *ConfigerNewer) getTokenFromWeb(config *oauth2.Config) (*oauth2.Token, error) {
	scanner := bufio.NewScanner(os.Stdin)

	authURL := config.AuthCodeURL("state-token", oauth2.AccessTypeOffline)
	msg := "Go to the following link in your browser: \n\n%v\n\nthen type the authorization code"
	authCode := util.GetInputString(scanner, fmt.Sprintf(msg, authURL), "")

	tok, err := config.Exchange(context.TODO(), authCode)
	if err != nil {
		return nil, fmt.Errorf("Unable to retrieve token from web: %v", err)
	}
	return tok, nil
}

// tokenFromFile retrieves a token from a local file.
func (cn *ConfigerNewer) tokenFromFile(path string) (*oauth2.Token, error) {
	f, err := os.Open(path)
	if err != nil {
		return nil, err
	}
	defer f.Close()
	tok := &oauth2.Token{}
	err = json.NewDecoder(f).Decode(tok)
	return tok, err
}

// saveToken writes a token to a local file.
func (cn *ConfigerNewer) saveToken(path string, token *oauth2.Token) error {
	f, err := os.OpenFile(path, os.O_RDWR|os.O_CREATE|os.O_TRUNC, 0600)
	if err != nil {
		return fmt.Errorf("Unable to cache oauth token: %v", err)
	}
	defer f.Close()

	json.NewEncoder(f).Encode(token)
	return nil
}

// Config takes input from the user to configure the gsuite provider.
func (cn *ConfigerNewer) Config() error {
	scanner := bufio.NewScanner(os.Stdin)

	if ok := util.CheckYes(scanner, "Configure gsuite provider", false); ok {
		credentiasPath := util.GetInputString(scanner, "GSuite credential path", viper.GetString("gsuite_credentials"))
		b, err := ioutil.ReadFile(credentiasPath)
		if err != nil {
			return fmt.Errorf("Unable to read client secret file: %v", err)
		}
		config, err := google.ConfigFromJSON(b, admin.AdminDirectoryUserAliasScope)
		if err != nil {
			return err
		}
		viper.Set("gsuite_credentials,", credentiasPath)

		tokenPath := util.GetInputString(scanner, "GSuite token path", viper.GetString("gsuite_token"))
		if tok, err := cn.tokenFromFile(tokenPath); err != nil {
			cmd.ErrorNoExit(err)
			if tok, err = cn.getTokenFromWeb(config); err != nil {
				return err
			}
			if err = cn.saveToken(tokenPath, tok); err != nil {
				return err
			}
		}
		viper.Set("gsuite_token,", tokenPath)

		if ok := util.CheckYes(scanner, "Make gsuite the default email provider?", true); ok {
			viper.Set("email_type", "gsuite")
		}
	}
	return nil
}

// Retrieve a token, saves the token, then returns the generated client.
func (cn *ConfigerNewer) getClient() (*http.Client, error) {
	b, err := ioutil.ReadFile(viper.GetString("gsuite_credentials"))
	if err != nil {
		return nil, fmt.Errorf("Unable to read client secret file: %v", err)
	}
	config, err := google.ConfigFromJSON(b, admin.AdminDirectoryUserAliasScope)
	if err != nil {
		return nil, err
	}

	path := viper.GetString("gsuite_token")
	tok, err := cn.tokenFromFile(path)
	if err != nil {
		cmd.ErrorNoExit(err)
		if tok, err = cn.getTokenFromWeb(config); err != nil {
			return nil, err
		}
		if err := cn.saveToken(path, tok); err != nil {
			return nil, err
		}
	}
	tokenSource := config.TokenSource(oauth2.NoContext, tok)
	tokenSource = oauth2.ReuseTokenSource(tok, tokenSource)

	newToken, err := tokenSource.Token()
	if err != nil {
		return nil, err
	}

	if newToken.AccessToken != tok.AccessToken {
		if err := cn.saveToken(path, newToken); err != nil {
			return nil, err
		}
	}

	return config.Client(context.Background(), newToken), nil
}

// New returns a usable instance of the rackspace_email_api provider.
func (cn *ConfigerNewer) New() (email.Provider, error) {
	// var err error

	// userKey := viper.GetString("rackspace_api_user_key")
	// secretKey := viper.GetString("rackspace_api_secret_key")
	// if userKey == "" || secretKey == "" {
	// 	return nil, fmt.Errorf("rackspace_email_api provider is not properly configured")
	// }
	client, err := cn.getClient()
	if err != nil {
		return nil, err
	}

	srv, err := admin.New(client)
	if err != nil {
		return nil, fmt.Errorf("Unable to retrieve directory Client %v", err)
	}

	emailer := Emailer{
		// BUG(patsoffice) need a different context type?
		ctx: context.TODO(),
		srv: srv,
	}
	// emailer.client, err = reago.New(nil, reago.SetUserKey(userKey), reago.SetSecretKey(secretKey))
	// if err != nil {
	// 	return nil, err
	// }

	return &emailer, nil
}
