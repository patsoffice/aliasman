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
	"encoding/json"
	"errors"
	"fmt"
	"io/ioutil"
	"os"
	"path/filepath"
	"sort"
	"time"

	"github.com/patsoffice/aliasman/internal/alias"
)

// Type returns the type of provider this package implements
func (s Storer) Type() string {
	return "files"
}

// Description returns the description of the provider this package implements
func (s Storer) Description() string {
	return "Files-based backed alias storage"
}

func (sa FileAlias) toAlias() alias.Alias {
	a := alias.Alias{
		Alias:          sa.Alias,
		Domain:         sa.Domain,
		EmailAddresses: sa.EmailAddresses,
		Description:    sa.Description,
		Suspended:      sa.Suspended,
		CreatedTS:      sa.CreatedTS,
		ModifiedTS:     sa.ModifiedTS,
		SuspendedTS:    sa.SuspendedTS,
	}

	return a
}

// Open reads the JSON files from the path set in the files_path configuration
// key. The readOnly argument determines if mutating functions can operate on
// the files. If the storage directory does not exist, it is created
// (assuming the files were not opened with readOnly set). An error is
// returned if there is a problem opening the files.
func (s *Storer) Open(readOnly bool) error {
	var err error

	s.readOnly = readOnly

	stat, err := os.Stat(s.filesPath)
	if err != nil {
		if s.readOnly {
			return fmt.Errorf("files opened read-only and files path does not exist: %s", s.filesPath)
		}
		if err := os.MkdirAll(s.filesPath, 0700); err != nil {
			return fmt.Errorf("cannot create files directory: %v", err)
		}
	}
	if !stat.IsDir() {
		return fmt.Errorf("%s is a file and not a directory", s.filesPath)
	}

	files, err := ioutil.ReadDir(s.filesPath)
	if err != nil {
		return fmt.Errorf("problem reading files directory %s: %v", s.filesPath, err)
	}

	s.aliases = alias.AliasesMap{}
	for _, file := range files {
		a := FileAlias{}
		b, err := ioutil.ReadFile(filepath.Join(s.filesPath, file.Name()))
		if err != nil {
			return fmt.Errorf("%v", err)
		}
		if err := json.Unmarshal(b, &a); err != nil {
			return err
		}
		s.aliases.Add(a.toAlias())
	}

	return nil
}

// Close for the files provider is a no-op currently.
func (s *Storer) Close() error {
	return nil
}

func aliasFilename(a, d string) string {
	return fmt.Sprintf("alias-%s-%s", a, d)
}

// Get retrieves a single Alias from the file on disk. A pointer to the Alias
// is returned with any error condition during retrieval.
func (s Storer) Get(alias, domain string) (*alias.Alias, error) {
	a := FileAlias{}
	b, err := ioutil.ReadFile(filepath.Join(s.filesPath, aliasFilename(alias, domain)))
	if err != nil {
		return nil, fmt.Errorf("%v", err)
	}
	if err := json.Unmarshal(b, &a); err != nil {
		return nil, err
	}
	r := a.toAlias()

	return &r, nil
}

// toFileAlias converts from a file's alias to the standard Alias type.
func toFileAlias(a alias.Alias) FileAlias {
	fa := FileAlias{
		Alias:          a.Alias,
		Domain:         a.Domain,
		EmailAddresses: a.EmailAddresses,
		Description:    a.Description,
		Suspended:      a.Suspended,
		CreatedTS:      a.CreatedTS,
		ModifiedTS:     a.ModifiedTS,
		SuspendedTS:    a.SuspendedTS,
	}
	return fa
}

func writeAliasFile(filesPath string, fa FileAlias) error {
	buf, err := json.MarshalIndent(fa, "", " ")
	if err != nil {
		return fmt.Errorf("error marshalling JSON: %v", err)
	}

	fn := filepath.Join(filesPath, aliasFilename(fa.Alias, fa.Domain))
	if err := ioutil.WriteFile(fn, buf, 0644); err != nil {
		return fmt.Errorf("error writing file %s: %v", fn, err)
	}

	return nil
}

// Put stores the Alias in a file. If the updateModified flag is set,
// the modified timestamp field is updated with the timestamp of when the
// Put call was made. The error condition of the Put operation is returned.
func (s Storer) Put(a alias.Alias, updateModified bool) error {
	if s.readOnly {
		return errors.New("files provider opened readonly")
	}

	if updateModified {
		a.ModifiedTS = s.clock.Now()
	}

	fa := toFileAlias(a)
	if err := writeAliasFile(s.filesPath, fa); err != nil {
		return fmt.Errorf("unable to write alias file: %v", err)
	}
	s.aliases.Add(a)

	return nil
}

// Update stores the updated Alias in the database. If the updateModified
// flag is set, the modified timestamp column is updated with the timestamp
// of when the Update call was made. The error condition of the Update
// operation is returned.
func (s Storer) Update(a alias.Alias, updateModified bool) error {
	if s.readOnly {
		return errors.New("S3 opened readonly")
	}

	if err := s.Put(a, updateModified); err != nil {
		return err
	}

	s.aliases.Del(a)
	s.aliases.Add(a)

	return nil
}

// Search applies the filter to the database query results and returns a
// sorted list of aliases that match the filter criteria.
func (s Storer) Search(f alias.Filter, matchAnyRegexp bool) (alias.Aliases, error) {
	var aliases alias.Aliases

	for _, a := range s.aliases {
		if a.Filter(f, matchAnyRegexp) {
			aliases = append(aliases, a)
		}
	}
	sort.Sort(aliases)

	return aliases, nil
}

func (s Storer) suspension(alias, domain string, suspending bool) error {
	if s.readOnly {
		return errors.New("files provider opened readonly")
	}

	a, err := s.Get(alias, domain)
	if err != nil {
		return err
	}
	if a != nil {
		if suspending {
			a.Suspended = true
			a.SuspendedTS = s.clock.Now().UTC()
		} else {
			a.Suspended = false
			a.SuspendedTS = time.Time{}
		}

		if err := s.Put(*a, true); err != nil {
			return err
		}
		s.aliases.Del(*a)
		s.aliases.Add(*a)
	} else {
		return fmt.Errorf("alias %s@%s does not exist", a.Alias, a.Domain)
	}
	return nil
}

// Suspend updates the row in the database representing the alias with the
// suspended flag is true and updates the suspended timestamp. An error is
// returned if there is a failure interacting with the database.
func (s Storer) Suspend(alias, domain string) error {
	return s.suspension(alias, domain, true)
}

// Unsuspend updates the row in the database representing the alias with the
// suspended flag is false and updates the suspended timestamp. An error is
// returned if there is a failure interacting with the database.
func (s Storer) Unsuspend(alias, domain string) error {
	return s.suspension(alias, domain, false)
}

// Delete removes the row from the database representing the alias. An error
// is returned if there is a failure interacting with database.
func (s Storer) Delete(a, d string) error {
	if s.readOnly {
		return errors.New("files provider opened readonly")
	}

	fn := filepath.Join(s.filesPath, aliasFilename(a, d))
	if err := os.Remove(fn); err != nil {
		return fmt.Errorf("error removing file %s: %v", fn, err)
	}

	s.aliases.Del(alias.Alias{Alias: a, Domain: d})

	return nil
}
