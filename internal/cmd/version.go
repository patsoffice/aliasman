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

package cmd

import (
	"fmt"

	"github.com/spf13/cobra"
)

var (
	// versionCmd represents the list-providers command
	versionCmd = &cobra.Command{
		Use:   "version",
		Short: "Prints the version of the program",
		Run:   versionRun,
	}
	// Rev represents the revision. When built via CI, this is replaced with Git
	// SHA1 by the linker.
	Rev string = "HEAD"
	// Ver represents the version. When built via CI, this is replaced with Git
	// tag it's built with.
	Ver string = "development version"
)

func init() {
	rootCmd.AddCommand(versionCmd)
}

func versionRun(cobraCmd *cobra.Command, args []string) {
	fmt.Printf("aliasman -- %s -- %s\n", Ver, Rev)
}
