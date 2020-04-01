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
	"io"
	"regexp"
	"time"
)

// Alias represents the state necessary for an email alias.
type Alias struct {
	Alias          string
	Domain         string
	EmailAddresses []string `column:"Email Address(es)"`
	Description    string
	Suspended      bool
	CreatedTS      time.Time `column:"Created Time"`
	ModifiedTS     time.Time `column:"Modified Time"`
	SuspendedTS    time.Time `column:"Suspended Time"`
}

// Aliases represents a slice of Alias structs.
type Aliases []Alias

// AliasesMap represents a map of Alias structs. The key is based
// on the Alias and Domain fields of the Alias structs.
type AliasesMap map[string]Alias

// Filter is the settings for filtering aliases. There are regexps for
// aliases, domains, email addresses and descriptions. By default
// enabled and suspended aliases will be included. There are options to
// exclude enabled and/or suspended aliases.
type Filter struct {
	Alias            *regexp.Regexp
	Domain           *regexp.Regexp
	EmailAddress     *regexp.Regexp
	Description      *regexp.Regexp
	ExcludeSuspended bool
	ExcludeEnabled   bool
}

// Table represents the state necessary to render a table of Aliases
// to an io.Writer.
type Table struct {
	aliases Aliases
	writer  io.Writer
	columns []string
	headers bool
	numbers bool
}
