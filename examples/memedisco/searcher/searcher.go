// Copyright (C) 2023, Ava Labs, Inc. All rights reserved.
// See the file LICENSE for licensing terms.

package searcher

import (
	"context"
	"fmt"
	"os"
	"os/signal"
	"sync"
	"time"

	"github.com/ava-labs/avalanchego/cache"
	"github.com/ava-labs/avalanchego/ids"
	"github.com/ava-labs/hypersdk/crypto"
	"github.com/ava-labs/hypersdk/utils"
	"github.com/ava-labs/hypersdk/vm"
	"github.com/ava-labs/indexvm/actions"
	"github.com/ava-labs/indexvm/auth"
	iclient "github.com/ava-labs/indexvm/client"
	"github.com/ava-labs/indexvm/examples/memedisco/reddit"
	"github.com/ava-labs/indexvm/examples/memedisco/reddit/models"
	"github.com/ava-labs/indexvm/examples/shared/consts"
	iutils "github.com/ava-labs/indexvm/utils"
	ipfs "github.com/ipfs/go-ipfs-api"
	"golang.org/x/sync/errgroup"
)

func Run(
	ctx context.Context,
	priv crypto.PrivateKey,
	indexEndpoint string,
	ipfsEndpoint string,
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

	// Create reddit client
	r, err := reddit.New("memedisco")
	if err != nil {
		return err
	}

	// Crete IPFS client
	ic := ipfs.NewShell(ipfsEndpoint)
	ic.SetTimeout(1 * time.Minute)

	// Configure worker groups
	var pendingL sync.Mutex
	pending := map[ids.ID]*models.Meme{}
	c := make(chan *models.Meme, 1024)
	eg, egctx := errgroup.WithContext(ctx)

	// Catch interrupt for smooth exit
	cl := make(chan os.Signal, 1)
	signal.Notify(cl, os.Interrupt)
	eg.Go(func() error {
		select {
		case <-egctx.Done():
			return nil
		case <-cl:
			utils.Outf("{{red}}starting shutdown (may take 60s)...{{/}}\n")
			return context.Canceled
		}
	})

	enqueuedCache := &cache.LRU{Size: 20_000}
	for _, sub := range []string{"wholesomememes", "memes", "dankmemes", "memeeconomy", "funny"} {
		for _, mode := range []string{"new", "top", "hot"} {
			var (
				s     = sub
				m     = mode
				after = ""
			)
			eg.Go(func() error {
				for egctx.Err() == nil {
					memes, next, err := r.GetNPosts(s, m, after, 100)
					if err != nil {
						utils.Outf("{{orange}}unable to fetch posts:{{/}} %v\n", err)
						return err
					}
					after = next
					utils.Outf(
						"{{green}}fetched %d memes{{/}} sub=%s mode=%s after=%s\n",
						len(memes),
						s,
						m,
						after,
					)
					if len(after) == 0 {
						utils.Outf("{{yellow}}sleeping...{{/}} sub=%s mode=%s\n", s, m)
						time.Sleep(5 * time.Second)
					}

					// Add all items to chain after uploading to IPFS
					added := 0
					for _, meme := range memes {
						if egctx.Err() != nil {
							return nil
						}
						if _, ok := enqueuedCache.Get(meme.Name); ok {
							continue
						}
						enqueuedCache.Put(meme.Name, nil)

						// Store image in IPFS
						cid, err := r.PinMeme(ic, meme)
						if err != nil {
							utils.Outf("{{orange}}unable to upload to IPFS:{{/}} %v\n", err)
							return err
						}
						meme.IPFS = fmt.Sprintf("ipfs://%s", cid)
						c <- meme
						added++
					}
					utils.Outf(
						"{{green}}added %d memes to queue:{{/}} sub=%s mode=%s after=%s\n",
						added,
						s,
						m,
						after,
					)
				}
				return nil
			})
		}
	}

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
				utils.Outf("{{orange}}tx failed:{{/}} %v\n", string(result.Output))
				c <- meta
			}
		}
		return nil
	})

	// Issue txs
	eg.Go(func() error {
		for {
			select {
			case meme := <-c:
				// Construct index tx
				content, err := meme.Pack()
				if err != nil {
					return err
				}
				action := &actions.Index{
					Schema:  consts.MemeDataSchemaID,
					Content: content,
					Royalty: 1, // breakeven after 1024 references
				}

				// Check if already owned
				owner, _, err := cli.Content(egctx, action.ContentID())
				if err != nil {
					return err
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
				_, tx, fees, err := cli.GenerateTransaction(egctx, action, auth)
				if err != nil {
					return err
				}

				// Ensure enough balance
				reqBalance := fees + gen.StateLockup
				if err := cli.WaitForBalance(egctx, address, reqBalance); err != nil {
					return err
				}

				pendingL.Lock()
				pending[tx.ID()] = meme
				pendingL.Unlock()

				// Submit tx
				if err := scli.IssueTx(tx); err != nil {
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
