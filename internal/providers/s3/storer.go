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
	"bytes"
	"crypto/md5"
	"encoding/json"
	"errors"
	"fmt"
	"os"
	"sort"
	"strings"
	"time"

	"github.com/aws/aws-sdk-go/aws"
	"github.com/aws/aws-sdk-go/aws/awserr"
	"github.com/aws/aws-sdk-go/service/s3"
	"github.com/patsoffice/aliasman/internal/alias"
)

// Type returns the type of provider this package implements
func (s *Storer) Type() string {
	return "s3"
}

// Description returns the description of the provider this package implements
func (s *Storer) Description() string {
	return "S3 backed alias storage"
}

func (o objects) Len() int { return len(o) }

func (o objects) Swap(i, j int) { o[i], o[j] = o[j], o[i] }

func (o objects) Less(i, j int) bool {
	if o[j].LastModified.Equal(*o[i].LastModified) {
		return *o[i].Key < *o[j].Key
	}
	return o[i].LastModified.Before(*o[j].LastModified)
}

// listKeys performs a paginated list of the keys in the bucket.
func (s *Storer) listKeys() error {
	input := &s3.ListObjectsV2Input{
		Bucket:  aws.String(s.bucket),
		MaxKeys: aws.Int64(100), // BUG(patsoffice) make configurable
	}

	err := s.svc.ListObjectsV2Pages(
		input,
		func(page *s3.ListObjectsV2Output, last bool) bool {
			for _, o := range page.Contents {
				s.objects = append(s.objects, o)
			}
			return true
		},
	)
	if err != nil {
		if aerr, ok := err.(awserr.Error); ok {
			switch aerr.Code() {
			case s3.ErrCodeNoSuchBucket:
				return aerr
			default:
				return aerr
			}
		}
		return err
	}
	return nil
}

// parseTS parses the timestamp stored in the S3 metadata. If the metadate key
// does not exist or is not parseable, the current time is confirmed.
// Otherwise, the parsed timestamp is returned.
func (s *Storer) parseTS(m map[string]*string, key string) time.Time {
	if ts, ok := m[key]; ok {
		parsedTS, err := time.Parse(time.RFC3339, *ts)
		if err != nil {
			parsedTS = s.clock.Now().UTC()
		}
		return parsedTS
	}
	return s.clock.Now().UTC()
}

func (s *Storer) headS3Key(input *s3.HeadObjectInput) (*alias.Alias, error) {
	res, err := s.svc.HeadObject(input)
	if err != nil {
		if aerr, ok := err.(awserr.Error); ok {
			switch aerr.Code() {
			case "NotFound":
				return nil, nil
			default:
				fmt.Fprintf(os.Stderr, "S3 error: %v\n", aerr.Error())
			}
		} else {
			fmt.Fprintf(os.Stderr, "S3 error: %v\n", err.Error())
		}
		return nil, err
	}

	createdTS := s.parseTS(res.Metadata, "Created_ts")
	modifiedTS := s.parseTS(res.Metadata, "Modified_ts")
	suspendedTS := s.parseTS(res.Metadata, "Suspended_ts")
	a := alias.Alias{
		Alias:          *res.Metadata["Alias"],
		Domain:         *res.Metadata["Domain"],
		EmailAddresses: strings.Split(*res.Metadata["Email_addresses"], ","),
		Description:    *res.Metadata["Description"],
		Suspended:      "true" == *res.Metadata["Suspended"],
		CreatedTS:      createdTS,
		ModifiedTS:     modifiedTS,
		SuspendedTS:    suspendedTS,
	}
	return &a, nil
}

func (s *Storer) headAlias() {
	defer s.workerWG.Done()

	for input := range s.keyChan {
		a, err := s.headS3Key(input)
		// BUG(patsoffice) pass the error back with the result...
		if err != nil {
			fmt.Fprintf(os.Stderr, "%v\n", err)
		}
		s.aliasChan <- a
	}
}

func (s *Storer) headKeys() {
	defer s.wg.Done()
	defer close(s.aliasChan)

	for i := 0; i < s.concurrentHeads; i++ {
		s.workerWG.Add(1)
		go s.headAlias()
	}

	s.workerWG.Wait()
}

func (s *Storer) assembleAliases() {
	defer s.wg.Done()

	for a := range s.aliasChan {
		s.aliases.Add(*a)
	}
}

func (s *Storer) getAliases() {
	s.wg.Add(1)
	go func() {
		defer s.wg.Done()
		defer close(s.keyChan)

		for _, o := range s.objects {
			if strings.HasPrefix(*o.Key, "alias-") {
				s.keyChan <- &s3.HeadObjectInput{
					Bucket: aws.String(s.bucket),
					Key:    o.Key,
				}
			}
		}
	}()

	s.wg.Add(1)
	go s.headKeys()

	s.wg.Add(1)
	go s.assembleAliases()

	s.wg.Wait()
}

