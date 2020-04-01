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
	"sort"
	"testing"

	"github.com/stretchr/testify/assert"
)

func TestAliasesToMap(t *testing.T) {
	tables := []struct {
		in    Aliases
		out   AliasesMap
		equal bool
		msg   string
	}{
		// Case 1: Aliases is nil
		{
			in:    nil,
			out:   AliasesMap{},
			equal: true,
			msg:   "case 1",
		},
		// Case 2: Aliases is empty
		{
			in:    Aliases{},
			out:   AliasesMap{},
			equal: true,
			msg:   "case 2",
		},
		// Case 3: Alias(es) defined
		{
			in: Aliases{
				Alias{Alias: "foo", Domain: "bar.com"},
				Alias{Alias: "bar", Domain: "foo.com"},
			},
			out: AliasesMap{
				"foo@bar.com": Alias{Alias: "foo", Domain: "bar.com"},
				"bar@foo.com": Alias{Alias: "bar", Domain: "foo.com"},
			},
			equal: true,
			msg:   "case 3",
		},
		// Case 4: Alias(es) defined, not equal
		{
			in: Aliases{
				Alias{Alias: "foo", Domain: "bar.com"},
				Alias{Alias: "bar", Domain: "foo.com"},
			},
			out: AliasesMap{
				"foo@bar.com": Alias{Alias: "foo", Domain: "bar.com"},
				"baz@foo.com": Alias{Alias: "baz", Domain: "foo.com"},
			},
			equal: false,
			msg:   "case 4",
		},
	}

	for _, table := range tables {
		m := table.in.ToMap()
		if table.equal {
			assert.Equal(t, table.out, m, table.msg)
		} else {
			assert.NotEqual(t, table.out, m, table.msg)
		}
	}
}

func TestAliasesSort(t *testing.T) {
	tables := []struct {
		in    Aliases
		out   Aliases
		equal bool
		msg   string
	}{
		// Case 1: Both nil
		{
			in:    nil,
			out:   nil,
			equal: true,
			msg:   "case 1",
		},
		// Case 2: Both empty
		{
			in:    Aliases{},
			out:   Aliases{},
			equal: true,
			msg:   "case 2",
		},
		// Case 3: Equal
		{
			in: Aliases{
				Alias{Domain: "foo.com", Alias: "userB"},
				Alias{Domain: "bar.com", Alias: "userA"},
				Alias{Domain: "foo.com", Alias: "userA"},
			},
			out: Aliases{
				Alias{Domain: "bar.com", Alias: "userA"},
				Alias{Domain: "foo.com", Alias: "userA"},
				Alias{Domain: "foo.com", Alias: "userB"},
			},
			equal: true,
			msg:   "case 3",
		},
		// Case 4: not equal
		{
			in: Aliases{
				Alias{Domain: "foo.com", Alias: "userB"},
				Alias{Domain: "bar.com", Alias: "userA"},
				Alias{Domain: "foo.com", Alias: "userA"},
			},
			out: Aliases{
				Alias{Domain: "foo.com", Alias: "userB"},
				Alias{Domain: "bar.com", Alias: "userA"},
				Alias{Domain: "foo.com", Alias: "userA"},
			},
			equal: false,
			msg:   "case 4",
		},
	}

	for _, table := range tables {
		sort.Sort(table.in)

		if table.equal {
			assert.Equal(t, table.out, table.in, table.msg)
		} else {
			assert.NotEqual(t, table.out, table.in, table.msg)
		}
	}
}

