package util

import (
	"testing"

	"github.com/stretchr/testify/assert"
)

type mockScanner struct {
	scannerOut   []bool
	textOut      []string
	scannerIndex int
	textIndex    int
}

func (m *mockScanner) Scan() bool {
	m.scannerIndex++
	return m.scannerOut[m.scannerIndex-1]
}

func (m *mockScanner) Text() string {
	m.textIndex++
	return m.textOut[m.textIndex-1]
}

func TestGetInputInt(t *testing.T) {
	tables := []struct {
		scannerOut []bool
		textOut    []string
		def        int
		out        int
		msg        string
	}{
		{[]bool{true, false}, []string{"10"}, 0, 10, "case 1"},
		{[]bool{true, false}, []string{""}, 10, 10, "case 2"},
		{[]bool{true, true, false}, []string{"abc", "10"}, 0, 10, "case 3"},
	}

	for _, table := range tables {
		ms := &mockScanner{scannerOut: table.scannerOut, textOut: table.textOut}

		out := GetInputInt(ms, "", table.def)
		assert.Equal(t, table.out, out, table.msg)
	}
}

func TestGetInputString(t *testing.T) {
	tables := []struct {
		scannerOut []bool
		textOut    []string
		def        string
		out        string
		msg        string
	}{
		// Case 1: no default value, got input
		{[]bool{true, false}, []string{"abc"}, "", "abc", "case 1"},
		// Case 2: default value, empty input
		{[]bool{true, false}, []string{""}, "abc", "abc", "case 2"},
		// Case 3: no default value, simulated error
		{[]bool{false}, []string{}, "", "", "case 3"},
	}

	for _, table := range tables {
		ms := &mockScanner{scannerOut: table.scannerOut, textOut: table.textOut}

		out := GetInputString(ms, "", table.def)
		assert.Equal(t, table.out, out, table.msg)
	}
}

func TestCheckYes(t *testing.T) {
	tables := []struct {
		scannerOut []bool
		textOut    []string
		defYes     bool
		out        bool
		msg        string
	}{
		// Case 1: Y
		{[]bool{true, false}, []string{"Y"}, false, true, "case 1"},
		// Case 2: y
		{[]bool{true, false}, []string{"y"}, false, true, "case 2"},
		// Case 3: yes
		{[]bool{true, false}, []string{"yes"}, false, true, "case 3"},
		// Case 4: Non-yes
		{[]bool{true, false}, []string{"N"}, false, false, "case 4"},
		// Case 5: Empty, default no
		{[]bool{true, false}, []string{""}, false, false, "case 5"},
		// Case 6: Empty, default yes
		{[]bool{true, false}, []string{""}, true, true, "case 6"},
	}

	for _, table := range tables {
		ms := &mockScanner{scannerOut: table.scannerOut, textOut: table.textOut}

		out := CheckYes(ms, "", table.defYes)
		assert.Equal(t, table.out, out, table.msg)
	}
}
