// Copyright (C) 2023, Ava Labs, Inc. All rights reserved.
// See the file LICENSE for licensing terms.

package cmd

import (
	"context"
	"path/filepath"

	"github.com/spf13/cobra"

	"github.com/ava-labs/hypersdk/crypto"
	"github.com/ava-labs/indexvm/examples/memedisco/searcher"
)

func init() {
	searcherCmd.PersistentFlags().StringVar(
		&ipfsEndpoint,
		"ipfs-endpoint",
		"",
		"IPFS endpoint to connect to",
	)
	searcherCmd.PersistentFlags().StringVar(
		&indexEndpoint,
		"indexvm-endpoint",
		"",
		"RPC endpoint for indexvm",
	)
	searcherCmd.PersistentFlags().StringVar(
		&privateKeyPath,
		"private-key-path",
		filepath.Join(workDir, ".memedisco.pk"),
		"private key file path",
	)
	searcherCmd.PersistentFlags().StringVar(
		&searcherDirPath,
		"searcher-dir-path",
		filepath.Join(workDir, ".searcher"),
		"searcher directory path",
	)
}

var searcherCmd = &cobra.Command{
	Use:   "searcher [options]",
	Short: "searcher",
	Long: `
TODO

$ memedisco searcher

`,
	RunE: searcherFunc,
}

func searcherFunc(*cobra.Command, []string) error {
	ctx := context.Background()

	// Load signing key
	priv, err := crypto.LoadKey(privateKeyPath)
	if err != nil {
		return err
	}

	return searcher.Run(ctx, priv, indexEndpoint, ipfsEndpoint)
}
