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
	"time"

	"github.com/patsoffice/aliasman/internal/alias"
	"github.com/patsoffice/aliasman/internal/storage"
	"github.com/patsoffice/aliasman/internal/util"
)

func init() {
	cn := new(ConfigerNewer)
	storage.ProviderFactories.Register(cn, "files")
}

// ConfigerNewer implements the factory methods for the files provider.
type ConfigerNewer struct{}

// Storer implements the storage provder methods and state for the files
// provider.
type Storer struct {
	filesPath string
	readOnly  bool
	aliases   alias.AliasesMap
	clock     util.Clock
}

// FileAlias represents the state necessary for a file alias.
type FileAlias struct {
	Alias          string    `json:"alias"`
	Domain         string    `json:"domain"`
	EmailAddresses []string  `json:"email_addresses"`
	Description    string    `json:"description"`
	Suspended      bool      `json:"suspended"`
	CreatedTS      time.Time `json:"created_ts"`
	ModifiedTS     time.Time `json:"modified_ts"`
	SuspendedTS    time.Time `json:"suspended_ts"`
}
