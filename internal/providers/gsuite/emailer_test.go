package gsuite

import (
	"testing"

	"github.com/stretchr/testify/assert"
)

func TestFullAlias(t *testing.T) {
	tables := []struct {
		alias, domain, out, msg string
	}{
		// Case 1
		{"foo", "bar.com", "foo@bar.com", "case 1"},
	}

	for _, table := range tables {
		assert.Equal(t, table.out, fullAlias(table.alias, table.domain), table.msg)
	}
}

func TestSplitAlias(t *testing.T) {
	tables := []struct {
		in, outAlias, outDomain string
		hasError                bool
		msg                     string
	}{
		// Case 1
		{"foo@bar.com", "foo", "bar.com", false, "case 1"},
		// Case 2
		{"foo", "", "", true, "case 2"},
		// Case 3
		{"foo@bar.com@baz.com", "", "", true, "case 3"},
	}

	for _, table := range tables {
		if table.hasError {
			_, _, err := splitAlias(table.in)
			assert.Error(t, err, table.msg)
		} else {
			a, d, err := splitAlias(table.in)
			assert.Equal(t, table.outAlias, a, table.msg)
			assert.Equal(t, table.outDomain, d, table.msg)
			assert.NoError(t, err, table.msg)
		}
	}
}
