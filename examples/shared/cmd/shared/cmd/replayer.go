// Copyright (C) 2023, Ava Labs, Inc. All rights reserved.
// See the file LICENSE for licensing terms.

package cmd

import (
	"context"
	"path/filepath"

	"github.com/ava-labs/hypersdk/crypto"
	"github.com/ava-labs/hypersdk/pebble"
	"github.com/ava-labs/indexvm/examples/shared/replayer"
	"github.com/spf13/cobra"
)

func init() {
	replayerCmd.PersistentFlags().StringVar(
		&indexEndpoint,
		"indexvm-endpoint",
		"",
		"RPC endpoint for indexvm",
	)
	replayerCmd.PersistentFlags().StringVar(
		&privateKeyPath,
		"private-key-path",
		filepath.Join(workDir, ".shared.pk"),
		"private key file path",
	)
	replayerCmd.PersistentFlags().StringVar(
		&replayerDirPath,
		"replayer-dir-path",
		filepath.Join(workDir, ".replayer"),
		"replayerdirectory path",
	)
}

var replayerCmd = &cobra.Command{
	Use:   "replayer [options]",
	Short: "replayer",
	Long: `
TODO

$ shared replayer

`,
	RunE: replayerFunc,
}

func replayerFunc(*cobra.Command, []string) error {
	ctx := context.Background()

	// Load signing key
	priv, err := crypto.LoadKey(privateKeyPath)
	if err != nil {
		return err
	}

	// Load database
	db, err := pebble.New(replayerDirPath, pebble.NewDefaultConfig())
	if err != nil {
		return err
	}
	defer db.Close()
	return replayer.Run(ctx, priv, db, indexEndpoint)
}
