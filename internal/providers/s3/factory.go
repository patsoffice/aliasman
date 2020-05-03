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

package s3

import (
	"bufio"
	"os"

	"github.com/aws/aws-sdk-go/aws"
	"github.com/aws/aws-sdk-go/aws/credentials"
	"github.com/aws/aws-sdk-go/aws/session"
	"github.com/aws/aws-sdk-go/service/s3"
	"github.com/patsoffice/aliasman/internal/alias"
	"github.com/patsoffice/aliasman/internal/cmd"
	"github.com/patsoffice/aliasman/internal/storage"
	"github.com/patsoffice/toolbox"
	"github.com/spf13/viper"
)

const (
	defaultConcurrentHeads = 25
	defaultChannelDepth    = 50
)

func init() {
	var (
		region, accessKey, secretKey, bucket string
	)

	rootCmd := cmd.RootCmd()
	rootCmd.PersistentFlags().StringVar(&region, "s3-region", "", "S3 region")
	rootCmd.PersistentFlags().StringVar(&accessKey, "s3-access-key", "", "S3 access key")
	rootCmd.PersistentFlags().StringVar(&secretKey, "s3-secret-key", "", "S3 secret key")
	rootCmd.PersistentFlags().StringVar(&bucket, "s3-bucket", "", "S3 bucket")

	viper.BindPFlag("s3_region", rootCmd.PersistentFlags().Lookup("s3-region"))
	viper.BindPFlag("s3_access_key", rootCmd.PersistentFlags().Lookup("s3-access-key"))
	viper.BindPFlag("s3_secret_key", rootCmd.PersistentFlags().Lookup("s3-secret-key"))
	viper.BindPFlag("s3_bucket", rootCmd.PersistentFlags().Lookup("s3-bucket"))

	viper.SetDefault("s3_concurrent_heads", defaultConcurrentHeads)
	viper.SetDefault("s3_channel_depth", defaultChannelDepth)
}

// Config takes input from the user to configure the s3 provider.
func (cn *ConfigerNewer) Config() error {
	scanner := bufio.NewScanner(os.Stdin)

	if ok := toolbox.CheckYes(scanner, "Configure S3 provider?", false); ok {
		region := toolbox.GetInputString(scanner, "S3 region", viper.GetString("s3_region"))
		viper.Set("s3_region", region)

		accessKey := toolbox.GetInputString(scanner, "S3 access key", viper.GetString("s3_access_key"))
		viper.Set("s3_access_key", accessKey)

		secretKey := toolbox.GetInputString(scanner, "S3 secret key", viper.GetString("s3_secret_key"))
		viper.Set("s3_secret_key", secretKey)

		bucket := toolbox.GetInputString(scanner, "S3 bucket", viper.GetString("s3_bucket"))
		viper.Set("s3_bucket", bucket)

		if ok := toolbox.CheckYes(scanner, "Configure advanced S3 options?", false); ok {
			concurrentHeads := toolbox.GetInputInt(scanner, "Number of concurrent HEAD calls", viper.GetInt("s3_concurrent_heads"))
			viper.Set("s3_concurrent_heads", concurrentHeads)

			channelDepth := toolbox.GetInputInt(scanner, "Size of concurrency channels", viper.GetInt("s3_channel_depth"))
			viper.Set("s3_channel_depth", channelDepth)
		}

		if ok := toolbox.CheckYes(scanner, "Make s3 the default storage provider?", true); ok {
			viper.Set("storage_type", "s3")
		}
	}
	return nil
}

// New returns a usable instance of the s3 provider.
func (cn *ConfigerNewer) New() (storage.Provider, error) {
	region := viper.GetString("s3_region")
	accessKey := viper.GetString("s3_access_key")
	secretKey := viper.GetString("s3_secret_key")
	bucket := viper.GetString("s3_bucket")

	storer := Storer{
		bucket: bucket,
		clock:  toolbox.RealClock{},
	}
	storer.svc = s3.New(session.New(&aws.Config{
		Region:      aws.String(region),
		Credentials: credentials.NewStaticCredentials(accessKey, secretKey, ""),
	}))

	storer.concurrentHeads = viper.GetInt("s3_concurrent_heads")
	storer.keyChan = make(chan *s3.HeadObjectInput, viper.GetInt("s3_channel_depth"))
	storer.aliasChan = make(chan *alias.Alias, viper.GetInt("s3_channel_depth"))
	storer.aliases = make(map[string]alias.Alias)
	storer.objects = make([]*s3.Object, 0)

	return &storer, nil
}
