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

package sqlite3

const (
	createAliasTableSQL = `
CREATE TABLE IF NOT EXISTS alias (
    alias         VARCHAR(64),
    addresses     VARCHAR(255),
    domain        VARCHAR(255),
    description   VARCHAR(255),
	suspended     TINYINT(1) NOT NULL DEFAULT '0',
	created_ts    TIMESTAMP DEFAULT (strftime('%s', 'now')),
	modified_ts   TIMESTAMP DEFAULT (strftime('%s', 'now')),
	suspended_ts  TIMESTAMP DEFAULT NULL,
	PRIMARY KEY   (alias, domain)
)
`
	getSingleAliasSQL = `
SELECT
	alias,
	domain,
	addresses,
	description,
	suspended,
	created_ts,
	modified_ts,
	suspended_ts
FROM alias
WHERE 
	    alias = ?
	AND domain = ?
`
	getAllAliasSQL = `
SELECT
	alias,
	domain,
	addresses,
	description,
	suspended,
	created_ts,
	modified_ts,
	suspended_ts
FROM alias
`
	putAliasSQL = `
INSERT OR REPLACE
INTO alias (
    alias,
    addresses,
    domain,
	description,
	suspended,
	created_ts,
	modified_ts,
	suspended_ts
) VALUES (
	?,
	?,
	?,
	?,
	?,
	?,
	?,
	?
)
`
	updateModifiedSQL = `
UPDATE alias
SET
	modified_ts = CURRENT_TIMESTAMP
WHERE
		alias = ?
	AND domain = ?
`
	suspendAliasSQL = `
UPDATE alias
SET
	suspended = true,
	modified_ts = CURRENT_TIMESTAMP,
	suspended_ts = CURRENT_TIMESTAMP
WHERE
	    alias = ?
	AND domain = ?
`
	unsuspendAliasSQL = `
UPDATE alias
SET
	suspended = false,
	modified_ts = CURRENT_TIMESTAMP,
	suspended_ts = NULL
WHERE
	    alias = ?
	AND domain = ?
`
	deleteAliasSQL = `
DELETE
FROM alias
WHERE
	    alias = ?
	AND domain = ?
`
)
