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

package files

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
	var filesPath string

	rootCmd := cmd.RootCmd()
	dir := cmd.DefaultDir()

	rootCmd.PersistentFlags().StringVar(&filesPath, "files-path", filepath.Join(dir, "files"), "files provider storage path")
	viper.BindPFlag("files_path", rootCmd.PersistentFlags().Lookup("files-path"))
}

// Config takes input from the user to configure the sqlite3 provider.
func (cn *ConfigerNewer) Config() error {
	scanner := bufio.NewScanner(os.Stdin)

	if ok := toolbox.CheckYes(scanner, "Configure files provider", false); ok {
		dbPath := toolbox.GetInputString(scanner, "Files path", viper.GetString("files_path"))
		viper.Set("files_path", dbPath)

		if ok := toolbox.CheckYes(scanner, "Make files the default storage provider?", true); ok {
			viper.Set("storage_type", "files")
		}
	}
	return nil
}

// New returns a usable instance of the sqlite3 provider.
func (cn *ConfigerNewer) New() (storage.Provider, error) {
	filesPath := viper.GetString("files_path")

	storer := Storer{
		filesPath: filesPath,
		clock:     toolbox.RealClock{},
	}

	return &storer, nil
}
