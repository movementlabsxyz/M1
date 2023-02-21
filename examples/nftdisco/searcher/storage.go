// Copyright (C) 2023, Ava Labs, Inc. All rights reserved.
// See the file LICENSE for licensing terms.

package searcher

import (
	"context"

	"github.com/ava-labs/avalanchego/database"
)

const (
	contractBypass = 0x0
)

func StoreContract(ctx context.Context, db database.Database, contract string) error {
	k := make([]byte, 1+len(contract))
	k[0] = contractBypass
	copy(k[1:], []byte(contract))
	return db.Put(k, nil)
}

func HasContract(ctx context.Context, db database.Database, contract string) (bool, error) {
	k := make([]byte, 1+len(contract))
	k[0] = contractBypass
	copy(k[1:], []byte(contract))
	return db.Has(k)
}
