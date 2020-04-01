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
	"testing"
	"time"

	"github.com/DATA-DOG/go-sqlmock"
	"github.com/stretchr/testify/assert"
)

var (
	sqlite   *Storer
	t1       time.Time = time.Date(2001, 9, 11, 12, 46, 40, 0, time.UTC)
	t2       time.Time = time.Date(2001, 9, 11, 13, 3, 0, 0, time.UTC)
	zeroTime time.Time
)

func setupSQLiteTest(t *testing.T) sqlmock.Sqlmock {
	var (
		mock sqlmock.Sqlmock
		err  error
	)

	sqlite = &Storer{}

	sqlite.db, mock, err = sqlmock.New()
	if err != nil {
		t.Fatalf("an error '%s' was not expected when opening a stub database connection", err)
	}

	return mock
}

func teardownSQLiteTest() {
	sqlite.Close()
}

func makeTestSQLiteRows(v int) *sqlmock.Rows {
	rows := sqlmock.NewRows([]string{
		"alias",
		"domain",
		"addresses",
		"description",
		"suspended",
		"created_ts",
		"modified_ts",
		"suspended_ts",
	})

	if (v & 1) == 1 {
		rows = rows.AddRow("foo", "baz.com", "null@null.com", "test 1", false, t1, t1, zeroTime)
	}
	if (v & 2) == 2 {
		rows = rows.AddRow("bar", "baz.com", "null@null.com,nil@null.com", "test 2", false, t2, t2, zeroTime)
	}
	if (v & 4) == 4 {
		rows = rows.AddRow("qux", "baz.com", "nil@null.com", "test 3", true, t1, t2, t2)
	}

	return rows
}

func TestSQLiteAliasToAlias(t *testing.T) {
	sa := Alias{
		alias:          "foo",
		domain:         "baz.com",
		emailAddresses: "null@null.com,nil@null.com",
		description:    "testing",
		suspended:      false,
		createdTS:      sql.NullTime{Time: t1, Valid: true},
		modifiedTS:     sql.NullTime{Time: t2, Valid: true},
		suspendedTS:    sql.NullTime{Time: zeroTime, Valid: true},
	}

	a := sa.toAlias()
	// Make sure the assignments from sqliteAlias to Alias is correct
	assert.Equal(t, "foo", a.Alias)
	assert.Equal(t, "baz.com", a.Domain)
	assert.ElementsMatch(t, []string{"null@null.com", "nil@null.com"}, a.EmailAddresses)
	assert.Equal(t, "testing", a.Description)
	assert.False(t, a.Suspended)
	assert.Equal(t, t1, a.CreatedTS)
	assert.Equal(t, t2, a.ModifiedTS)
	assert.Equal(t, zeroTime, a.SuspendedTS)
}

func TestSQLiteGet(t *testing.T) {
	mock := setupSQLiteTest(t)
	rows := makeTestSQLiteRows(1)

	mock.ExpectQuery("^SELECT (.+) FROM alias WHERE alias = (.+) AND domain = (.+)$").
		WithArgs("foo", "baz.com").
		WillReturnRows(rows)

	_, err := sqlite.Get("foo", "baz.com")
	if err != nil {
		t.Errorf("unexepected error from sqlite.Get() call: %v", err)
	}

	// we make sure that all expectations were met
	if err := mock.ExpectationsWereMet(); err != nil {
		t.Errorf("there were unfulfilled expectations: %s", err)
	}

	teardownSQLiteTest()
}
