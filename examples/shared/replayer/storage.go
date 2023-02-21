// Copyright (C) 2023, Ava Labs, Inc. All rights reserved.
// See the file LICENSE for licensing terms.

package replayer

import (
	"context"

	"github.com/ava-labs/avalanchego/database"
	"github.com/ava-labs/avalanchego/ids"
	"github.com/ava-labs/hypersdk/codec"
	"github.com/ava-labs/hypersdk/consts"
	"github.com/ava-labs/indexvm/actions"
)

const (
	actionBackup = 0x0
)

func Backup(ctx context.Context, db database.Database, txID ids.ID, item *actions.Index) error {
	p := codec.NewWriter(actions.MaxContentSize)
	item.Marshal(p)
	if err := p.Err(); err != nil {
		return err
	}
	k := make([]byte, 1+consts.IDLen)
	k[0] = actionBackup
	copy(k[1:], txID[:])
	return db.Put(k, p.Bytes())
}

func Restore(ctx context.Context, db database.Database, f func(item *actions.Index) error) error {
	iter := db.NewIteratorWithPrefix([]byte{actionBackup})
	defer iter.Release()
	for iter.Next() && ctx.Err() == nil {
		p := codec.NewReader(iter.Value(), actions.MaxContentSize)
		action, err := actions.UnmarshalIndex(p)
		if err != nil {
			return err
		}
		index := action.(*actions.Index)
		if err := f(index); err != nil {
			return err
		}
	}
	return nil
}