func TestAliasesDiff(t *testing.T) {
	tables := []struct {
		s1, s2, diff Aliases
		equal        bool
		msg          string
	}{
		// Case 1: One or both nil
		{
			s1:    nil,
			s2:    nil,
			diff:  Aliases{},
			equal: true,
			msg:   "case 1",
		},
		// Case 2: One or both empty
		{
			s1:    Aliases{},
			s2:    Aliases{},
			diff:  Aliases{},
			equal: true,
			msg:   "case 2",
		},
		// Case 3: Difference is equal
		{
			s1: Aliases{
				Alias{Alias: "bar", Domain: "bar.com", Description: "Alias 1"},
				Alias{Alias: "baz", Domain: "baz.com", Description: "Alias 2"},
				Alias{Alias: "foo", Domain: "foo.com", Description: "Alias 3"},
			},
			s2: Aliases{
				Alias{Alias: "baz", Domain: "baz.com", Description: "Alias 2"},
				Alias{Alias: "foo", Domain: "foo.com", Description: "Different"},
			},
			diff: Aliases{
				Alias{Alias: "bar", Domain: "bar.com", Description: "Alias 1"},
				Alias{Alias: "foo", Domain: "foo.com", Description: "Alias 3"},
			},
			equal: true,
			msg:   "case 3",
		},
		// Case 4: Difference is not equal
		{
			s1: Aliases{
				Alias{Alias: "baz", Domain: "baz.com", Description: "Alias 2"},
				Alias{Alias: "foo", Domain: "foo.com", Description: "Alias 3"},
			},
			s2: Aliases{
				Alias{Alias: "baz", Domain: "baz.com", Description: "Alias 2"},
				Alias{Alias: "foo", Domain: "foo.com", Description: "Different"},
			},
			diff: Aliases{
				Alias{Alias: "bar", Domain: "bar.com", Description: "Alias 1"},
			},
			equal: false,
			msg:   "case 4",
		},
	}

	for _, table := range tables {
		d := table.s1.Diff(table.s2)

		if table.equal {
			assert.Equal(t, table.diff, d, table.msg)
		} else {
			assert.NotEqual(t, table.diff, d, table.msg)
		}
	}
}

func TestAliasesEqual(t *testing.T) {
	tables := []struct {
		s1, s2 Aliases
		equal  bool
		msg    string
	}{
		// Case 1: One or both nil
		{
			s1:    nil,
			s2:    nil,
			equal: false,
			msg:   "case 1",
		},
		// Case 2: Both empty
		{
			s1:    Aliases{},
			s2:    Aliases{},
			equal: true,
			msg:   "case 2",
		},
		// Case 3: Different lengths
		{
			s1: Aliases{
				Alias{Alias: "baz", Domain: "baz.com", Description: "Alias 2"},
			},
			s2:    Aliases{},
			equal: false,
			msg:   "case 3",
		},
		// Case 4: Equal
		{
			s1: Aliases{
				Alias{Alias: "baz", Domain: "baz.com", Description: "Alias 2"},
				Alias{Alias: "foo", Domain: "foo.com", Description: "Alias 3"},
			},
			s2: Aliases{
				Alias{Alias: "baz", Domain: "baz.com", Description: "Alias 2"},
				Alias{Alias: "foo", Domain: "foo.com", Description: "Alias 3"},
			},
			equal: true,
			msg:   "case 4",
		},
		// Case 5: Order matters
		{
			s1: Aliases{
				Alias{Alias: "baz", Domain: "baz.com", Description: "Alias 2"},
				Alias{Alias: "foo", Domain: "foo.com", Description: "Alias 3"},
			},
			s2: Aliases{
				Alias{Alias: "foo", Domain: "foo.com", Description: "Alias 3"},
				Alias{Alias: "baz", Domain: "baz.com", Description: "Alias 2"},
			},
			equal: false,
			msg:   "case 5",
		},
	}

	for _, table := range tables {
		if table.equal {
			assert.True(t, table.s1.Equal(table.s2), table.msg)
		} else {
			assert.False(t, table.s1.Equal(table.s2), table.msg)
		}
	}
}

func TestAliasesStripData(t *testing.T) {
	tables := []struct {
		in    Aliases
		names []string
		out   Aliases
		equal bool
		msg   string
	}{
		// Case 1: input is nil
		{
			in:    nil,
			out:   Aliases{},
			names: []string{},
			equal: true,
			msg:   "case 1",
		},
		// Case 2: input is empty
		{
			in:    Aliases{},
			out:   Aliases{},
			names: []string{},
			equal: true,
			msg:   "case 2",
		},
		// Case 3: stripped data matches
		{
			in: Aliases{
				Alias{Alias: "baz", Domain: "baz.com", Description: "Alias 2"},
				Alias{Alias: "foo", Domain: "foo.com", Description: "Alias 3"},
			},
			out: Aliases{
				Alias{Alias: "baz"},
				Alias{Alias: "foo"},
			},
			names: []string{"Alias"},
			equal: true,
			msg:   "case 3",
		},
		// Case 4: stripped data doesn't match
		{
			in: Aliases{
				Alias{Alias: "baz", Domain: "baz.com", Description: "Alias 2"},
				Alias{Alias: "foo", Domain: "foo.com", Description: "Alias 3"},
			},
			out: Aliases{
				Alias{Alias: "baz"},
				Alias{Alias: "foo"},
			},
			names: []string{"Alias", "Domain"},
			equal: false,
			msg:   "case 4",
		},
	}

	for _, table := range tables {
		a, err := table.in.StripData(table.names)
		assert.NoError(t, err)
		if table.equal {
			assert.Equal(t, table.out, a, table.msg)
		} else {
			assert.NotEqual(t, table.out, a, table.msg)
		}
	}
}

