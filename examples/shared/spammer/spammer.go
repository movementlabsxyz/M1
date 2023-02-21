// Copyright (C) 2023, Ava Labs, Inc. All rights reserved.
// See the file LICENSE for licensing terms.

package spammer

import (
	"context"
	"crypto/rand"
	"fmt"
	"os"
	"os/signal"
	"sync"
	"time"

	"github.com/ava-labs/avalanchego/ids"
	"github.com/ava-labs/hypersdk/crypto"
	"github.com/ava-labs/hypersdk/utils"
	"github.com/ava-labs/hypersdk/vm"
	"github.com/ava-labs/indexvm/actions"
	"github.com/ava-labs/indexvm/auth"
	iclient "github.com/ava-labs/indexvm/client"
	"github.com/ava-labs/indexvm/examples/shared/consts"
	iutils "github.com/ava-labs/indexvm/utils"
	"golang.org/x/sync/errgroup"
)

func Run(
	ctx context.Context,
	priv crypto.PrivateKey,
	indexEndpoints []string,
	persist bool,
	contentSize int,
) error {
	auth := auth.NewDirectFactory(priv)
	address := iutils.Address(priv.PublicKey())
	utils.Outf("{{blue}}loaded spammer: %s{{/}}\n", address)

	// Ensure enough balance
	// TODO: compute how many txs
	cli := iclient.New(indexEndpoints[0])
	if err := cli.WaitForBalance(ctx, address, 1_000_000_000); err != nil {
		return err
	}

	// Configure worker groups
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
	c := make(chan *actions.Index, 1024)
	// Construct all senders
	for _, tendpoint := range indexEndpoints {
		endpoint := tendpoint
		cli := iclient.New(endpoint)
		port, err := cli.DecisionsPort(context.TODO())
		if err != nil {
			return err
		}
		host, err := utils.GetHost(endpoint)
		if err != nil {
			return err
		}
		scli, err := vm.NewDecisionRPCClient(fmt.Sprintf("%s:%d", host, port))
		if err != nil {
			return err
		}
		defer scli.Close()
		utils.Outf("{{blue}}creating client %s{{/}}\n", endpoint)

		// Confirm and/or retry transactions
		var pendingL sync.Mutex
		pending := map[ids.ID]*actions.Index{}
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
					for len(pending) > 10_000 {
						// We don't want mempool to gett too big
						time.Sleep(10 * time.Millisecond)
					}
					// Issue tx
					// TODO: make way more efficient
					_, tx, _, err := cli.GenerateTransaction(egctx, action, auth)
					if err != nil {
						return err
					}
					pendingL.Lock()
					pending[tx.ID()] = action
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
	}

	eg.Go(func() error {
		for egctx.Err() == nil {
			content := make([]byte, contentSize)
			if _, err := rand.Read(content); err != nil {
				return err
			}
			action := &actions.Index{
				Schema:  consts.SpamDataSchemaID,
				Content: content,
			}
			if persist {
				// If there is a royalty, the tx will claim the [ContentID] in state
				action.Royalty = 1
			}
			c <- action
		}
		return nil
	})

	// Wait for error
	return eg.Wait()
}
