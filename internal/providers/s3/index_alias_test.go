package s3

import (
	"testing"
	"time"

	"github.com/patsoffice/aliasman/internal/alias"
	"github.com/stretchr/testify/assert"
)

func TestToAlias(t *testing.T) {
	tables := []struct {
		in  IndexAlias
		out alias.Alias
	}{
		{
			in: IndexAlias{
				Alias:          "foo",
				Domain:         "bar.com",
				EmailAddresses: []string{"baz@baz.com"},
				Description:    "An alias from an S3 index file",
				Suspended:      true,
				CreatedTS:      metadataTime(time.Date(2001, 9, 11, 12, 46, 40, 0, time.UTC)),
				ModifiedTS:     metadataTime(time.Date(2001, 9, 11, 12, 46, 40, 0, time.UTC)),
				SuspendedTS:    metadataTime(time.Date(2001, 9, 11, 13, 3, 0, 0, time.UTC)),
			},
			out: alias.Alias{
				Alias:          "foo",
				Domain:         "bar.com",
				EmailAddresses: []string{"baz@baz.com"},
				Description:    "An alias from an S3 index file",
				Suspended:      true,
				CreatedTS:      time.Date(2001, 9, 11, 12, 46, 40, 0, time.UTC),
				ModifiedTS:     time.Date(2001, 9, 11, 12, 46, 40, 0, time.UTC),
				SuspendedTS:    time.Date(2001, 9, 11, 13, 3, 0, 0, time.UTC),
			},
		},
	}

	for _, table := range tables {
		ok := table.out.Equal(table.in.ToAlias())
		assert.True(t, ok, table.out.UnifiedDiff(table.in.ToAlias()))
	}
}

func TestFromAlias(t *testing.T) {
	tables := []struct {
		a alias.Alias
	}{
		{
			a: alias.Alias{
				Alias:          "foo",
				Domain:         "bar.com",
				EmailAddresses: []string{"baz@baz.com"},
				Description:    "An alias from an S3 index file",
				Suspended:      true,
				CreatedTS:      time.Date(2001, 9, 11, 12, 46, 40, 0, time.UTC),
				ModifiedTS:     time.Date(2001, 9, 11, 12, 46, 40, 0, time.UTC),
				SuspendedTS:    time.Date(2001, 9, 11, 13, 3, 0, 0, time.UTC),
			},
		},
	}

	for _, table := range tables {
		ia := IndexAlias{}
		ia.FromAlias(table.a)
		ok := table.a.Equal(ia.ToAlias())
		assert.True(t, ok, table.a.UnifiedDiff(ia.ToAlias()))
	}
}
