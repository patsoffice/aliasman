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
	"errors"
	"sort"
	"strings"
	"time"

	// Register the sqlite3 driver as a database driver
	_ "github.com/mattn/go-sqlite3"
	"github.com/patsoffice/aliasman/internal/alias"
)

// Type returns the type of provider this package implements
func (s Storer) Type() string {
	return "sqlite3"
}

// Description returns the description of the provider this package implements
func (s Storer) Description() string {
	return "SQLite backed alias storage"
}

// Open reads the sqlite3 database file from the value set in the
// sqlite3_db_path configuration key. The readOnly argument determines if
// mutating functions can operate on the database. If the table containing
// state does not exist, it is created (assuming the database was not opened
// with readOnly set. An error is returned if there is a problem opening the
// database.
func (s *Storer) Open(readOnly bool) error {
	var err error

	s.readOnly = readOnly

	s.db, err = sql.Open("sqlite3", s.dbPath)
	if err != nil {
		return err
	}

	if !s.readOnly {
		_, err = s.db.Exec(createAliasTableSQL)
		if err != nil {
			return err
		}
	}

	return nil
}

// Close safely closes the database and returns the error result from the
// close call.
func (s *Storer) Close() error {
	err := s.db.Close()
	return err
}

func (sa Alias) toAlias() alias.Alias {
	a := alias.Alias{
		Alias:          sa.alias,
		Domain:         sa.domain,
		EmailAddresses: strings.Split(sa.emailAddresses, ","),
		Description:    sa.description,
		Suspended:      sa.suspended,
	}

	if sa.createdTS.Valid {
		a.CreatedTS = sa.createdTS.Time
	}
	if sa.modifiedTS.Valid {
		a.ModifiedTS = sa.modifiedTS.Time
	}
	if sa.suspendedTS.Valid {
		a.SuspendedTS = sa.suspendedTS.Time
	}

	return a
}

// scanAlias scans the columns from an sql.Row or sql.Rows type and returns
// the results in an Alias. If there is an error scanning, an error is
// returned.
func scanAlias(row Scanner) (alias.Alias, error) {
	sa := Alias{}
	err := row.Scan(
		&sa.alias,
		&sa.domain,
		&sa.emailAddresses,
		&sa.description,
		&sa.suspended,
		&sa.createdTS,
		&sa.modifiedTS,
		&sa.suspendedTS,
	)
	return sa.toAlias(), err
}

// Get retrieves a single Alias from the database. A pointer to the Alias
// is returned with the error condition during retrieval.
func (s Storer) Get(alias, domain string) (*alias.Alias, error) {
	row := s.db.QueryRow(getSingleAliasSQL, alias, domain)
	a, err := scanAlias(row)
	if err != nil {
		return nil, err
	}

	return &a, nil
}

// Put stores the Alias in the database. If the updateModified flag is set,
// the modified timestamp column is updated with the timestamp of when the
// Put call was made. The error condition of the Put operation is returned.
func (s Storer) Put(alias alias.Alias, updateModified bool) error {
	var (
		stmt *sql.Stmt
		err  error
	)

	if s.readOnly {
		return errors.New("SQLite DB opened readonly")
	}

	dest := strings.Join(alias.EmailAddresses, ",")

	stmt, err = s.db.Prepare(putAliasSQL)
	if err != nil {
		return err
	}

	createdTS := alias.CreatedTS
	modifiedTS := alias.ModifiedTS
	if createdTS.IsZero() {
		createdTS = time.Now()
	}
	if updateModified {
		modifiedTS = time.Now()
	}
	_, err = stmt.Exec(alias.Alias, dest, alias.Domain, alias.Description, alias.Suspended, createdTS, modifiedTS, alias.SuspendedTS)
	if err != nil {
		return err
	}

	return nil
}

// Update stores the updated Alias in the database. If the updateModified
// flag is set, the modified timestamp column is updated with the timestamp
// of when the Update call was made. The error condition of the Update
// operation is returned.
func (s Storer) Update(alias alias.Alias, updateModified bool) error {
	// We can just call Put since the SQL is an INSERT OR UPDATE
	return s.Put(alias, updateModified)
}

// Search applies the filter to the database query results and returns a
// sorted list of aliases that match the filter criteria.
func (s Storer) Search(f alias.Filter, matchAnyRegexp bool) (alias.Aliases, error) {
	var aliases alias.Aliases

	stmt, err := s.db.Prepare(getAllAliasSQL)
	if err != nil {
		return nil, err
	}

	rows, err := stmt.Query()
	if err != nil {
		if err != sql.ErrNoRows {
			return nil, err
		}
		return nil, nil
	}
	defer rows.Close()

	for rows.Next() {
		a, err := scanAlias(rows)
		if err != nil {
			return nil, err
		}

		if a.Filter(f, matchAnyRegexp) {
			aliases = append(aliases, a)
		}
	}
	sort.Sort(aliases)

	return aliases, nil
}

// Suspend updates the row in the database representing the alias with the
// suspended flag is true and updates the suspended timestamp. An error is
// returned if there is a failure interacting with the database.
func (s Storer) Suspend(alias, domain string) error {
	var (
		stmt *sql.Stmt
		err  error
	)

	if s.readOnly {
		return errors.New("SQLite DB opened readonly")
	}

	stmt, err = s.db.Prepare(suspendAliasSQL)
	if err != nil {
		return err
	}

	_, err = stmt.Exec(
		alias,
		domain,
	)
	if err != nil {
		return err
	}
	return nil
}

// Unsuspend updates the row in the database representing the alias with the
// suspended flag is false and updates the suspended timestamp. An error is
// returned if there is a failure interacting with the database.
func (s Storer) Unsuspend(alias, domain string) error {
	var (
		stmt *sql.Stmt
		err  error
	)

	if s.readOnly {
		return errors.New("SQLite DB opened readonly")
	}

	stmt, err = s.db.Prepare(unsuspendAliasSQL)
	if err != nil {
		return err
	}

	_, err = stmt.Exec(
		alias,
		domain,
	)
	if err != nil {
		return err
	}
	return nil
}

// Delete removes the row from the database representing the alias. An error
// is returned if there is a failure interacting with database.
func (s Storer) Delete(alias, domain string) error {
	var (
		stmt *sql.Stmt
		err  error
	)

	if s.readOnly {
		return errors.New("SQLite DB opened readonly")
	}

	stmt, err = s.db.Prepare(deleteAliasSQL)
	if err != nil {
		return err
	}

	_, err = stmt.Exec(
		alias,
		domain,
	)
	if err != nil {
		return err
	}
	return nil
}
