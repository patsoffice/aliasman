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

package gsuite

import (
	"fmt"
	"strings"

	"github.com/patsoffice/aliasman/internal/alias"
	admin "google.golang.org/api/admin/directory/v1"
)

// Type returns the type of provider this package implements
func (g *Emailer) Type() string {
	return "gsuite"
}

// Description returns the description of the provider this package implements
func (g *Emailer) Description() string {
	return "GSuite Admin Domain API"
}

func fullAlias(a, d string) string {
	return fmt.Sprintf("%s@%s", a, d)
}

func splitAlias(a string) (string, string, error) {
	s := strings.Split(a, "@")
	if len(s) != 2 {
		return "", "", fmt.Errorf("full alias was not able to be split: %s", a)
	}
	return s[0], s[1], nil
}

func checkUsers(u []string) error {
	if len(u) == 0 || len(u) > 1 {
		return fmt.Errorf("gsuite only supports, and requires, a single user for for performing alias actions")
	}
	return nil
}

// AliasCreate creates an email alias for a G Suite user. If there is a
// problem creating the email alias, an error is returned.
func (g *Emailer) AliasCreate(alias, domain string, users ...string) error {
	if err := checkUsers(users); err != nil {
		return err
	}
	_, err := g.srv.Users.Aliases.Insert(users[0], &admin.Alias{Alias: fmt.Sprintf("%s@%s", alias, domain)}).Do()
	if err != nil {
		return err
	}

	return nil
}

// AliasDelete deletes an email alias from a G Suite user. If there is a
// problem deleting the email alias, an error is returned.
func (g *Emailer) AliasDelete(alias, domain string, users ...string) error {
	if err := checkUsers(users); err != nil {
		return err
	}
	if err := g.srv.Users.Aliases.Delete(users[0], fullAlias(alias, domain)).Do(); err != nil {
		return err
	}
	return nil
}

// AliasList reads all of the aliases for a G Suite user.
func (g *Emailer) AliasList(domain string, users ...string) (alias.Aliases, error) {
	if err := checkUsers(users); err != nil {
		return nil, err
	}
	r, err := g.srv.Users.Aliases.List(users[0]).Do()
	if err != nil {
		return nil, err
	}

	aliases := alias.Aliases{}
	for _, a := range r.Aliases {
		t := a.(map[string]interface{})
		emailAlias, ok := t["alias"]
		if !ok {
			continue
		}
		primaryEmail, ok := t["primaryEmail"]
		if !ok {
			continue
		}
		aliasPart, domainPart, err := splitAlias(emailAlias.(string))
		if err != nil {
			return nil, err
		}
		aliases = append(aliases, alias.Alias{
			Alias:          aliasPart,
			Domain:         domainPart,
			EmailAddresses: []string{primaryEmail.(string)},
		})
	}
	return aliases, nil
}
