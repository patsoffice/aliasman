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
	"sync"
	"time"

	"github.com/aws/aws-sdk-go/service/s3"
	"github.com/aws/aws-sdk-go/service/s3/s3iface"
	"github.com/patsoffice/aliasman/internal/alias"
	"github.com/patsoffice/aliasman/internal/storage"
	"github.com/patsoffice/aliasman/internal/util"
)

func init() {
	cn := new(ConfigerNewer)
	storage.ProviderFactories.Register(cn, "s3")
}

// ConfigerNewer implements the factory methods for the s3 provider.
type ConfigerNewer struct{}

// metadtataTime is a time struct with MarshalJSON and UnmarshalJSON methods
// that allow us to store timestamps in an object's metadata and in the JSON
// index object.
type metadataTime time.Time

type objects []*s3.Object

// IndexAlias represents an alias as stored in the S3 index blob
type IndexAlias struct {
	Alias          string       `json:"alias"`
	Domain         string       `json:"domain"`
	EmailAddresses []string     `json:"email_addresses"`
	Description    string       `json:"description"`
	Suspended      bool         `json:"suspended"`
	CreatedTS      metadataTime `json:"created_ts"`
	ModifiedTS     metadataTime `json:"modified_ts"`
	SuspendedTS    metadataTime `json:"suspended_ts"`
}

// IndexAliases represent an array of aliases in the S3 index blob
type IndexAliases []IndexAlias

// Storer implements the storage provder methods and state for the s3
// provider.
type Storer struct {
	readOnly        bool
	clock           util.Clock
	aliases         alias.AliasesMap
	objects         objects
	region          string
	accessKey       string
	secretKey       string
	bucket          string
	prefix          string
	svc             s3iface.S3API
	concurrentHeads int
	keyChan         chan *s3.HeadObjectInput
	aliasChan       chan *alias.Alias
	wg              sync.WaitGroup
	workerWG        sync.WaitGroup
	indexMD5        string
}
