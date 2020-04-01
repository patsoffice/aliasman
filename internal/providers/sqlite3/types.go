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
	"database/sql"

	"github.com/patsoffice/aliasman/internal/storage"
)

func init() {
	cn := new(ConfigerNewer)
	storage.ProviderFactories.Register(cn, "sqlite3")
}

// ConfigerNewer implements the factory methods for the sqlite3 provider.
type ConfigerNewer struct{}

// Storer implements the storage provder methods and state for the sqlite3
// provider.
type Storer struct {
	db       *sql.DB
	dbPath   string
	readOnly bool
}

// Scanner is an interface for scanning an sql.Row or sql.Rows type.
type Scanner interface {
	Scan(...interface{}) error
}

// Alias represent an alias as stored in the SQLite database
type Alias struct {
	alias          string
	domain         string
	emailAddresses string
	description    string
	suspended      bool
	createdTS      sql.NullTime
	modifiedTS     sql.NullTime
	suspendedTS    sql.NullTime
}
