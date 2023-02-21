// Copyright (C) 2023, Ava Labs, Inc. All rights reserved.
// See the file LICENSE for licensing terms.

package cmd

import (
	"os"
	"time"

	"github.com/spf13/cobra"
)

const requestTimeout = 30 * time.Second

var (
	// Params
	privateKeyPath  string
	replayerDirPath string
	indexEndpoint   string
	ipfsEndpoint    string
	gorseEndpoint   string
	servicer        string
	commission      string
	pending         int
	port            int
	persist         bool
	contentSize     int

	// Parsed
	workDir string

	rootCmd = &cobra.Command{
		Use:        "shared",
		Short:      "Shared CLI",
		SuggestFor: []string{"shared", "shared-cli"},
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
		ingestCmd,
		servicerCmd,
		replayerCmd,
		spamCmd,
	)
}

func Execute() error {
	return rootCmd.Execute()
}
