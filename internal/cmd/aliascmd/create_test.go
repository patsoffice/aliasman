package aliascmd

import (
	"errors"
	"testing"

	"github.com/stretchr/testify/assert"
)

type mockRandReader struct{}

func (m mockRandReader) Read(b []byte) (int, error) {
	for i := 0; i < len(b); i++ {
		b[i] = 0
	}
	return len(b), nil
}

type mockErrorRandReader struct{}

func (m mockErrorRandReader) Read(b []byte) (int, error) {
	return 0, errors.New("read earror")
}

func TestRandomAlias(t *testing.T) {
	tables := []struct {
		length         int
		useB64Encoding bool
		out            string
		hasErr         bool
		msg            string
	}{
		{15, false, "348a9791dc41b89", false, "case 1"},
		{15, true, "aaaaaaaaaaaaaaa", false, "case 2"},
		{15, false, "", true, "case 3"},
	}

	for _, table := range tables {
		if table.hasErr {
			alias, err := randomAlias(mockErrorRandReader{}, table.length, table.useB64Encoding)
			assert.Error(t, err)
			assert.Equal(t, table.out, alias, table.msg)
		} else {
			alias, _ := randomAlias(mockRandReader{}, table.length, table.useB64Encoding)
			assert.Equal(t, table.out, alias, table.msg)
		}
	}
}
