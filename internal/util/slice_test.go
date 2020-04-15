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

package util

import (
	"math/rand"
	"testing"

	"github.com/stretchr/testify/assert"
)

func TestStringSliceEqual(t *testing.T) {
	tables := []struct {
		a     []string
		b     []string
		equal bool
	}{
		// Case 1: slice elements are equal
		{[]string{"a", "b", "c"}, []string{"a", "b", "c"}, true},
		// Case 2: slice elements are not equal (different length)
		{[]string{"a", "b", "c"}, []string{"a"}, false},
		// Case 3: slice elements are not equal (same length)
		{[]string{"a", "b", "c"}, []string{"c", "b", "a"}, false},
		// Case 4: slices are nil
		{nil, nil, true},
		// Case 5: slices are empty
		{[]string{}, []string{}, true},
	}

	for _, table := range tables {
		assert.Equal(t, table.equal, StringSliceEqual(table.a, table.b))
	}
}

func makeAlphaStringSlice() []string {
	alpha := []string{}
	// Generate a string slice of characters "A-Za-z"
	for i := 65; i <= 122; i++ {
		// A-Z is ASCII 65-90
		// a-z is ASCII 97-122
		if i > 90 && i < 97 {
			continue
		}
		alpha = append(alpha, string(byte(i)))
	}
	return alpha
}

func TestStringInSlice(t *testing.T) {
	alpha := makeAlphaStringSlice()

	tables := []struct {
		a     string
		b     []string
		equal bool
	}{
		// Case 1: string in slice
		{"a", alpha, true},
		// Case 2: string not in slice
		{"1", alpha, false},
		// Case 3: slice is nil
		{"", nil, false},
		// Case 4: slice is empty
		{"", []string{}, false},
	}

	for _, table := range tables {
		assert.Equal(t, table.equal, StringInSlice(table.a, table.b))
	}
}

func BenchmarkStringInSlice(b *testing.B) {
	alpha := makeAlphaStringSlice()

	for n := 0; n < b.N; n++ {
		// Make an ASCII character from
		// A-Z, [\]^`, a-z
		i := 65 + rand.Intn(57)
		StringInSlice(string(byte(i)), alpha)
	}
}
