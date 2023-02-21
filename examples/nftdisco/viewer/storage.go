// Copyright (C) 2023, Ava Labs, Inc. All rights reserved.
// See the file LICENSE for licensing terms.

package viewer

import (
	"context"
	"encoding/binary"
	"encoding/json"

	"github.com/ava-labs/avalanchego/database"
	"github.com/ava-labs/indexvm/examples/nftdisco/thegraph"
)

const (
	filterPrefix = 0x0
	nftPrefix    = 0x1
)

func StoreFilter(ctx context.Context, db database.Database, f *filter) error {
	b, err := json.Marshal(f)
	if err != nil {
		return err
	}
	k := binary.BigEndian.AppendUint64([]byte{filterPrefix}, uint64(f.CreatedAt))
	return db.Put(k, b)
}

func RemoveFilter(ctx context.Context, db database.Database, t int64) error {
	k := binary.BigEndian.AppendUint64([]byte{filterPrefix}, uint64(t))
	return db.Delete(k)
}

func LoadFilters(ctx context.Context, db database.Database) ([]*filter, error) {
	filters := []*filter{}
	iter := db.NewIteratorWithPrefix([]byte{filterPrefix})
	defer iter.Release()
	for iter.Next() {
		var f filter
		if err := json.Unmarshal(iter.Value(), &f); err != nil {
			return nil, err
		}
		filters = append(filters, &f)
	}
	return filters, iter.Error()
}

func StoreNFT(ctx context.Context, db database.Database, n *thegraph.NFT) error {
	b, err := json.Marshal(n)
	if err != nil {
		return err
	}
	return db.Put(append([]byte{nftPrefix}, n.ID()...), b)
}

func RemoveNFT(ctx context.Context, db database.Database, id string) error {
	return db.Delete(append([]byte{nftPrefix}, id...))
}

func LoadNFTs(ctx context.Context, db database.Database) ([]*thegraph.NFT, error) {
	nfts := []*thegraph.NFT{}
	iter := db.NewIteratorWithPrefix([]byte{nftPrefix})
	defer iter.Release()
	for iter.Next() {
		var nft thegraph.NFT
		if err := json.Unmarshal(iter.Value(), &nft); err != nil {
			return nil, err
		}
		nfts = append(nfts, &nft)
	}
	return nfts, iter.Error()
}
