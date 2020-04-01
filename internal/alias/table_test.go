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

package alias

import (
	"bytes"
	"testing"
	"time"

	"github.com/jedib0t/go-pretty/table"
	"github.com/stretchr/testify/assert"
)

func TestHeader(t *testing.T) {
	a := Alias{
		Alias:          "foo",
		Domain:         "bar.com",
		EmailAddresses: []string{"baz@baz.com"},
		Description:    "description",
		Suspended:      false,
		CreatedTS:      time.Date(2001, 9, 11, 12, 46, 40, 0, time.UTC),
		ModifiedTS:     time.Date(2001, 9, 11, 13, 3, 0, 0, time.UTC),
		SuspendedTS:    time.Time{},
	}
	testTables := []struct {
		tableStruct Table
		alias       Alias
		out         table.Row
	}{
		// Case 1, no columns specified
		{
			tableStruct: Table{},
			alias:       a,
			out:         table.Row{},
		},
		// Case 2, specified columns that have both a struct tag and dont
		{
			tableStruct: Table{numbers: true, columns: []string{"Alias", "Domain", "CreatedTS"}},
			alias:       a,
			out:         table.Row{"Alias", "Domain", "Created Time"},
		},
	}

	for _, testTable := range testTables {
		row := testTable.tableStruct.header(a)
		for i, v := range testTable.out {
			assert.Equal(t, v, row[i])
		}
	}
}

func TestRow(t *testing.T) {
	a := Alias{
		Alias:          "foo",
		Domain:         "bar.com",
		EmailAddresses: []string{"baz@baz.com", "dup@baz.com"},
		Description:    "description",
		Suspended:      false,
		CreatedTS:      time.Date(2001, 9, 11, 12, 46, 40, 0, time.UTC),
		ModifiedTS:     time.Date(2001, 9, 11, 13, 3, 0, 0, time.UTC),
		SuspendedTS:    time.Time{},
	}
	testTables := []struct {
		tableStruct Table
		alias       Alias
		out         table.Row
	}{
		// Case 1, no columns specified
		{
			tableStruct: Table{},
			alias:       a,
			out:         table.Row{},
		},
		// Case 2, specified columns that have both a struct tag and dont
		{
			tableStruct: Table{numbers: true, columns: []string{"Alias", "Domain", "EmailAddresses", "Suspended", "CreatedTS"}},
			alias:       a,
			out:         table.Row{"foo", "bar.com", "baz@baz.com, dup@baz.com", "No", "2001-09-11T12:46:40Z"},
		},
	}

	for _, testTable := range testTables {
		row := testTable.tableStruct.row(a)
		for i, v := range testTable.out {
			assert.Equal(t, v, row[i])
		}
	}
}

func TestSetTableWriter(t *testing.T) {
	b := &bytes.Buffer{}
	tw, err := NewTable(SetTableWriter(b))
	assert.NoError(t, err)
	assert.Equal(t, b, tw.writer)
}

func TestSetAliases(t *testing.T) {
	a := Aliases{}
	tw, err := NewTable(SetAliases(a))
	assert.NoError(t, err)
	assert.Equal(t, a, tw.aliases)
}

func TestSetColumns(t *testing.T) {
	c := []string{"a", "b", "c"}
	tw, err := NewTable(SetColumns(c))
	assert.NoError(t, err)
	for i, v := range c {
		assert.Equal(t, v, tw.columns[i])
	}
}

func TestSetHeaders(t *testing.T) {
	tw, err := NewTable(SetHeaders(true))
	assert.NoError(t, err)
	assert.True(t, tw.headers)
}

func TestSetNumbers(t *testing.T) {
	tw, err := NewTable(SetNumbers(true))
	assert.NoError(t, err)
	assert.True(t, tw.numbers)
}