func TestAliasesMapToSlice(t *testing.T) {
	tables := []struct {
		in    AliasesMap
		out   Aliases
		equal bool
		msg   string
	}{
		// Case 1: input is nil
		{
			in:    nil,
			out:   Aliases{},
			equal: true,
			msg:   "case 1",
		},
		// Case 2: input is empty
		{
			in:    AliasesMap{},
			out:   Aliases{},
			equal: true,
			msg:   "case 2",
		},
		// Case 3: equal
		{
			in: AliasesMap{
				"foo@bar.com": Alias{Alias: "foo", Domain: "bar.com"},
				"bar@foo.com": Alias{Alias: "bar", Domain: "foo.com"},
			},
			out: Aliases{
				Alias{Alias: "foo", Domain: "bar.com"},
				Alias{Alias: "bar", Domain: "foo.com"},
			},
			equal: true,
			msg:   "case 3",
		},
		// Case 4: not equal
		{
			in: AliasesMap{
				"foo@bar.com": Alias{Alias: "foo", Domain: "bar.com"},
				"bar@foo.com": Alias{Alias: "bar", Domain: "foo.com"},
			},
			out: Aliases{
				Alias{Alias: "foo", Domain: "bar.com"},
			},
			equal: false,
			msg:   "case 4",
		},
		// Case 5: output not sorted
		{
			in: AliasesMap{
				"foo@bar.com": Alias{Alias: "foo", Domain: "bar.com"},
				"bar@foo.com": Alias{Alias: "bar", Domain: "foo.com"},
			},
			out: Aliases{
				Alias{Alias: "bar", Domain: "foo.com"},
				Alias{Alias: "foo", Domain: "bar.com"},
			},
			equal: false,
			msg:   "case 5",
		},
	}

	for _, table := range tables {
		s := table.in.ToSlice()
		if table.equal {
			assert.Equal(t, table.out, s, table.msg)
		} else {
			assert.NotEqual(t, table.out, s, table.msg)
		}
	}
}

func TestAliasesMapAdd(t *testing.T) {
	tables := []struct {
		in    AliasesMap
		alias Alias
		out   AliasesMap
		panic bool
		msg   string
	}{
		// Case 1: nil input
		{
			in:    nil,
			alias: Alias{},
			out:   nil,
			msg:   "case 1",
		},
		// Case 2: attempt to add when already exists
		{
			in: AliasesMap{
				"foo@bar.com": Alias{Alias: "foo", Domain: "bar.com"},
			},
			alias: Alias{Alias: "foo", Domain: "bar.com"},
			panic: true,
			msg:   "case 2",
		},
		// Case 3: successful add
		{
			in:    AliasesMap{},
			alias: Alias{Alias: "foo", Domain: "bar.com"},
			out: AliasesMap{
				"foo@bar.com": Alias{Alias: "foo", Domain: "bar.com"},
			},
			msg: "case 3",
		},
	}

	for _, table := range tables {
		if table.panic {
			assert.Panics(t, func() { table.in.Add(table.alias) }, table.msg)
		} else {
			table.in.Add(table.alias)
			assert.Equal(t, table.out, table.in, table.msg)
		}
	}
}

func TestAliasesMapDel(t *testing.T) {
	tables := []struct {
		in    AliasesMap
		alias Alias
		out   AliasesMap
		msg   string
	}{
		// Case 1: nil input
		{
			in:    nil,
			alias: Alias{},
			out:   nil,
			msg:   "case 1",
		},
		// Case 2: attempt to delete an alias when it doesn't exist
		{
			in: AliasesMap{
				"foo@baz.com": Alias{Alias: "foo", Domain: "baz.com"},
			},
			alias: Alias{Alias: "foo", Domain: "bar.com"},
			out: AliasesMap{
				"foo@baz.com": Alias{Alias: "foo", Domain: "baz.com"},
			},
			msg: "case 2",
		},
		// Case 3: successful delete
		{
			in: AliasesMap{
				"foo@bar.com": Alias{Alias: "foo", Domain: "bar.com"},
			},
			alias: Alias{Alias: "foo", Domain: "bar.com"},
			out:   AliasesMap{},
			msg:   "case 3",
		},
	}

	for _, table := range tables {
		table.in.Del(table.alias)
		assert.Equal(t, table.out, table.in, table.msg)
	}
}
