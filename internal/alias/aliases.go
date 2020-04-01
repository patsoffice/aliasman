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
	"sort"
)

// ToMap transforms an Aliases type to an AliasesMap type.
func (s Aliases) ToMap() AliasesMap {
	m := AliasesMap{}
	for _, a := range s {
		m[a.Key()] = a
	}
	return m
}

// Diff is the difference between two slices of Alias. A slice of Alias
// of the elements not in common are returned and are sorted.
func (s Aliases) Diff(s2 Aliases) Aliases {
	diff := Aliases{}

	m1 := s.ToMap()
	m2 := s2.ToMap()
	for k, a := range m1 {
		if _, ok := m2[k]; !ok {
			diff = append(diff, a)
		} else {
			if ok := a.Equal(m2[k]); !ok {
				diff = append(diff, a)
			}
		}
	}
	sort.Sort(diff)
	return diff
}

// Equal checks for equality between two Aliases types.
func (s Aliases) Equal(s2 Aliases) bool {
	if s == nil || s2 == nil {
		return false
	}
	if len(s) != len(s2) {
		return false
	}

	for i, a := range s {
		if ok := a.Equal(s2[i]); !ok {
			return false
		}
	}
	return true
}

// StripData takes a list for field names as a string slice. Any field name
// not present in the string slice has it's field value set to the zero
// value. This is applied to all elements of the Aliases type. The stripped
// data is returned as well as an error if there was a problem setting a
// field value.
func (s Aliases) StripData(names []string) (Aliases, error) {
	dest := make(Aliases, len(s))
	for i, a := range s {
		alias, err := a.StripData(names)
		if err != nil {
			return nil, err
		}
		dest[i] = alias
	}
	return dest, nil
}

// ToSlice transforms an AliasesMap type to an Aliases type. The returned
// Aliases type is sorted.
func (m AliasesMap) ToSlice() Aliases {
	s := make(Aliases, len(m))

	i := 0
	for _, a := range m {
		s[i] = a
		i++
	}
	sort.Sort(s)
	return s
}

// Add adds an Alias to an AliasMap type.
func (m AliasesMap) Add(a Alias) {
	if m == nil {
		return
	}
	k := a.Key()
	_, ok := m[k]
	if ok {
		panic(fmt.Sprintf("key %s already exists in AliasesMap", k))
	}
	m[k] = a
}

// Del removes an Alias from an AliasMap type.
func (m AliasesMap) Del(a Alias) {
	if m == nil {
		return
	}
	k := a.Key()
	if _, ok := m[k]; ok {
		delete(m, k)
	}
}
