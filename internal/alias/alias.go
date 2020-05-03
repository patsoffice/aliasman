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
	"reflect"
	"time"

	"github.com/davecgh/go-spew/spew"
	"github.com/patsoffice/toolbox"
	"github.com/pmezard/go-difflib/difflib"
)

// Equal checks for equality between two Alias types. All fields are checked
// for equality and all of them must match. A true value is returned when all
// fields match. When a field does not match, an error specifying the
// mismatched field is returned.
func (a Alias) Equal(b Alias) bool {
	if a.Alias != b.Alias || a.Domain != b.Domain || a.Description != b.Description || a.Suspended != b.Suspended {
		return false
	}
	if !a.CreatedTS.Equal(b.CreatedTS) || !a.ModifiedTS.Equal(b.ModifiedTS) || !a.SuspendedTS.Equal(b.SuspendedTS) {
		return false
	}
	if !toolbox.StringSliceEqual(a.EmailAddresses, b.EmailAddresses) {
		return false
	}
	return true
}

var spewConfig = spew.ConfigState{
	Indent:                  " ",
	DisablePointerAddresses: true,
	DisableCapacities:       true,
	SortKeys:                true,
	DisableMethods:          true,
}

// UnifiedDiff returns a unified text diff of both Alias(es). Otherwise it
// returns an empty string.
func (a Alias) UnifiedDiff(b Alias) string {
	et := reflect.TypeOf(a)

	var expected, actual string
	if et != reflect.TypeOf("") {
		expected = spewConfig.Sdump(a)
		actual = spewConfig.Sdump(b)
	} else {
		expected = reflect.ValueOf(a).String()
		actual = reflect.ValueOf(b).String()
	}

	diff, _ := difflib.GetUnifiedDiffString(difflib.UnifiedDiff{
		A:        difflib.SplitLines(expected),
		B:        difflib.SplitLines(actual),
		FromFile: "Expected",
		ToFile:   "Actual",
		Context:  1,
	})

	return "\n\nDiff:\n" + diff
}

// Key returns a key name for maps based on the Alias and Domain fields which
// should be unique.
func (a Alias) Key() string {
	return fmt.Sprintf("%s@%s", a.Alias, a.Domain)
}

func (s Aliases) Len() int { return len(s) }

func (s Aliases) Swap(i, j int) { s[i], s[j] = s[j], s[i] }

func (s Aliases) Less(i, j int) bool {
	if s[i].Domain < s[j].Domain {
		return true
	} else if s[i].Domain > s[j].Domain {
		return false
	} else {
		return s[i].Alias < s[j].Alias
	}
}

// Fields returns a slice of strings composed of the field names of the
// Alias struct.
func (a Alias) Fields() []string {
	fields := []string{}

	t := reflect.TypeOf(a)
	for i := 0; i < t.NumField(); i++ {
		f := t.Field(i)
		fields = append(fields, f.Name)
	}
	return fields
}

// StripData takes a list for field names as a string slice. Any field name
// not present in the string slice has it's field value set to the zero
// value. An Alias with all the specified values still set is returned
// as well as an error if there was a problem setting a field value.
func (a Alias) StripData(fields []string) (Alias, error) {
	s := Alias{}

	srcValue := reflect.ValueOf(&a).Elem()
	destValue := reflect.ValueOf(&s).Elem()

	for _, name := range fields {
		value := srcValue.FieldByName(name)
		field := destValue.FieldByName(name)
		if field.CanSet() {
			field.Set(value)
		} else {
			err := fmt.Errorf("cannot set field %s: %v", name, value)
			return Alias{}, err
		}
	}

	return s, nil
}

// Filter applies the fields in a Filter struct against the fields on an
// Alias struct. If all Alias fields match against the Filter struct,
// then true is returned. Otherwise, false is returned.
func (a Alias) Filter(filter Filter, matchAnyRegexp bool) bool {
	foundAlias := true
	foundDomain := true
	foundEmailAddress := true
	foundDescription := true
	includeEnabled := true
	includeSuspended := true

	if filter.Alias != nil && !filter.Alias.MatchString(a.Alias) {
		foundAlias = false
	}
	if filter.Domain != nil && !filter.Domain.MatchString(a.Domain) {
		foundDomain = false
	}
	if filter.EmailAddress != nil {
		matchAny := false
		for _, emailAddress := range a.EmailAddresses {
			matchAny = matchAny || filter.EmailAddress.MatchString(emailAddress)
		}
		foundEmailAddress = matchAny
	}
	if filter.Description != nil && !filter.Description.MatchString(a.Description) {
		foundDescription = false
	}
	if filter.ExcludeSuspended && a.Suspended {
		includeSuspended = false
	}
	if filter.ExcludeEnabled && !a.Suspended {
		includeEnabled = false
	}
	if matchAnyRegexp {
		return ((foundAlias ||
			foundDomain ||
			foundEmailAddress ||
			foundDescription) &&
			includeSuspended &&
			includeEnabled)
	}
	return (foundAlias &&
		foundDomain &&
		foundEmailAddress &&
		foundDescription &&
		includeSuspended &&
		includeEnabled)
}

// TStoString converts a timestamp to RFC3339 format string. If the time is
// the zero time, it is rendered as an empty string. The string is returned.
func TStoString(ts time.Time) string {
	if ts.IsZero() {
		return ""
	}
	return ts.Format(time.RFC3339)
}
