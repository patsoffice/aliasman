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

package sqlite3

import (
	"bufio"
	"os"
	"path/filepath"

	"github.com/patsoffice/aliasman/internal/cmd"
	"github.com/patsoffice/aliasman/internal/storage"
	toolbox "github.com/patsoffice/go.toolbox"
	"github.com/spf13/viper"
)

func init() {
	var dbPath string

	rootCmd := cmd.RootCmd()
	dir := cmd.DefaultDir()

	rootCmd.PersistentFlags().StringVar(&dbPath, "sqlite3-db-path", filepath.Join(dir, "aliasman.sqlite"), "sqlite3 database path")
	viper.BindPFlag("sqlite3_db_path", rootCmd.PersistentFlags().Lookup("sqlite3-db-path"))
}

// Config takes input from the user to configure the sqlite3 provider.
func (cn *ConfigerNewer) Config() error {
	scanner := bufio.NewScanner(os.Stdin)

	if ok := toolbox.CheckYes(scanner, "Configure sqlite3 provider", false); ok {
		dbPath := toolbox.GetInputString(scanner, "Sqlite3 database path", viper.GetString("sqlite3_db_path"))
		viper.Set("sqlite3_db_path", dbPath)

		if ok := toolbox.CheckYes(scanner, "Make sqlite3 the default storage provider?", true); ok {
			viper.Set("storage_type", "sqlite3")
		}
	}
	return nil
}

// New returns a usable instance of the sqlite3 provider.
func (cn *ConfigerNewer) New() (storage.Provider, error) {
	dbPath := viper.GetString("sqlite3_db_path")

	storer := Storer{dbPath: dbPath}

	return &storer, nil
}
