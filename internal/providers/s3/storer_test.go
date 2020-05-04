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
	"errors"
	"sort"
	"testing"
	"time"

	"github.com/aws/aws-sdk-go/aws"
	"github.com/aws/aws-sdk-go/aws/awserr"
	"github.com/aws/aws-sdk-go/service/s3"
	"github.com/aws/aws-sdk-go/service/s3/s3iface"
	toolbox "github.com/patsoffice/go.toolbox"
	"github.com/stretchr/testify/assert"
)

func TestSortObjecs(t *testing.T) {
	t1 := time.Date(2001, 9, 11, 12, 46, 40, 0, time.UTC)
	t2 := time.Date(2001, 9, 11, 13, 3, 0, 0, time.UTC)
	t3 := t2.Add(time.Hour)

	tables := []struct {
		in  objects
		out objects
	}{
		{
			in: []*s3.Object{
				{
					Key:          aws.String("foo"),
					LastModified: &t3,
				},
				{
					Key:          aws.String("bar"),
					LastModified: &t2,
				},
				{
					Key:          aws.String("baz"),
					LastModified: &t1,
				},
			},
			out: []*s3.Object{
				{
					Key:          aws.String("baz"),
					LastModified: &t1,
				},
				{
					Key:          aws.String("bar"),
					LastModified: &t2,
				},
				{
					Key:          aws.String("foo"),
					LastModified: &t3,
				},
			},
		},
	}

	for _, table := range tables {
		sort.Sort(table.in)
		for i := range table.in {
			assert.Equal(t, *table.out[i].Key, *table.in[i].Key)
		}
	}
}

type mockListObjectsV2Pages struct {
	s3iface.S3API
	Resp  []s3.ListObjectsV2Output
	Error error
}

func (m mockListObjectsV2Pages) ListObjectsV2Pages(input *s3.ListObjectsV2Input, fn func(*s3.ListObjectsV2Output, bool) bool) error {
	if m.Error != nil {
		return m.Error
	}

	l := len(m.Resp)
	if l > 0 {
		l--
	}
	for i, r := range m.Resp {
		lastPage := i == l
		res := fn(&r, lastPage)
		if !res {
			break
		}
	}
	return nil
}

func TestListKeys(t *testing.T) {
	tables := []struct {
		Resp        []s3.ListObjectsV2Output
		ObjectCount int
		Error       error
	}{
		{ // Case 1
			Resp: []s3.ListObjectsV2Output{
				{Contents: []*s3.Object{{}, {}}},
				{Contents: []*s3.Object{{}, {}, {}}},
			},
			ObjectCount: 5,
			Error:       nil,
		},
		{ // Case 2 -- NoSuchBucket
			Error: awserr.New(s3.ErrCodeNoSuchBucket, "Handled AWS error", errors.New("Go error")),
		},
		{ // Case 3 -- NoSuchKey (which ListObjectsV2 should never throw?)
			Error: awserr.New(s3.ErrCodeNoSuchKey, "Generic AWS error", errors.New("Go error")),
		},
		{ // Case 4 -- Just a generic Go error
			Error: errors.New("Not an AWS error"),
		},
	}

	for _, table := range tables {
		s3a := Storer{
			svc: mockListObjectsV2Pages{
				Resp:  table.Resp,
				Error: table.Error,
			},
		}
		err := s3a.listKeys()
		assert.Equal(t, table.Error, err)
		assert.Equal(t, table.ObjectCount, len(s3a.objects))
	}
}

func TestParseTS(t *testing.T) {
	tables := []struct {
		metadata map[string]*string
		key      string
		expected time.Time
	}{
		{
			// Case 1 -- good timestamp
			metadata: map[string]*string{"Created_ts": aws.String("2001-09-11T13:03:00Z")},
			key:      "Created_ts",
			expected: time.Date(2001, 9, 11, 13, 3, 0, 0, time.UTC),
		},
		{
			// Case 2 -- missing timestamp
			metadata: map[string]*string{},
			key:      "Created_ts",
			expected: time.Date(2001, 9, 11, 12, 46, 40, 0, time.UTC), // What MockClock.Now() returns
		},
		{
			// Case 3 -- bad timestamp
			metadata: map[string]*string{"Created_ts": aws.String("not-a-timestamp")},
			key:      "Created_ts",
			expected: time.Date(2001, 9, 11, 12, 46, 40, 0, time.UTC), // What MockClock.Now() returns
		},
	}

	for _, table := range tables {
		s := Storer{
			clock: toolbox.MockClock{NowTime: table.expected},
		}

		ts := s.parseTS(table.metadata, table.key)
		assert.Equal(t, table.expected, ts)
	}
}

type mockHeadObject struct {
	s3iface.S3API
	Resp  []s3.HeadObjectOutput
	Error error
}

func TestHeadAlias(t *testing.T) {
	// tables := []struct {
	// 	Resp        []s3.ListObjectsV2Output
	// 	ObjectCount int
	// 	Error       error
	// }{}

	// for _, _ in range tables {

	// }
}
