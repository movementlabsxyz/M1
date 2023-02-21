// Copyright (C) 2023, Ava Labs, Inc. All rights reserved.
// See the file LICENSE for licensing terms.

package viewer

import (
	"context"
	"encoding/base64"
	"time"

	"github.com/ava-labs/indexvm/examples/nftdisco/thegraph"
	"github.com/ava-labs/indexvm/examples/shared/consts"
	"github.com/ava-labs/indexvm/examples/shared/gorse"
	"github.com/ava-labs/indexvm/examples/shared/ipfs"
	"github.com/ava-labs/indexvm/examples/shared/servicer"
	"golang.org/x/sync/errgroup"
)

type rec struct {
	reply *servicer.GetRecommendationReply

	nft *thegraph.NFT
	img []byte
}

func (v *Viewer) provideRecommendations(ctx context.Context) error {
	g, gctx := errgroup.WithContext(ctx)

	// Feed processors
	w := make(chan *servicer.GetRecommendationReply)
	g.Go(func() error {
		// Start with serving unrated items
		unrated, err := v.cli.GetUnrated()
		if err != nil {
			return err
		}
		for _, un := range unrated.Unrated {
			select {
			case w <- un:
			case <-gctx.Done():
				return nil
			}
		}

		// Fetch new recommendations
		for gctx.Err() == nil {
			rec, err := v.cli.GetRecommendation(consts.NFTDataSchemaID)
			if err != nil {
				return err
			}
			v.seenL.Lock()
			if _, ok := v.seenM[rec.ID]; ok {
				v.seenL.Unlock()
				time.Sleep(2 * time.Second)
				continue
			}
			v.seenM[rec.ID] = struct{}{}
			v.seenL.Unlock()
			select {
			case w <- rec:
			case <-gctx.Done():
				return nil
			}
		}
		return nil
	})

	// Start processors
	for i := 0; i < v.pending; i++ {
		g.Go(func() error {
			for {
				select {
				case <-gctx.Done():
					return nil
				case work := <-w:
					// Decode raw data
					parsedContent, err := base64.StdEncoding.DecodeString(work.Content)
					if err != nil {
						_ = v.submitRating(gctx, work, gorse.Junk)
						continue
					}
					// Parse as NFT
					nft, err := thegraph.Unpack(parsedContent)
					if err != nil {
						_ = v.submitRating(gctx, work, gorse.Junk)
						continue
					}
					// Attempt to skip if filtering
					act := v.processRec(nft)
					if len(act) > 0 {
						_ = v.submitRating(gctx, work, act)
						continue
					}
					// Load image
					tctx, cancel := context.WithTimeout(gctx, 1*time.Minute)
					defer cancel()
					content, err := ipfs.FetchContent(tctx, v.ipfsAPI, nft.Image)
					if err != nil {
						_ = v.submitRating(gctx, work, gorse.Inaccessible)
						continue
					}
					// Send to CLI
					select {
					case v.recs <- &rec{
						reply: work,

						nft: nft,
						img: content,
					}:
					case <-gctx.Done():
						return nil
					}
				}
			}
		})
	}
	return g.Wait()
}
