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
	"fmt"
	"io"
	"os"
	"reflect"
	"strings"
	"time"

	"github.com/jedib0t/go-pretty/table"
	"github.com/jedib0t/go-pretty/text"
)

func (t *Table) header(a Alias) table.Row {
	aliasType := reflect.TypeOf(a)
	headers := table.Row{}

	for _, fieldName := range t.columns {
		if field, ok := aliasType.FieldByName(fieldName); ok {
			column := field.Tag.Get("column")
			if column != "" {
				headers = append(headers, column)
			} else {
				headers = append(headers, fieldName)
			}
		}
	}

	return headers
}

func (t *Table) row(a Alias) table.Row {
	v := reflect.ValueOf(a)
	row := table.Row{}

	for _, fieldName := range t.columns {
		value := v.FieldByName(fieldName)

		switch value.Kind() {
		case reflect.Bool:
			v := func() string {
				if value.Bool() {
					return ("Yes")
				}
				return ("No")
			}()
			row = append(row, v)
		case reflect.String:
			row = append(row, value.String())
		case reflect.Struct:
			var (
				timeStruct time.Time
				ok         bool
			)

			timeStruct, ok = value.Interface().(time.Time)
			if ok {
				row = append(row, TStoString(timeStruct))
				break
			}
			// Catch-all
			row = append(row, fmt.Sprintf("%v", value.Interface()))
		case reflect.Slice:
			var (
				stringSlice []string
				ok          bool
			)

			stringSlice, ok = value.Interface().([]string)
			if ok {
				row = append(row, strings.Join(stringSlice, ", "))
				break
			}
			// Catch-all
			row = append(row, fmt.Sprintf("%v", value.Interface()))
		default:
			v := fmt.Sprintf("%v", value.Interface())
			row = append(row, v)
		}
	}
	return row
}

// Render outputs a table of aliases.
func (t *Table) Render() {
	tw := table.NewWriter()
	tw.SetOutputMirror(t.writer)
	tw.SetAutoIndex(t.numbers)
	tw.SetStyle(table.StyleLight)
	tw.Style().Format.Header = text.FormatDefault
	firstRow := true

	for _, a := range t.aliases {
		if t.headers && firstRow {
			headers := t.header(a)
			tw.AppendHeader(headers)
			firstRow = false
		}

		row := t.row(a)
		tw.AppendRow(row)
	}
	tw.Render()
}

// NewTable returns a new *Table instance.
func NewTable(options ...func(*Table) error) (*Table, error) {
	fields := Alias{}.Fields()
	t := Table{
		writer:  os.Stdout,
		columns: fields,
		headers: true,
		numbers: true,
	}

	for _, opt := range options {
		if err := opt(&t); err != nil {
			return nil, err
		}
	}

	return &t, nil
}

// SetTableWriter is a Table option for setting the writer.
func SetTableWriter(w io.Writer) func(*Table) error {
	return func(t *Table) error {
		t.writer = w
		return nil
	}
}

// SetAliases is a Table option for setting the aliases.
func SetAliases(a Aliases) func(*Table) error {
	return func(t *Table) error {
		t.aliases = a
		return nil
	}
}

// SetColumns is a Table option for setting the columns to display.
func SetColumns(c []string) func(*Table) error {
	return func(t *Table) error {
		t.columns = c
		return nil
	}
}

// SetHeaders is a Table option for setting the option to display headers.
func SetHeaders(h bool) func(*Table) error {
	return func(t *Table) error {
		t.headers = h
		return nil
	}
}

// SetNumbers is a Table option for setting the option to display numbers.
func SetNumbers(n bool) func(*Table) error {
	return func(t *Table) error {
		t.numbers = n
		return nil
	}
}
