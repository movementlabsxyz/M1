// Copyright (C) 2023, Ava Labs, Inc. All rights reserved.
// See the file LICENSE for licensing terms.

package cmd

import (
	"github.com/ava-labs/hypersdk/crypto"
	hutils "github.com/ava-labs/hypersdk/utils"
	iclient "github.com/ava-labs/indexvm/client"
	"github.com/ava-labs/indexvm/utils"
	"github.com/spf13/cobra"

	"github.com/ava-labs/indexvm/examples/shared/gorse"
	server "github.com/ava-labs/indexvm/examples/shared/servicer"
)

func init() {
	servicerCmd.PersistentFlags().StringVar(
		&servicer,
		"servicer",
		"",
		"servicer to listen for",
	)
	servicerCmd.PersistentFlags().StringVar(
		&commission,
		"commission",
		"",
		"minimum commission for servicer to be considered valid",
	)
	servicerCmd.PersistentFlags().IntVar(
		&pending,
		"pending",
		10,
		"number of unpaid recommednations to allow",
	)
	servicerCmd.PersistentFlags().IntVar(
		&port,
		"port",
		10_000,
		"port to run server",
	)
	servicerCmd.PersistentFlags().StringVar(
		&indexEndpoint,
		"indexvm-endpoint",
		"",
		"RPC endpoint for indexvm",
	)
	servicerCmd.PersistentFlags().StringVar(
		&gorseEndpoint,
		"gorse-endpoint",
		"",
		"RPC endpoint for gorse",
	)
}

var servicerCmd = &cobra.Command{
	Use:   "servicer [options]",
	Short: "servicer",
	Long: `
TODO

$ shared servicer

`,
	RunE: servicerFunc,
}

func servicerFunc(*cobra.Command, []string) error {
	var pk crypto.PublicKey
	pcommission, err := hutils.ParseBalance(commission)
	if err != nil {
		return err
	}
	if pcommission > 0 {
		var err error
		pk, err = utils.ParseAddress(servicer)
		if err != nil {
			panic(err)
		}
	}

	icli := iclient.New(indexEndpoint)
	g := gorse.New(gorseEndpoint, "", requestTimeout)
	server.Run("shared", port, icli, g, pk, pcommission, pending)
	return nil
}
