// Copyright (C) 2023, Ava Labs, Inc. All rights reserved.
// See the file LICENSE for licensing terms.

package cmd

import (
	"context"
	"path/filepath"

	"github.com/spf13/cobra"

	"github.com/ava-labs/hypersdk/pebble"
	"github.com/ava-labs/indexvm/examples/shared/ingestor"
)

func init() {
	ingestCmd.PersistentFlags().StringVar(
		&indexEndpoint,
		"indexvm-endpoint",
		"",
		"RPC endpoint for indexvm",
	)
	ingestCmd.PersistentFlags().StringVar(
		&gorseEndpoint,
		"gorse-endpoint",
		"",
		"RPC endpoint for gorse",
	)
	ingestCmd.PersistentFlags().StringVar(
		&servicer,
		"servicer",
		"",
		"servicer to listen for",
	)
	ingestCmd.PersistentFlags().StringVar(
		&commission,
		"commission",
		"0",
		"minimum commission for servicer to be considered valid",
	)
	ingestCmd.PersistentFlags().StringVar(
		&replayerDirPath,
		"replayer-dir-path",
		filepath.Join(workDir, ".replayer"),
		"replayerdirectory path",
	)
}

var ingestCmd = &cobra.Command{
	Use:   "ingest [options]",
	Short: "ingest",
	Long: `
TODO

$ shared ingest

`,
	RunE: ingestFunc,
}

func ingestFunc(*cobra.Command, []string) error {
	ctx := context.Background()

	// Load database
	db, err := pebble.New(replayerDirPath, pebble.NewDefaultConfig())
	if err != nil {
		return err
	}
	defer db.Close()
	return ingestor.Run(ctx, db, commission, servicer, gorseEndpoint, indexEndpoint)
}
