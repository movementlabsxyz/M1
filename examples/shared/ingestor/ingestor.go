// Copyright (C) 2023, Ava Labs, Inc. All rights reserved.
// See the file LICENSE for licensing terms.

package ingestor

import (
	"context"
	"encoding/base64"
	"fmt"
	"time"

	"github.com/ava-labs/avalanchego/database"
	"github.com/ava-labs/hypersdk/crypto"
	"github.com/ava-labs/hypersdk/utils"
	"github.com/ava-labs/hypersdk/vm"
	"github.com/ava-labs/indexvm/actions"
	"github.com/ava-labs/indexvm/auth"
	iclient "github.com/ava-labs/indexvm/client"
	"github.com/ava-labs/indexvm/examples/shared/consts"
	"github.com/ava-labs/indexvm/examples/shared/gorse"
	"github.com/ava-labs/indexvm/examples/shared/replayer"
	iutils "github.com/ava-labs/indexvm/utils"
)

func Run(
	ctx context.Context,
	db database.Database,
	commission string,
	servicer string,
	gorseEndpoint string,
	indexEndpoint string,
) error {
	// Prepare servicer
	var pk crypto.PublicKey
	pcommission, err := utils.ParseBalance(commission)
	if err != nil {
		return err
	}
	if pcommission > 0 {
		var err error
		pk, err = iutils.ParseAddress(servicer)
		if err != nil {
			return err
		}
	}

	// Create gorse client
	g := gorse.New(gorseEndpoint, "", 10*time.Second)

	// Create indexvm clients
	cli := iclient.New(indexEndpoint)
	port, err := cli.BlocksPort(ctx)
	if err != nil {
		return err
	}
	host, err := utils.GetHost(indexEndpoint)
	if err != nil {
		return err
	}
	scli, err := vm.NewBlockRPCClient(fmt.Sprintf("%s:%d", host, port))
	if err != nil {
		return err
	}
	defer scli.Close()
	parser, err := cli.Parser(ctx)
	if err != nil {
		return err
	}
	utils.Outf("{{yellow}}listening for new transactions...{{/}}\n")

	// Compute schema
	for {
		blk, results, err := scli.Listen(parser)
		if err != nil {
			return err
		}

		var (
			items   = 0
			ratings = 0
		)
		for i, tx := range blk.Txs {
			ractor := auth.GetActor(tx.Auth)
			actor := iutils.Address(ractor)
			if !results[i].Success {
				// Skip any failed txs
				continue
			}
			switch action := tx.Action.(type) {
			case *actions.Index:
				switch action.Schema {
				case consts.SpamDataSchemaID:
					// Ignore spam
					continue
				case consts.RatingSchemaID:
					// Check if previous recommended, if so enforce servicer (otherwise
					// could be from some other platform)
					unrated, err := g.Unrated(context.TODO(), actor)
					if err != nil {
						utils.Outf("{{yellow}}unable to check pending: %v{{/}}\n", err)
						continue
					}
					pending := false
					parent := action.Parent.String()
					for _, item := range unrated {
						if item.ItemId == parent {
							pending = true
							break
						}
					}

					// Enforce servicer requirement if came from us
					if pending && pk != crypto.EmptyPublicKey && pk != action.Servicer && pcommission > action.Commission {
						utils.Outf("{{yellow}}tx did not pay valid commission: %s{{/}}\n", tx.ID().String())
						continue
					}

					if !pending {
						utils.Outf("{{yellow}}rated item %s was not pending{{/}}: %s %s\n", parent, actor, tx.ID())
					} else {
						// This is required to ensure we don't have an increase in unrated
						//
						// TODO: figure out why PUT doesn't work
						if err := g.DeleteSeen(context.TODO(), actor, parent); err != nil {
							utils.Outf("{{yellow}}unable to delete seen: %v{{/}}\n", err)
							continue
						}
					}

					// Parse rating
					feedback, err := gorse.ParseFeedback(action.Content)
					if err != nil {
						utils.Outf("{{yellow}}unable to parse feedback: %v{{/}}\n", err)
						continue
					}

					// Submit rating
					if err := g.Rate(context.TODO(), actor, parent, feedback); err != nil {
						utils.Outf("{{yellow}}unable to submit feedback: %v{{/}}\n", err)
						continue
					}
					ratings++
				default: // if not ratings, assume useful data
					// Ensure we aren't overwriting data
					has, err := g.Has(context.TODO(), action.ContentID().String())
					if err != nil {
						utils.Outf("{{yellow}}unable to check if content exists: %v{{/}}\n", err)
						continue
					}
					if has {
						utils.Outf("{{yellow}}skipping %s because it already exists{{/}}\n", action.ContentID())
						continue
					}

					// Insert data
					if err := g.Insert(context.TODO(), action.ContentID().String(), base64.StdEncoding.EncodeToString(action.Content), action.Schema.String(), actor); err != nil {
						utils.Outf("{{yellow}}unable to insert content: %v{{/}}\n", err)
						continue
					}
					items++

					// Backup data
					if err := replayer.Backup(ctx, db, tx.ID(), action); err != nil {
						utils.Outf("{{orange}}unable to backup tx:{{/}} %s\n", tx.ID())
						continue
					}
				}
			default:
				utils.Outf("{{yellow}}ignoring tx of type %T{{/}}\n", action)
			}
		}
		utils.Outf("{{green}}block %d had %d items and %d ratings{{/}}\n", blk.Hght, items, ratings)
	}
}
