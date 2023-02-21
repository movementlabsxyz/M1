// Copyright (C) 2023, Ava Labs, Inc. All rights reserved.
// See the file LICENSE for licensing terms.

package searcher

import (
	"context"
	"encoding/base64"
	"fmt"
	"os"
	"os/signal"
	"sync"
	"time"

	"github.com/ava-labs/avalanchego/database"
	"github.com/ava-labs/avalanchego/ids"
	"github.com/ava-labs/hypersdk/crypto"
	"github.com/ava-labs/hypersdk/utils"
	"github.com/ava-labs/hypersdk/vm"
	"github.com/ava-labs/indexvm/actions"
	"github.com/ava-labs/indexvm/auth"
	iclient "github.com/ava-labs/indexvm/client"
	"github.com/ava-labs/indexvm/examples/nftdisco/thegraph"
	"github.com/ava-labs/indexvm/examples/shared/consts"
	"github.com/ava-labs/indexvm/examples/shared/gorse"
	iutils "github.com/ava-labs/indexvm/utils"
	"golang.org/x/sync/errgroup"
)

func Run(
	ctx context.Context,
	priv crypto.PrivateKey,
	db database.Database,
	indexEndpoint string,
	ipfsEndpoint string,
	gorseEndpoint string,
	startTimestamp int64,
) error {
	auth := auth.NewDirectFactory(priv)
	address := iutils.Address(priv.PublicKey())
	utils.Outf("{{blue}}loaded searcher: %s{{/}}\n", address)

	// Create indexvm clients
	cli := iclient.New(indexEndpoint)
	port, err := cli.DecisionsPort(context.TODO())
	if err != nil {
		return err
	}
	host, err := utils.GetHost(indexEndpoint)
	if err != nil {
		return err
	}
	scli, err := vm.NewDecisionRPCClient(fmt.Sprintf("%s:%d", host, port))
	if err != nil {
		return err
	}
	defer scli.Close()

	// Load genesis info
	gen, err := cli.Genesis(ctx)
	if err != nil {
		return err
	}

	// Create graph client
	g := thegraph.New(ipfsEndpoint, func(contract string) (bool, error) {
		return HasContract(ctx, db, contract)
	}, func(contract string) error {
		return StoreContract(ctx, db, contract)
	})

	// Create gorse client
	gor := gorse.New(gorseEndpoint, "", 10*time.Second)

	// Configure worker groups
	var pendingL sync.Mutex
	pending := map[ids.ID]*thegraph.NFT{}
	c := make(chan *thegraph.NFT, 1024)
	eg, egctx := errgroup.WithContext(ctx)

	// Catch interrupt for smooth exit
	cl := make(chan os.Signal, 1)
	signal.Notify(cl, os.Interrupt)
	eg.Go(func() error {
		select {
		case <-egctx.Done():
			return nil
		case <-cl:
			utils.Outf("{{red}}starting shutdown (may take 180s)...{{/}}\n")
			return context.Canceled
		}
	})

	// Fetch NFTs
	_, _, content, err := gor.Latest(ctx, consts.NFTDataSchemaID.String())
	if err != nil {
		return err
	}
	var timestamp int64
	if len(content) > 0 {
		rcontent, err := base64.StdEncoding.DecodeString(content)
		if err != nil {
			return err
		}
		nft, err := thegraph.Unpack(rcontent)
		if err != nil {
			return err
		}
		// NOTE: ASSUMES WE INGEST IN ORDER
		timestamp = nft.CreatedAt
	}
	if startTimestamp > -1 {
		timestamp = startTimestamp
	}
	utils.Outf("{{yellow}}starting to fetch NFTs from timestamp: %d{{/}}\n", timestamp)
	eg.Go(func() error {
		for egctx.Err() == nil {
			nfts, cont, err := g.Fetch(egctx, timestamp)
			if err != nil {
				return err
			}
			if cont < 0 {
				break
			}
			timestamp = cont

			// Skip if nothing to do
			if len(nfts) == 0 {
				continue
			}

			// Add all items to chain
			nftsAdded := 0
			for _, collectionNFTs := range nfts {
				for _, colnft := range collectionNFTs {
					nftsAdded++
					c <- colnft
				}
			}
			if nftsAdded > 0 {
				utils.Outf("{{green}}added %d nfts to queue{{/}}\n", nftsAdded)
			}
		}
		return nil
	})

	// Confirm and/or retry transactions
	eg.Go(func() error {
		go func() {
			<-egctx.Done()
			_ = scli.Close()
		}()
		for egctx.Err() == nil {
			id, confirmed, result, err := scli.Listen()
			if err != nil {
				return err
			}
			pendingL.Lock()
			meta := pending[id]
			delete(pending, id)
			pendingL.Unlock()

			// Re-send tx
			if confirmed != nil || !result.Success {
				c <- meta
			}
		}
		return nil
	})

	// Issue txs
	eg.Go(func() error {
		for {
			select {
			case nft := <-c:
				// Construct index tx
				content, err := nft.Pack()
				if err != nil {
					return err
				}
				action := &actions.Index{
					Schema:  consts.NFTDataSchemaID,
					Content: content,
					Royalty: 1, // breakeven after 1024 references
				}

				// Check if already owned
				owner, _, err := cli.Content(egctx, action.ContentID())
				if err != nil {
					panic(err)
				}
				if len(owner) > 0 {
					utils.Outf(
						"{{yellow}}%s is already owned by %s{{/}}\n",
						action.ContentID(),
						owner,
					)
					continue
				}

				// Issue tx
				submit, tx, fees, err := cli.GenerateTransaction(egctx, action, auth)
				if err != nil {
					return err
				}

				pendingL.Lock()
				pending[tx.ID()] = nft
				pendingL.Unlock()

				// Ensure enough balance
				reqBalance := fees + gen.StateLockup
				if err := cli.WaitForBalance(egctx, address, reqBalance); err != nil {
					return err
				}

				// Submit tx
				utils.Outf(
					"{{green}}submitting transaction for %s:%d{{/}}\n",
					nft.Contract,
					nft.TokenID,
				)
				if err := submit(egctx); err != nil {
					utils.Outf("{{red}}tx submission failed: %v{{/}}\n", err)
					return err
				}
			case <-egctx.Done():
				return nil
			}
		}
	})

	// Wait for error
	return eg.Wait()
}
