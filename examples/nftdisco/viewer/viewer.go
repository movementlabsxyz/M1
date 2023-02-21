// Copyright (C) 2023, Ava Labs, Inc. All rights reserved.
// See the file LICENSE for licensing terms.

package viewer

import (
	"context"
	"errors"
	"fmt"
	"os"
	"os/signal"
	"sync"

	"github.com/ava-labs/avalanchego/database"
	"github.com/ava-labs/avalanchego/ids"
	"github.com/ava-labs/hypersdk/chain"
	"github.com/ava-labs/hypersdk/crypto"
	"github.com/ava-labs/hypersdk/utils"
	"github.com/ava-labs/hypersdk/vm"
	"github.com/ava-labs/indexvm/auth"
	iclient "github.com/ava-labs/indexvm/client"
	"github.com/ava-labs/indexvm/examples/nftdisco/thegraph"
	"github.com/ava-labs/indexvm/examples/shared/client"
	"github.com/ava-labs/indexvm/examples/shared/ipfs"
	iutils "github.com/ava-labs/indexvm/utils"
	"github.com/inancgumus/screen"
	icore "github.com/ipfs/interface-go-ipfs-core"
	"golang.org/x/sync/errgroup"
)

type Viewer struct {
	pending int
	priv    crypto.PrivateKey
	auth    chain.AuthFactory
	addr    string
	icli    *iclient.Client
	cli     *client.Client
	scli    *vm.DecisionRPCClient
	ipfsAPI icore.CoreAPI

	db database.Database

	filters  []*filter
	filtersL sync.Mutex
	nfts     []*thegraph.NFT // don't need to lock

	autoRateLogs chan string // only log when processing

	recs     chan *rec
	txc      chan *pendingTx
	pendingL sync.Mutex
	pendingM map[ids.ID]*pendingTx
	seenL    sync.Mutex
	seenM    map[string]struct{}

	issued int
}

func Run(
	ctx context.Context,
	priv crypto.PrivateKey,
	db database.Database,
	ipfsDir string,
	servicerEndpoint string,
	indexEndpoint string,
	pending int,
) error {
	v := &Viewer{
		priv:         priv,
		auth:         auth.NewDirectFactory(priv),
		pending:      pending,
		db:           db,
		filters:      []*filter{},
		nfts:         []*thegraph.NFT{},
		autoRateLogs: make(chan string, 128),
		recs:         make(chan *rec),
		txc:          make(chan *pendingTx),
		pendingM:     map[ids.ID]*pendingTx{},
		seenM:        map[string]struct{}{},
	}

	// Load filters
	lfilters, err := LoadFilters(ctx, v.db)
	if err != nil {
		return err
	}
	v.filters = lfilters
	utils.Outf("{{yellow}}loaded filters:{{/}} %d\n", len(v.filters))

	// Load saved NFTs
	lnfts, err := LoadNFTs(ctx, v.db)
	if err != nil {
		return err
	}
	v.nfts = lnfts
	utils.Outf("{{yellow}}loaded nfts:{{/}} %d\n", len(v.nfts))

	// Create IPFS node
	ipfsAPI, ipfsNode, err := ipfs.New(ctx, ipfsDir)
	if err != nil {
		utils.Outf("{{red}}unable to start IPFS node:{{/}}\n")
		return err
	}
	defer ipfsNode.Close()
	v.ipfsAPI = ipfsAPI

	// Create recommendation client
	c := client.New(servicerEndpoint, priv)
	v.cli = c

	// Create indexvm client
	icli := iclient.New(indexEndpoint)
	v.icli = icli
	port, err := icli.DecisionsPort(ctx)
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
	v.scli = scli

	// Orient user
	screen.Clear()
	screen.MoveTopLeft()

	// Get balance
	pk := priv.PublicKey()
	addr := iutils.Address(pk)
	v.addr = addr
	unlocked, _, _, err := icli.Balance(ctx, addr)
	if err != nil {
		return err
	}

	utils.Outf("{{green}}loaded identity:{{/}} %s\n", addr)
	utils.Outf("{{green}}loaded balance:{{/}} %s\n", utils.FormatBalance(unlocked))
	utils.Outf("{{green}}started IPFS node:{{/}} %s\n\n", ipfsNode.Identity.String())
	localAddrs, err := ipfsAPI.Swarm().LocalAddrs(ctx)
	if err != nil {
		return err
	}
	utils.Outf("{{yellow}}local addrs:{{/}} %+v\n", localAddrs)
	listenAddrs, err := ipfsAPI.Swarm().ListenAddrs(ctx)
	if err != nil {
		return err
	}
	utils.Outf("{{yellow}}listen addrs:{{/}} %+v\n", listenAddrs)
	utils.Outf("\n")

	// Wait for a balance before starting (recommender won't allow)
	if err := icli.WaitForBalance(ctx, addr, 10_000); err != nil {
		return err
	}

	g, gctx := errgroup.WithContext(ctx)

	// Catch interrupt for smooth exit
	cl := make(chan os.Signal, 1)
	signal.Notify(cl, os.Interrupt)
	g.Go(func() error {
		select {
		case <-gctx.Done():
			return nil
		case <-cl:
			return context.Canceled
		}
	})

	g.Go(func() error {
		return v.runUI(gctx)
	})

	g.Go(func() error {
		return v.provideRecommendations(gctx)
	})

	// Process transactions
	g.Go(func() error {
		return v.broadcastTransactions(gctx)
	})

	// Confirm and/or retry transactions
	g.Go(func() error {
		return v.confirmTransactions(gctx)
	})

	// Wait for exit or error
	if err := g.Wait(); !errors.Is(err, context.Canceled) {
		return err
	}
	return nil
}
