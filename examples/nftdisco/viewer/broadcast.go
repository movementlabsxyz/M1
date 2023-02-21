// Copyright (C) 2023, Ava Labs, Inc. All rights reserved.
// See the file LICENSE for licensing terms.

package viewer

import (
	"context"

	"github.com/ava-labs/avalanchego/ids"
	"github.com/ava-labs/hypersdk/utils"
	"github.com/ava-labs/indexvm/actions"
	"github.com/ava-labs/indexvm/examples/shared/consts"
	"github.com/ava-labs/indexvm/examples/shared/gorse"
	"github.com/ava-labs/indexvm/examples/shared/servicer"
	iutils "github.com/ava-labs/indexvm/utils"
)

type pendingTx struct {
	itx     *actions.Index
	royalty uint64
}

func (v *Viewer) submitRating(
	ctx context.Context,
	work *servicer.GetRecommendationReply,
	feedback gorse.FeedbackType,
) error {
	pid, err := ids.FromString(work.ID)
	if err != nil {
		return err
	}
	action := &actions.Index{
		Parent:  pid,
		Schema:  consts.RatingSchemaID,
		Content: feedback.Value(),
	}
	searcher, royalty, err := v.icli.Content(ctx, pid)
	if err != nil {
		return err
	}
	if len(searcher) > 0 {
		pk, err := iutils.ParseAddress(searcher)
		if err != nil {
			return err
		}
		action.Searcher = pk
	}
	if work.Commission > 0 {
		// Commission is optional
		action.Servicer = work.Servicer
		action.Commission = work.Commission
	}
	select {
	case v.txc <- &pendingTx{action, royalty}:
	case <-ctx.Done():
	}
	return nil
}

func (v *Viewer) broadcastTransactions(ctx context.Context) error {
	for {
		select {
		case pending := <-v.txc:
			_, tx, fee, err := v.icli.GenerateTransaction(ctx, pending.itx, v.auth)
			if err != nil {
				utils.Outf("{{red}}tx generation error:{{/}} %v\n", err)
				return err
			}
			required := fee + pending.royalty + pending.itx.Commission
			if err := v.icli.WaitForBalance(ctx, v.addr, required); err != nil {
				return err
			}
			v.pendingL.Lock()
			v.pendingM[tx.ID()] = pending
			v.pendingL.Unlock()
			if err := v.scli.IssueTx(tx); err != nil {
				utils.Outf("{{red}}tx issue error:{{/}} %v\n", err)
				return err
			}
			v.issued++
		case <-ctx.Done():
			return nil
		}
	}
}

func (v *Viewer) confirmTransactions(ctx context.Context) error {
	go func() {
		// Closing the connection will unblock anything waiting
		<-ctx.Done()
		v.scli.Close()
	}()

	for ctx.Err() == nil {
		// TODO: pass context
		id, confirmed, result, err := v.scli.Listen()
		if err != nil {
			return err
		}

		// Track txs in-flight for retry
		v.pendingL.Lock()
		meta := v.pendingM[id]
		delete(v.pendingM, id)
		v.pendingL.Unlock()

		// Re-send tx
		if confirmed != nil || !result.Success {
			v.txc <- meta
		}
	}
	return nil
}
