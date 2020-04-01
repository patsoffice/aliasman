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

package rea

import (
	"fmt"
	"strings"

	"github.com/patsoffice/aliasman/internal/alias"
)

// Type returns the type of provider this package implements
func (r *Emailer) Type() string {
	return "rackspace_email_api"
}

// Description returns the description of the provider this package implements
func (r *Emailer) Description() string {
	return "Rackspace Email backed alias configuration"
}

// AliasCreate creates an email alias on the Rackspace Email service. If there
// is a problem creating the email address, an error is returned.
func (r *Emailer) AliasCreate(alias, domain string, addresses []string) error {
	if resp, err := r.client.RackspaceEmailAliases.Add(r.ctx, domain, alias, addresses); err != nil {
		if v, ok := resp.Response.Header["X-Error-Message"]; ok {
			return fmt.Errorf("rackspace_email_api: failure creating alias: %s", strings.Join(v, " "))
		}
		return err
	}
	return nil
}

// AliasDelete deletes an email alias from the Rackspace Email service. If
// there is a problem deleting the email address, an error is returned.
func (r *Emailer) AliasDelete(alias, domain string) error {
	if resp, err := r.client.RackspaceEmailAliases.Delete(r.ctx, domain, alias); err != nil {
		if v, ok := resp.Response.Header["X-Error-Message"]; ok {
			// We can ignore 404 error codes ("Non-existent alias")
			if resp.Response.StatusCode == 404 {
				return nil
			}
			return fmt.Errorf("rackspace_email_api: failure deleting alias: %s", strings.Join(v, " "))
		}
		return err
	}
	return nil
}

// AliasList reads all of the aliases on the Rackspace Email service for the
// specified domain.
//
// Please note that this operation can be slow for a large number of aliases
// since there is no bulk GET operation provided by the API. The client
// performs throttling internally to prevent errors from the service.
func (r *Emailer) AliasList(domain string) (alias.Aliases, error) {
	list, resp, err := r.client.RackspaceEmailAliases.Index(r.ctx, nil, domain)
	if err != nil {
		if v, ok := resp.Response.Header["X-Error-Message"]; ok {
			return nil, fmt.Errorf("rackspace_email_api: failure listing aliases: %s", strings.Join(v, " "))
		}
		return nil, err
	}

	aliases := alias.Aliases{}
	for _, a := range list {
		t, resp, err := r.client.RackspaceEmailAliases.Show(r.ctx, domain, a.Name)
		if err != nil {
			if v, ok := resp.Response.Header["X-Error-Message"]; ok {
				return nil, fmt.Errorf("rackspace_email_api: failure listing aliases: %s", strings.Join(v, " "))
			}
			return nil, err
		}
		a := alias.Alias{
			Alias:          a.Name,
			Domain:         domain,
			EmailAddresses: t.EmailAddressList.Addresses,
		}
		aliases = append(aliases, a)
	}

	return aliases, nil
}
