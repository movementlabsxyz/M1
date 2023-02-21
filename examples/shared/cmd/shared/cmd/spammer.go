// Copyright (C) 2023, Ava Labs, Inc. All rights reserved.
// See the file LICENSE for licensing terms.

package cmd

import (
	"context"
	"errors"

	"github.com/ava-labs/hypersdk/crypto"
	"github.com/ava-labs/indexvm/examples/shared/spammer"
	"github.com/spf13/cobra"
)

func init() {
	spamCmd.PersistentFlags().BoolVar(
		&persist,
		"persist",
		false,
		"should persist created items",
	)
	spamCmd.PersistentFlags().IntVar(
		&contentSize,
		"content-size",
		10,
		"size of content randomly generated",
	)
}

var spamCmd = &cobra.Command{
	Use:   "spam <endpoint 1> ... [options]",
	Short: "spam",
	Long: `
TODO

$ shared spam

`,
	PreRunE: func(_ *cobra.Command, args []string) error {
		if len(args) == 0 {
			return errors.New("no endpoints provided")
		}
		return nil
	},
	RunE: spamFunc,
}

func spamFunc(_ *cobra.Command, args []string) error {
	ctx := context.Background()

	// Load signing key
	priv, err := crypto.LoadKey(privateKeyPath)
	if err != nil {
		return err
	}
	return spammer.Run(ctx, priv, args, persist, contentSize)
}
