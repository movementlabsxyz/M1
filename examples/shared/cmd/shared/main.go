// Copyright (C) 2023, Ava Labs, Inc. All rights reserved.
// See the file LICENSE for licensing terms.

package main

import (
	"context"
	"errors"
	"os"

	"github.com/ava-labs/hypersdk/utils"

	"github.com/ava-labs/indexvm/examples/shared/cmd/shared/cmd"
)

func main() {
	if err := cmd.Execute(); err != nil && !errors.Is(err, context.Canceled) {
		utils.Outf("{{red}}Exited:{{/}} %v\n", err)
		os.Exit(1)
	}
	os.Exit(0)
}
