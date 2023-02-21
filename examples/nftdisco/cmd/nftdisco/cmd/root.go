// Copyright (C) 2023, Ava Labs, Inc. All rights reserved.
// See the file LICENSE for licensing terms.

package cmd

import (
	"os"

	"github.com/spf13/cobra"
)

var (
	// Params
	privateKeyPath   string
	ipfsDirPath      string
	viewerDirPath    string
	indexEndpoint    string
	servicerEndpoint string
	pending          int
	ipfsEndpoint     string
	gorseEndpoint    string
	searcherDirPath  string
	startTimestamp   int64

	// Parsed
	workDir string

	rootCmd = &cobra.Command{
		Use:        "nftdisco",
		Short:      "NFTDisco CLI",
		SuggestFor: []string{"nftdisco", "nftdisco-cli"},
	}
)

func init() {
	p, err := os.Getwd()
	if err != nil {
		panic(err)
	}
	workDir = p

	cobra.EnablePrefixMatching = true
	rootCmd.AddCommand(
		searcherCmd,
		viewerCmd,
	)
}

func Execute() error {
	return rootCmd.Execute()
}