// indexBeforeAlias returns true if the key named "index" is before any
// keys that start with "alias-". The number of alias keys are returned
// as well.
func indexBeforeAliasCount(objects []*s3.Object) (bool, int) {
	aliasCount := 0
	indexBeforeAlias := false
	foundAlias := false
	for _, o := range objects {
		if !indexBeforeAlias && !foundAlias {
			if "index" == *o.Key {
				indexBeforeAlias = true
			}
		}
		if strings.HasPrefix(*o.Key, "alias-") {
			if !foundAlias {
				foundAlias = true
			}
			aliasCount++
		}
	}
	return indexBeforeAlias, aliasCount
}

// Open prepares the s3 provider for use. The bucket will be listed and if
// the alias key is the newest key in the bucket listing, the index file
// is loaded. Otherwise, all alias keys are HEADed and their metadata is
// extracted and assembled into a slice of aliases. An error is returned
// if there is a failure interacting with S3.
func (s *Storer) Open(readOnly bool) error {
	s.readOnly = readOnly
	if err := s.listKeys(); err != nil {
		return err
	}

	sort.Sort(sort.Reverse(s.objects))
	haveIndex, aliasCount := indexBeforeAliasCount(s.objects)

	// If we have aliases and the index key came before any alias keys, load
	// the JSON blob from the index and convert it to Aliases.
	loadedIndex := false
	if aliasCount > 0 && haveIndex {
		input := &s3.GetObjectInput{
			Bucket: &s.bucket,
			Key:    aws.String("index"),
		}
		result, err := s.svc.GetObject(input)
		if err != nil {
			if aerr, ok := err.(awserr.Error); ok {
				switch aerr.Code() {
				default:
					return aerr
				}
			}
			return err
		}

		aliases := IndexAliases{}
		dec := json.NewDecoder(result.Body)
		if err := dec.Decode(&aliases); err != nil {
			return err
		}
		if len(aliases) == aliasCount {
			for _, a := range aliases {
				ra := alias.Alias{
					Alias:          a.Alias,
					Domain:         a.Domain,
					EmailAddresses: a.EmailAddresses,
					Description:    a.Description,
					Suspended:      a.Suspended,
					CreatedTS:      time.Time(a.CreatedTS),
					ModifiedTS:     time.Time(a.ModifiedTS),
					SuspendedTS:    time.Time(a.SuspendedTS),
				}
				key := keyIndex(a.Alias, a.Domain)
				s.aliases[key] = ra
			}
			loadedIndex = true
			s.indexMD5 = strings.Trim(*result.ETag, "\"") // The object's ETAG is the MD5 checksum of the object's body
		}
	}
	if !loadedIndex {
		s.getAliases()
	}
	return nil
}

// Close updates the index stored in S3 if the MD5 sum of the index JSON and
// key MD5 sum are different. An error is returned if there is a failure
// interacting with S3.
func (s *Storer) Close() error {
	// Put the aliases into a slice so we can sort it. This should give us a
	// stable MD5 checksum if there are no changes.
	aliases := s.aliases.ToSlice()
	sort.Sort(aliases)

	storeraliases := IndexAliases{}
	for _, a := range aliases {
		storeralias := IndexAlias{}
		storeralias.FromAlias(a)
		storeraliases = append(storeraliases, storeralias)
	}
	b, err := json.MarshalIndent(storeraliases, "", "  ")
	if err != nil {
		fmt.Fprintf(os.Stderr, "Failure marshalling JSON: %v\n", err)
	}

	// If the MD5 sum of the JSON blob does not match that of the one stored
	// in S3, upload a new index file.
	indexMD5 := fmt.Sprintf("%x", md5.Sum(b))
	if indexMD5 != s.indexMD5 {
		input := &s3.PutObjectInput{
			Body:   bytes.NewReader(b),
			Bucket: &s.bucket,
			Key:    aws.String("index"),
			Metadata: map[string]*string{
				"creation_ts": aws.String(s.clock.Now().UTC().Format(time.RFC3339)),
			},
		}

		_, err = s.svc.PutObject(input)
		if err != nil {
			if aerr, ok := err.(awserr.Error); ok {
				switch aerr.Code() {
				default:
					return aerr
				}
			}
			return err
		}
	}
	return nil
}

func (s *Storer) s3KeyName(a, d string) string {
	return fmt.Sprintf("alias-%s@%s", a, d)
}

// Get HEADs a key in S3 and returns a pointer to an Alias. An error is
// returned if there is a failure interacting with S3.
func (s *Storer) Get(a, d string) (*alias.Alias, error) {
	s3Key := s.s3KeyName(a, d)
	input := &s3.HeadObjectInput{
		Bucket: &s.bucket,
		Key:    &s3Key,
	}
	alias, err := s.headS3Key(input)
	if err != nil {
		return nil, err
	}
	return alias, nil
}

