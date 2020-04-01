package s3

import (
	"time"

	"github.com/patsoffice/aliasman/internal/alias"
)

// ToAlias converts the internal S3 representation of an alias to an Alias
func (ia *IndexAlias) ToAlias() alias.Alias {
	a := alias.Alias{
		Alias:          ia.Alias,
		Domain:         ia.Domain,
		EmailAddresses: ia.EmailAddresses,
		Description:    ia.Description,
		Suspended:      ia.Suspended,
		CreatedTS:      time.Time(ia.CreatedTS),
		ModifiedTS:     time.Time(ia.ModifiedTS),
		SuspendedTS:    time.Time(ia.SuspendedTS),
	}

	return a
}

// FromAlias converts an Alias to an internal S3 representation of an alias
func (ia *IndexAlias) FromAlias(a alias.Alias) {
	ia.Alias = a.Alias
	ia.Domain = a.Domain
	ia.EmailAddresses = a.EmailAddresses
	ia.Description = a.Description
	ia.Suspended = a.Suspended
	ia.CreatedTS = metadataTime(a.CreatedTS)
	ia.ModifiedTS = metadataTime(a.ModifiedTS)
	ia.SuspendedTS = metadataTime(a.SuspendedTS)
}
