// Copyright (C) 2023, Ava Labs, Inc. All rights reserved.
// See the file LICENSE for licensing terms.

package replayer

import (
	"context"
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
	iutils "github.com/ava-labs/indexvm/utils"
	"golang.org/x/sync/errgroup"
)

func Run(
	ctx context.Context,
	priv crypto.PrivateKey,
	db database.Database,
	indexEndpoint string,
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

	// Configure worker groups
	var pendingL sync.Mutex
	pending := map[ids.ID]*actions.Index{}
	c := make(chan *actions.Index, 1024)
	eg, egctx := errgroup.WithContext(ctx)

	// Catch interrupt for smooth exit
	cl := make(chan os.Signal, 1)
	signal.Notify(cl, os.Interrupt)
	eg.Go(func() error {
		select {
		case <-egctx.Done():
			return nil
		case <-cl:
			utils.Outf("{{red}}starting shutdown...{{/}}\n")
			return context.Canceled
		}
	})

	// Enqueue restored txs
	eg.Go(func() error {
		err := Restore(egctx, db, func(item *actions.Index) error {
			utils.Outf("{{yellow}}restoring:{{/}} %s\n", item.ContentID())
			c <- item
			return nil
		})
		if err != nil {
			return err
		}
		utils.Outf("{{green}}replayed all items{{/}}\n")
		return context.Canceled
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
			case action := <-c:
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
				_, tx, fees, err := cli.GenerateTransaction(egctx, action, auth)
				if err != nil {
					return err
				}

				pendingL.Lock()
				pending[tx.ID()] = action
				pendingL.Unlock()

				for len(pending) > 256 {
					if egctx.Err() != nil {
						return nil
					}
					// Don't allow too many to be pending at once
					utils.Outf("{{yellow}}sleeping until less pending...{{/}}\n")
					time.Sleep(1 * time.Second)
				}

				// Ensure enough balance
				reqBalance := fees + gen.StateLockup
				if err := cli.WaitForBalance(egctx, address, reqBalance); err != nil {
					return err
				}

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