func (s *Storer) putAlias(a alias.Alias, updateModified bool) error {
	keyName := s.s3KeyName(a.Alias, a.Domain)
	emailAddresses := strings.Join(a.EmailAddresses, ",")

	modifiedTS := time.Time{}
	if updateModified {
		modifiedTS = s.clock.Now()
	} else {
		modifiedTS = a.ModifiedTS
	}

	input := &s3.PutObjectInput{
		Body:   nil,
		Bucket: aws.String(s.bucket),
		Key:    aws.String(keyName),
		Metadata: map[string]*string{
			"alias":           aws.String(a.Alias),
			"domain":          aws.String(a.Domain),
			"description":     aws.String(a.Description),
			"email_addresses": aws.String(emailAddresses),
			"suspended":       aws.String("false"),
			"modified_ts":     aws.String(modifiedTS.UTC().Format(time.RFC3339)),
		},
	}

	if a.Suspended {
		input.Metadata["suspended"] = aws.String("true")
	} else {
		input.Metadata["suspended"] = aws.String("false")
	}
	if a.CreatedTS.IsZero() {
		input.Metadata["created_ts"] = aws.String(s.clock.Now().UTC().Format(time.RFC3339))
	} else {
		input.Metadata["created_ts"] = aws.String(a.CreatedTS.Format(time.RFC3339))
	}
	if a.SuspendedTS.IsZero() {
		input.Metadata["suspended_ts"] = aws.String(time.Time{}.Format(time.RFC3339))
	} else {
		input.Metadata["suspended_ts"] = aws.String(a.SuspendedTS.Format(time.RFC3339))
	}

	_, err := s.svc.PutObject(input)
	if err != nil {
		if aerr, ok := err.(awserr.Error); ok {
			switch aerr.Code() {
			default:
				return aerr
			}
		} else {
			return err
		}
	}

	return nil
}

func keyIndex(alias, domain string) string {
	return fmt.Sprintf("%s@%s", alias, domain)
}

// Put PUTs an Alias' content into an S3 key. The alias fields are stored in
// the key metadata. An error is returned if there is a failure interacting
// with S3.
func (s *Storer) Put(alias alias.Alias, updateModified bool) error {
	if s.readOnly {
		return errors.New("S3 opened readonly")
	}

	s.aliases.Add(alias)

	if err := s.putAlias(alias, updateModified); err != nil {
		return err
	}

	return nil
}

// Update PUTs an Alias' content into an S3 key. The alias fields are stored in
// the key metadata. An error is returned if there is a failure interacting
// with S3.
func (s *Storer) Update(alias alias.Alias, updateModified bool) error {
	if s.readOnly {
		return errors.New("S3 opened readonly")
	}

	if err := s.putAlias(alias, updateModified); err != nil {
		return err
	}

	s.aliases.Del(alias)
	s.aliases.Add(alias)

	return nil
}

// Search applies the filter to the internal map of aliases and returns a
// sorted list of aliases that match the filter criteria.
func (s *Storer) Search(f alias.Filter, matchAnyRegexp bool) (alias.Aliases, error) {
	var aliases alias.Aliases

	for _, a := range s.aliases {
		if a.Filter(f, matchAnyRegexp) {
			aliases = append(aliases, a)
		}
	}
	sort.Sort(aliases)

	return aliases, nil
}

// Suspend updates the S3 key representing the alias with the suspended flag
// and updates the suspended timestamp. An error is returned if there is a
// failure interacting with S3.
func (s *Storer) Suspend(alias, domain string) error {
	if s.readOnly {
		return errors.New("S3 opened readonly")
	}

	key := keyIndex(alias, domain)
	if a, ok := s.aliases[key]; ok {
		a.Suspended = true
		a.SuspendedTS = s.clock.Now().UTC()

		if err := s.putAlias(a, true); err != nil {
			return err
		}
		s.aliases.Del(a)
		s.aliases.Add(a)
	} else {
		return fmt.Errorf("alias %s does not exist", key)
	}
	return nil
}

// Unsuspend updates the S3 key representing the alias with the suspended flag
// and updates the suspended timestamp. An error is returned if there is a
// failure interacting with S3.
func (s *Storer) Unsuspend(alias, domain string) error {
	if s.readOnly {
		return errors.New("S3 opened readonly")
	}

	key := keyIndex(alias, domain)
	if a, ok := s.aliases[key]; ok {
		a.Suspended = false
		a.SuspendedTS = time.Time{}

		if err := s.putAlias(a, true); err != nil {
			return err
		}
		s.aliases.Del(a)
		s.aliases.Add(a)
	} else {
		return fmt.Errorf("alias %s does not exist", key)
	}
	return nil
}

// Delete removes the S3 key representing the alias. An error is returned if
// there is a failure interacting with S3.
func (s *Storer) Delete(alias, domain string) error {
	if s.readOnly {
		return errors.New("S3 opened readonly")
	}

	keyName := s.s3KeyName(alias, domain)
	input := &s3.DeleteObjectInput{
		Bucket: aws.String(s.bucket),
		Key:    aws.String(keyName),
	}

	_, err := s.svc.DeleteObject(input)
	if err != nil {
		if aerr, ok := err.(awserr.Error); ok {
			switch aerr.Code() {
			default:
				return aerr
			}
		} else {
			return err
		}
	}

	return nil
}
