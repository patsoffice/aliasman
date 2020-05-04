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

package files

import (
	"io/ioutil"
	"math/rand"
	"os"
	"path/filepath"
	"testing"
	"time"

	"github.com/patsoffice/aliasman/internal/alias"
	toolbox "github.com/patsoffice/go.toolbox"
	"github.com/stretchr/testify/assert"
)

const letterBytes = "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ"

var (
	t1       time.Time = time.Date(2001, 9, 11, 12, 46, 40, 0, time.UTC)
	t2       time.Time = time.Date(2001, 9, 11, 13, 3, 0, 0, time.UTC)
	zeroTime time.Time
)

func setupFilesTest() (*Storer, error) {
	tempDir, err := ioutil.TempDir("", "")
	if err != nil {
		return nil, err
	}
	s := Storer{
		filesPath: tempDir,
		clock:     toolbox.MockClock{NowTime: t1},
	}
	return &s, nil
}

func teardownFilesTest(s *Storer) error {
	return os.RemoveAll(s.filesPath)
}

func randStringBytes(n int) string {
	b := make([]byte, n)
	for i := range b {
		b[i] = letterBytes[rand.Intn(len(letterBytes))]
	}
	return string(b)
}

func makeAliasesTest(s *Storer, n int) (alias.Aliases, error) {
	domain := "example.com"
	aliases := alias.Aliases{}

	for i := 0; i < n; i++ {
		a := alias.Alias{
			Alias:  randStringBytes(16),
			Domain: domain,
		}

		fa := toFileAlias(a)
		if err := writeAliasFile(s.filesPath, fa); err != nil {
			return nil, err
		}

		aliases = append(aliases, a)
	}
	return aliases, nil
}

func TestOpenEmptyDir(t *testing.T) {
	s, err := setupFilesTest()
	if err != nil {
		t.Error(err)
	}
	defer teardownFilesTest(s)

	err = s.Open(false)
	if err != nil {
		t.Error(err)
	}
	s.Close()

	// No aliases in the directory
	assert.Equal(t, len(s.aliases), 0)
}

func TestOpenWithAliases(t *testing.T) {
	s, err := setupFilesTest()
	if err != nil {
		t.Error(err)
	}
	defer teardownFilesTest(s)

	_, err = makeAliasesTest(s, 10)
	if err != nil {
		t.Error(err)
	}

	err = s.Open(false)
	if err != nil {
		t.Error(err)
	}
	s.Close()

	// 10 aliases in the directory
	assert.Equal(t, len(s.aliases), 10)
}

func TestOpenReadOnlyWithMissingDir(t *testing.T) {
	s, err := setupFilesTest()
	if err != nil {
		t.Error(err)
	}
	defer teardownFilesTest(s)

	// Remove the directory
	os.Remove(s.filesPath)

	// Opening read-only should error because the directory does not exist
	err = s.Open(true)
	assert.Error(t, err)

	s.Close()
}

func TestOpenWithFileInsteadOfDir(t *testing.T) {
	s, err := setupFilesTest()
	if err != nil {
		t.Error(err)
	}
	defer teardownFilesTest(s)

	// Remove the directory
	os.Remove(s.filesPath)

	// Create a file in place
	f, err := os.Create(s.filesPath)
	if err != nil {
		t.Error(err)
	}
	f.Close()

	// Opening should error because the directory is a file
	err = s.Open(false)
	assert.Error(t, err)

	s.Close()
}

func TestOpenWithBadAlias(t *testing.T) {
	s, err := setupFilesTest()
	if err != nil {
		t.Error(err)
	}
	defer teardownFilesTest(s)

	aliases, err := makeAliasesTest(s, 1)
	if err != nil {
		t.Error(err)
	}
	a := aliases[0]

	// Write an empty file that will not un-marshall
	fn := filepath.Join(s.filesPath, aliasFilename(a.Alias, a.Domain))
	f, err := os.Create(fn)
	if err != nil {
		t.Error(err)
	}
	f.Close()

	err = s.Open(false)
	assert.Error(t, err)
}
