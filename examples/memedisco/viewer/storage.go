// Copyright (C) 2023, Ava Labs, Inc. All rights reserved.
// See the file LICENSE for licensing terms.

package viewer

import (
	"context"

	"github.com/ava-labs/avalanchego/database"
	"github.com/ava-labs/indexvm/examples/memedisco/reddit/models"
)

const (
	memePrefix = 0x0
)

func StoreMeme(ctx context.Context, db database.Database, m *models.Meme) error {
	contents, err := m.Pack()
	if err != nil {
		return err
	}
	return db.Put(append([]byte{memePrefix}, m.Name...), contents)
}

func RemoveMeme(ctx context.Context, db database.Database, name string) error {
	return db.Delete(append([]byte{memePrefix}, name...))
}

func LoadMemes(ctx context.Context, db database.Database) ([]*models.Meme, error) {
	memes := []*models.Meme{}
	iter := db.NewIteratorWithPrefix([]byte{memePrefix})
	defer iter.Release()
	for iter.Next() {
		meme, err := models.Unpack(iter.Value())
		if err != nil {
			return nil, err
		}
		memes = append(memes, meme)
	}
	return memes, iter.Error()
}
