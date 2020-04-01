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

package s3

import (
	"testing"
	"time"

	"github.com/stretchr/testify/assert"
)

func TestMetadataTimeMarshalJSON(t *testing.T) {
	tables := []struct {
		in  time.Time
		out []byte
	}{
		{time.Date(2001, 9, 11, 12, 46, 40, 0, time.UTC), []byte("\"2001-09-11T12:46:40Z\"")},
		{time.Date(2001, 9, 11, 13, 3, 0, 0, time.UTC), []byte("\"2001-09-11T13:03:00Z\"")},
	}

	for _, table := range tables {
		out, _ := metadataTime(table.in).MarshalJSON()
		assert.ElementsMatch(t, table.out, out)
	}
}

func TestMetadataTimeUnmarshalJSON(t *testing.T) {
	tables := []struct {
		in  []byte
		out metadataTime
	}{
		// Case 1
		{[]byte("\"2001-09-11T12:46:40Z\""), metadataTime(time.Date(2001, 9, 11, 12, 46, 40, 0, time.UTC))},
		// Case 2 -- results in empty metadataTime
		{[]byte("\"invalid-time-stamp\""), metadataTime{}},
	}

	for _, table := range tables {
		out := metadataTime{}
		out.UnmarshalJSON(table.in)

		assert.True(t, time.Time(table.out).Equal(time.Time(out)))
	}
}
