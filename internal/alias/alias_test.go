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
	"math/rand"
	"regexp"
	"testing"
	"time"

	"github.com/stretchr/testify/assert"
)

const letterBytes = "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ"

func TestAliasEqual(t *testing.T) {
	now := time.Now()
	later := now.Add(time.Hour)

	tables := []struct {
		a     Alias
		b     Alias
		equal bool
	}{
		// Case 1: Alias equality
		{Alias{Alias: "foo"}, Alias{Alias: "foo"}, true},
		{Alias{Alias: "foo"}, Alias{Alias: "bar"}, false},
		// Case 2: Domain equality
		{Alias{Domain: "foo.com"}, Alias{Domain: "foo.com"}, true},
		{Alias{Domain: "foo.com"}, Alias{Domain: "bar.com"}, false},
		// Case 3: Description equality
		{Alias{Description: "foo service"}, Alias{Description: "foo service"}, true},
		{Alias{Description: "foo service"}, Alias{Description: "bar service"}, false},
		// Case 4: Suspended equality
		{Alias{Suspended: false}, Alias{Suspended: false}, true},
		{Alias{Suspended: false}, Alias{Suspended: true}, false},
		// Case 5: CreatedTS equality
		{Alias{CreatedTS: now}, Alias{CreatedTS: now}, true},
		{Alias{CreatedTS: now}, Alias{CreatedTS: later}, false},
		// Case 6: ModifiedTS equality
		{Alias{ModifiedTS: now}, Alias{ModifiedTS: now}, true},
		{Alias{ModifiedTS: now}, Alias{ModifiedTS: later}, false},
		// Case 7: SuspendedTS equality
		{Alias{SuspendedTS: now}, Alias{SuspendedTS: now}, true},
		{Alias{SuspendedTS: now}, Alias{SuspendedTS: later}, false},
		// Case 8: EmailAddresses equality
		{Alias{EmailAddresses: []string{"a@foo.com", "b@foo.com"}}, Alias{EmailAddresses: []string{"a@foo.com", "b@foo.com"}}, true},
		{Alias{EmailAddresses: []string{"a@foo.com", "b@foo.com"}}, Alias{EmailAddresses: []string{"a@foo.com", "c@foo.com"}}, false},
	}

	for _, table := range tables {
		ok := table.a.Equal(table.b)
		assert.Equal(t, table.equal, ok, table.a.UnifiedDiff(table.b))
	}
}

func randStringBytes(n int) string {
	b := make([]byte, n)
	for i := range b {
		b[i] = letterBytes[rand.Intn(len(letterBytes))]
	}
	return string(b)
}

func randStringSlice(n, m int) []string {
	s := make([]string, n)
	for i := 0; i < n; i++ {
		s[i] = randStringBytes(m)
	}
	return s
}

func randBool() bool {
	if rand.Intn(2) > 0 {
		return true
	}
	return false
}

func randTime() time.Time {
	t := time.Date(
		rand.Intn(3000),
		time.Month(rand.Intn(12)+1),
		rand.Intn(27)+1,
		rand.Intn(24),
		rand.Intn(60),
		rand.Intn(60),
		rand.Intn(1000),
		time.UTC,
	)
	return t
}

func makeRandomAlias() Alias {
	a := Alias{
		Alias:          randStringBytes(10),
		Domain:         randStringBytes(10),
		Description:    randStringBytes(10),
		EmailAddresses: randStringSlice(10, 10),
		Suspended:      randBool(),
		CreatedTS:      randTime(),
		ModifiedTS:     randTime(),
		SuspendedTS:    randTime(),
	}
	return a
}

func BenchmarkAliasEqual(b *testing.B) {
	a := make([]Alias, 20)
	for i := 0; i < 20; i++ {
		a[i] = makeRandomAlias()
	}
	for n := 0; n < b.N; n++ {
		i := rand.Intn(20)
		j := rand.Intn(20)
		a[i].Equal(a[j])
	}
}

func TestAliasFields(t *testing.T) {
	expected := []string{"Alias", "Domain", "EmailAddresses", "Description", "Suspended", "CreatedTS", "ModifiedTS", "SuspendedTS"}
	fields := Alias{}.Fields()

	assert.Len(t, fields, len(expected))
	for i, v := range expected {
		assert.Equal(t, v, fields[i])
	}
}

func BenchmarkAliasFields(b *testing.B) {
	a := Alias{}
	for n := 0; n < b.N; n++ {
		a.Fields()
	}
}

func TestAliasStripData(t *testing.T) {
	tables := []struct {
		in  Alias
		out Alias
	}{
		{
			in: Alias{
				Alias:          "foo",
				Domain:         "bar.com",
				EmailAddresses: []string{"bar@baz.com"},
				Suspended:      true,
				Description:    "A fake alias",
				CreatedTS:      time.Now(),
				ModifiedTS:     time.Now(),
				SuspendedTS:    time.Now(),
			},
			out: Alias{
				Alias:          "foo",
				Domain:         "bar.com",
				EmailAddresses: []string{"bar@baz.com"},
			},
		},
	}

	for _, table := range tables {
		a, err := table.in.StripData([]string{"Alias", "Domain", "EmailAddresses"})
		assert.NoError(t, err)
		ok := table.out.Equal(a)
		assert.True(t, ok, err)
	}
}

