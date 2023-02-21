// Copyright (C) 2023, Ava Labs, Inc. All rights reserved.
// See the file LICENSE for licensing terms.

package cmd

import (
	"context"
	"path/filepath"

	"github.com/spf13/cobra"

	"github.com/ava-labs/hypersdk/crypto"
	"github.com/ava-labs/hypersdk/pebble"

	"github.com/ava-labs/indexvm/examples/nftdisco/viewer"
)

func init() {
	viewerCmd.PersistentFlags().StringVar(
		&indexEndpoint,
		"indexvm-endpoint",
		"",
		"RPC endpoint for indexvm",
	)
	viewerCmd.PersistentFlags().StringVar(
		&servicerEndpoint,
		"servicer-endpoint",
		"http://localhost:10000/rpc",
		"RPC endpoint for servicer",
	)
	viewerCmd.PersistentFlags().IntVar(
		&pending,
		"pending",
		5,
		"number of pending items to allow",
	)
	viewerCmd.PersistentFlags().StringVar(
		&privateKeyPath,
		"private-key-path",
		filepath.Join(workDir, ".nftdisco.pk"),
		"private key file path",
	)
	viewerCmd.PersistentFlags().StringVar(
		&ipfsDirPath,
		"ipfs-dir-path",
		filepath.Join(workDir, ".ipfs"),
		"IPFS directory path",
	)
	viewerCmd.PersistentFlags().StringVar(
		&viewerDirPath,
		"viewer-dir-path",
		filepath.Join(workDir, ".viewer"),
		"viewer directory path",
	)
}

var viewerCmd = &cobra.Command{
	Use:   "viewer [options]",
	Short: "viewer",
	Long: `
TODO

$ nftdisco viewer

`,
	RunE: viewerFunc,
}

func viewerFunc(*cobra.Command, []string) error {
	ctx := context.Background()

	// Load Key (or generate new one)
	priv, err := crypto.LoadKey(privateKeyPath)
	if err != nil {
		return err
	}

	// Load database
	db, err := pebble.New(viewerDirPath, pebble.NewDefaultConfig())
	if err != nil {
		return err
	}
	defer db.Close()
	return viewer.Run(ctx, priv, db, ipfsDirPath, servicerEndpoint, indexEndpoint, pending)
}
