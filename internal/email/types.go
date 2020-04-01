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

// If the interface for ProviderFactory is changed, in the root directory of
// this project run:
//
//     go install github.com/progrium/go-extpoints
//     go generate ./...
//
//go:generate go-extpoints . ProviderFactory

package email

import "github.com/patsoffice/aliasman/internal/alias"

// ProviderFactory is implemented by any email provider in order to create
// new instances and configure them for use. The configuration values are
// stored in and read from Viper.
type ProviderFactory interface {
	New() (Provider, error)
	Config() error
}

// Provider is implemented by any email provider giving the ability to create,
// list and delete aliases on the email system.
type Provider interface {
	Type() string
	Description() string
	AliasCreate(string, string, []string) error
	AliasDelete(string, string) error
	AliasList(string) (alias.Aliases, error)
}