func BenchmarkAliasStripData(b *testing.B) {
	a := Alias{
		Alias:          "foo",
		Domain:         "bar.com",
		EmailAddresses: []string{"bar@baz.com"},
		Suspended:      true,
		Description:    "A fake alias",
		CreatedTS:      time.Now(),
		ModifiedTS:     time.Now(),
		SuspendedTS:    time.Now(),
	}
	for n := 0; n < b.N; n++ {
		a.StripData([]string{"Alias", "Domain", "EmailAddresses"})
	}
}

func TestAliasFilter(t *testing.T) {
	tables := []struct {
		in             Alias
		filter         Filter
		matchAnyRegexp bool
		out            bool
		msg            string
	}{
		// Case: Alias found
		{
			in:     Alias{Alias: "search term"},
			filter: Filter{Alias: regexp.MustCompile("^search term$")},
			out:    true,
			msg:    "Alias regexp found",
		},
		// Case: Alias not found
		{
			in:     Alias{Alias: "missing search term"},
			filter: Filter{Alias: regexp.MustCompile("^search term$")},
			out:    false,
			msg:    "Alias regexp not found",
		},
		// Case: Domain found
		{
			in:     Alias{Domain: "search term"},
			filter: Filter{Domain: regexp.MustCompile("^search term$")},
			out:    true,
			msg:    "Domain regexp found",
		},
		// Case: Domain not found
		{
			in:     Alias{Domain: "missing search term"},
			filter: Filter{Domain: regexp.MustCompile("^search term$")},
			out:    false,
			msg:    "Domain regexp not found",
		},
		// Case: EmailAddresses found
		{
			in:     Alias{EmailAddresses: []string{"search term"}},
			filter: Filter{EmailAddress: regexp.MustCompile("^search term$")},
			out:    true,
			msg:    "Email addresses regexp found",
		},
		// Case: EmailAddresses not found
		{
			in:     Alias{EmailAddresses: []string{"missing search term"}},
			filter: Filter{EmailAddress: regexp.MustCompile("^search term$")},
			out:    false,
			msg:    "Email addresses regexp not found",
		},
		// Case: Description found
		{
			in:     Alias{Description: "search term"},
			filter: Filter{Description: regexp.MustCompile("^search term$")},
			out:    true,
			msg:    "Description regexp found",
		},
		// Case: Description not found
		{
			in:     Alias{Description: "missing search term"},
			filter: Filter{Description: regexp.MustCompile("^search term$")},
			out:    false,
			msg:    "Description regexp not found",
		},
		// Case: Exclude suspended, alias Suspended set
		{
			in:     Alias{Suspended: true},
			filter: Filter{ExcludeSuspended: true},
			out:    false,
			msg:    "Exclude suspended, alias suspended",
		},
		// Case: Exclude suspended not set, alias Suspended set
		{
			in:     Alias{Suspended: true},
			filter: Filter{},
			out:    true,
			msg:    "Alias suspended",
		},
		// Case: Exclude enabled
		{
			in:     Alias{},
			filter: Filter{ExcludeEnabled: true},
			out:    false,
			msg:    "Exclude enabled",
		},
		// Case: Match any field
		{
			in: Alias{
				Alias:          "missing search term",
				Domain:         "missing search term",
				EmailAddresses: []string{"search term"},
				Description:    "missing search term",
			},
			filter: Filter{
				Alias:        regexp.MustCompile("^search term$"),
				Domain:       regexp.MustCompile("^search term$"),
				EmailAddress: regexp.MustCompile("^search term$"),
				Description:  regexp.MustCompile("^search term$"),
			},
			matchAnyRegexp: true,
			out:            true,
			msg:            "Match any field",
		},
	}

	for _, table := range tables {
		assert.Equal(t, table.out, table.in.Filter(table.filter, table.matchAnyRegexp), table.msg)
	}
}

func TestTimeStampToString(t *testing.T) {
	tables := []struct {
		in  time.Time
		out string
	}{
		{time.Date(2001, 9, 11, 12, 46, 40, 0, time.UTC), "2001-09-11T12:46:40Z"},
		{time.Time{}, ""},
	}

	for _, table := range tables {
		assert.Equal(t, table.out, TStoString(table.in))
	}
}

func BenchmarkTStoStringZero(b *testing.B) {
	t := time.Time{}
	for n := 0; n < b.N; n++ {
		TStoString(t)
	}
}

func BenchmarkTStoStringNonZero(b *testing.B) {
	t := time.Date(2001, 9, 11, 12, 46, 40, 0, time.UTC)
	for n := 0; n < b.N; n++ {
		TStoString(t)
	}
}
