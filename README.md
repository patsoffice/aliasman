# Aliasman

Aliasman is a CLI tool for managing a larger number of email aliases. Since I've had my own domain, I've been generating email aliases for every company I do business with on the internet for a number of years and this is the tool that I use to manage it.

## Overview

Aliasman currently uses two kinds of providers. First is an `email_type` provider that is the email service API for creating and deleting the aliases. Second is a `storage_type` provider that is the API storing the information about the aliases and holds information that is not typically stored in the `email_type` (such as a description of the alias and when it was created/modified). The `storage_type`, generally, should be more easily queryable for faster lookups.

The tool is currently only tested on Mac OS X systems, but should just work on other UNIX like systems that have a Go compiler. With minimal work, it should work fine on Windows too but this is currently untested.

## Installing

Installing Aliasman is easy. First, use `go get` to install the latest version
of the application from GitHub. This command will install the `aliasman` executable in the Go path:

    go get -u github.com/aliasman/cmd/aliasman

## Configuring

Aliasman has a command used to configure all of the available providers:

    aliasman config

The program will then prompt for information relevant to the operation of the provider.

## Providers

Aliasman supports the following providers and their configuration options (stored in the configuration file):

* `rackspace_email_api` - The API fronting [Rackspace Email](https://www.rackspace.com/email-hosting/webmail) (`email_type`)
  * `rackspace_api_user_key` - Rackspace API user key ([link to get/make API key](https://cp.rackspace.com/MyAccount/Profile?showApiKeys))
  * `rackspace_api_secret_key` - Rackspace API secret key
* `s3` - Amazon Web Services' [Simple Storage Service](https://aws.amazon.com/s3/) (`storage_type`)
  * `s3_region` - The S3 endpoint to connect to
  * `s3_bucket` - The S3 bucket to use
  * `s3_access_key` - The AWS IAM user access key to use. The user needs the following IAM permissions for the specified `s3_bucket` resource:
    * s3:ListBucket
    * s3:GetObject
    * s3:PutObject
  * `s3_secret_key` - The AWS IAM user secret key to use
* `sqlite3` - [SQLite](https://sqlite.org/index.html) is a small, fast, self-contained, high-reliability, full-featured, SQL database engine (`storage_type`)
  * `sqlite_db_path` - The path for the SQLite database file

## Using

Once configured, Aliasman can be used to create an email alias. In this case, we will be creating a random email address with the use of the `-r` flag. However, we could have supplied an alias of our choosing with `-a alias`.

    aliasman alias create -d example.com -D "company.com" -r -e person1@example.com,person2@example.com

This will create both an email alias with our `email_type` provider and an entry with our `storage_type` provider. The output of the above command is:

    Created alias 5f888d1272833b09@example.com that points to person1@example.com, person2@example.com

Since we likely won't remember the alias name that was created for us, we can for the alias in the future with the search subcommand:

    aliasman alias search -s company.com
    ┌───┬──────────────────┬─────────────┬──────────────────────────────────────────┬─────────────┬───────────┬──────────────────────┬──────────────────────┬────────────────┐
    │   │ Alias            │ Domain      │ Email Address(es)                        │ Description │ Suspended │ Created Time         │ Modified Time        │ Suspended Time │
    ├───┼──────────────────┼────────────────┼───────────────────────────────────────┼─────────────┼───────────┼──────────────────────┼──────────────────────┼────────────────┤
    │ 1 │ 5f888d1272833b09 │ example.com │ person1@example.com, person2@example.com │ example.com │ No        │ 2020-03-28T13:46:43Z │ 2020-03-28T13:46:43Z │                │
    └───┴──────────────────┴─────────────┴──────────────────────────────────────────┴─────────────┴───────────┴──────────────────────┴──────────────────────┴────────────────┘

When we don't want to do business with `example.com` anymore, we can `suspend` the alias. This will keep the information in the `storage_type` provider but will disable or delete the alias in the `email_type` provider:

    aliasman alias suspend -d example.com -a 5f888d1272833b09

If we search for our alias again we see that it's suspended status is `Yes` and a suspended timestamp is set:

    aliasman alias search -s company.com
    ┌───┬──────────────────┬─────────────┬──────────────────────────────────────────┬─────────────┬───────────┬──────────────────────┬──────────────────────┬──────────────────────┐
    │   │ Alias            │ Domain      │ Email Address(es)                        │ Description │ Suspended │ Created Time         │ Modified Time        │ Suspended Time       │
    ├───┼──────────────────┼─────────────┼──────────────────────────────────────────┼─────────────┼───────────┼──────────────────────┼──────────────────────┼──────────────────────┤
    │ 1 │ 5f888d1272833b09 │ example.com │ person1@example.com, person2@example.com │ example.com │ Yes       │ 2020-03-28T13:46:43Z │ 2020-03-28T13:46:43Z │ 2020-03-28T14:53:42Z │
    └───┴──────────────────┴─────────────┴──────────────────────────────────────────┴─────────────┴───────────┴──────────────────────┴──────────────────────┴──────────────────────┘

If we don't want a record of the alias anymore, it can be deleted:

    aliasman alias delete -d example.com -a 5f888d1272833b09

Full help is always available for commands and sub-commands with `--help`.

## Development

Aliasman uses [go-extpoints](https://github.com/progrium/go-extpoints) for registering providers. If any changes are made to the provider interfaces, the extpoints will need to be regenerated.

    go install github.com/progrium/go-extpoints
    go generate ./...
