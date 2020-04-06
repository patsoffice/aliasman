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

package storage

import "github.com/patsoffice/aliasman/internal/alias"

// ProviderFactory is implemented by any storage provider in order to create
// new instances and configure them for use. The configuration values are
// stored in and read from Viper.
type ProviderFactory interface {
	New() (Provider, error)
	Config() error
}

// Provider is implemented by any storage provider giving the ability to
// manipulate the storage of email aliases and associated metadata separate
// from the email system being used.
type Provider interface {
	Type() string
	Description() string
	Open(bool) error
	Close() error
	Get(string, string) (*alias.Alias, error)
	Put(alias.Alias, bool) error
	Update(alias.Alias, bool) error
	Search(alias.Filter, bool) (alias.Aliases, error)
	Suspend(string, string) error
	Unsuspend(string, string) error
	Delete(string, string) error
}
