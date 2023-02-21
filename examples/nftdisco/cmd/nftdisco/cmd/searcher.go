// Copyright (C) 2023, Ava Labs, Inc. All rights reserved.
// See the file LICENSE for licensing terms.

package cmd

import (
	"context"
	"path/filepath"

	"github.com/ava-labs/hypersdk/crypto"
	"github.com/ava-labs/hypersdk/pebble"
	"github.com/ava-labs/indexvm/examples/nftdisco/searcher"
	"github.com/spf13/cobra"
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
		&gorseEndpoint,
		"gorse-endpoint",
		"",
		"RPC endpoint for gorse",
	)
	searcherCmd.PersistentFlags().StringVar(
		&privateKeyPath,
		"private-key-path",
		filepath.Join(workDir, ".nftdisco.pk"),
		"private key file path",
	)
	searcherCmd.PersistentFlags().StringVar(
		&searcherDirPath,
		"searcher-dir-path",
		filepath.Join(workDir, ".searcher"),
		"searcher directory path",
	)
	searcherCmd.PersistentFlags().Int64Var(
		&startTimestamp,
		"start-timestamp",
		-1,
		"timestamp to start search after",
	)
}

var searcherCmd = &cobra.Command{
	Use:   "searcher [options]",
	Short: "searcher",
	Long: `
TODO

$ nftdisco searcher

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

	// Load database
	db, err := pebble.New(searcherDirPath, pebble.NewDefaultConfig())
	if err != nil {
		return err
	}
	defer db.Close()
	return searcher.Run(ctx, priv, db, indexEndpoint, ipfsEndpoint, gorseEndpoint, startTimestamp)
}
