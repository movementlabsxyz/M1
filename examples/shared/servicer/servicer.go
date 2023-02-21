// Copyright (C) 2023, Ava Labs, Inc. All rights reserved.
// See the file LICENSE for licensing terms.

package servicer

import (
	"crypto/rand"
	"encoding/binary"
	"fmt"
	"math/big"
	"net/http"
	"sync"
	"time"

	"github.com/gorilla/rpc/v2"
	json "github.com/gorilla/rpc/v2/json2"
	"github.com/rs/cors"

	"github.com/ava-labs/avalanchego/ids"
	"github.com/ava-labs/hypersdk/consts"
	"github.com/ava-labs/hypersdk/crypto"
	"github.com/ava-labs/hypersdk/utils"

	"github.com/ava-labs/indexvm/client"
	iutils "github.com/ava-labs/indexvm/utils"

	"github.com/ava-labs/indexvm/examples/shared/gorse"
)

type DiscoService struct {
	icli *client.Client
	g    *gorse.Client

	heartbeatsL sync.RWMutex
	heartbeats  *utils.BoundedBuffer[uint64]

	servicer   crypto.PublicKey
	commission uint64
	pending    int
}

func (d *DiscoService) setHeartbeat() {
	next, err := rand.Int(rand.Reader, new(big.Int).SetUint64(consts.MaxUint64))
	if err != nil {
		panic(err)
	}

	d.heartbeatsL.Lock()
	d.heartbeats.Insert(next.Uint64())
	d.heartbeatsL.Unlock()
}

func (d *DiscoService) checkHeartbeat(i uint64) bool {
	d.heartbeatsL.RLock()
	defer d.heartbeatsL.RUnlock()

	for _, item := range d.heartbeats.Items() {
		if item == i {
			return true
		}
	}
	return false
}

func (d *DiscoService) updateHeartbeat() {
	ticker := time.NewTicker(30 * time.Second)
	defer ticker.Stop()

	d.setHeartbeat()
	for {
		select {
		case <-ticker.C:
			d.setHeartbeat()
		}
	}
}

type GetHeartbeatReply struct {
	Heartbeat uint64
}

func (d *DiscoService) GetHeartbeat(
	_ *http.Request,
	_ *struct{},
	reply *GetHeartbeatReply,
) error {
	d.heartbeatsL.RLock()
	defer d.heartbeatsL.RUnlock()

	last, ok := d.heartbeats.Last()
	if !ok {
		return ErrMissingHeartbeat
	}
	reply.Heartbeat = last
	return nil
}

type GetRecommendationArgs struct {
	Heartbeat uint64
	Actor     crypto.PublicKey
	Signature crypto.Signature

	Schema ids.ID
}

type GetRecommendationReply struct {
	ID      string
	Content string

	Servicer   crypto.PublicKey
	Commission uint64
}

func (d *DiscoService) GetRecommendation(
	r *http.Request,
	args *GetRecommendationArgs,
	reply *GetRecommendationReply,
) error {
	// Verify request
	if !d.checkHeartbeat(args.Heartbeat) {
		return ErrMissingHeartbeat
	}
	if !crypto.Verify(
		binary.BigEndian.AppendUint64(nil, args.Heartbeat),
		args.Actor,
		args.Signature,
	) {
		return crypto.ErrInvalidSignature
	}

	// Produce Recommendation
	addr := iutils.Address(args.Actor)
	unrated, err := d.g.Unrated(r.Context(), addr)
	if err != nil {
		return err
	}
	unlocked, _, _, err := d.icli.Balance(r.Context(), addr)
	if err != nil {
		return err
	}

	// If unable to pay for a recommendation, don't recommend
	//
	// Because funds must be locked for each new account, this deters sybil
	// attacks.
	requiredFunds := d.commission*uint64(len(unrated)) + 1
	if unlocked < requiredFunds {
		return fmt.Errorf("insufficient funds (have=%d, required=%d", unlocked, requiredFunds)
	}

	// If unpaid debt is significant and have money, replay one of unpaid until all paid
	if len(unrated) >= d.pending {
		n, err := rand.Int(rand.Reader, big.NewInt(int64(len(unrated))))
		if err != nil {
			return err
		}
		item := unrated[n.Int64()]
		content, err := d.g.Get(r.Context(), item.ItemId)
		if err != nil {
			return err
		}
		reply.ID = item.ItemId
		reply.Content = content
		reply.Servicer = d.servicer
		reply.Commission = d.commission
		utils.Outf(
			"{{red}}%s has %d unrated items, replaying:{{/}} %s\n",
			addr,
			len(unrated),
			item.ItemId,
		)
		return nil
	}
	id, content, err := d.g.Recommend(r.Context(), addr, args.Schema.String())
	if err != nil {
		return err
	}
	utils.Outf("{{yellow}}serving recommendation:{{/}}%s to %s\n", id, addr)
	reply.ID = id
	reply.Content = content
	reply.Servicer = d.servicer
	reply.Commission = d.commission
	return nil
}

type GetUnratedArgs struct {
	Heartbeat uint64
	Actor     crypto.PublicKey
	Signature crypto.Signature
}

type GetUnratedReply struct {
	Unrated []*GetRecommendationReply
}

func (d *DiscoService) GetUnrated(
	r *http.Request,
	args *GetUnratedArgs,
	reply *GetUnratedReply,
) error {
	// Verify request
	if !d.checkHeartbeat(args.Heartbeat) {
		return ErrMissingHeartbeat
	}
	if !crypto.Verify(
		binary.BigEndian.AppendUint64(nil, args.Heartbeat),
		args.Actor,
		args.Signature,
	) {
		return crypto.ErrInvalidSignature
	}

	// Get Unrated Items
	addr := iutils.Address(args.Actor)
	unrated, err := d.g.Unrated(r.Context(), addr)
	if err != nil {
		return err
	}
	simpleUnrated := make([]*GetRecommendationReply, len(unrated))
	for i, u := range unrated {
		content, err := d.g.Get(r.Context(), u.ItemId)
		if err != nil {
			return err
		}
		simpleUnrated[i] = &GetRecommendationReply{
			ID:         u.ItemId,
			Content:    content,
			Servicer:   d.servicer,
			Commission: d.commission,
		}
	}
	reply.Unrated = simpleUnrated
	return nil
}

func Run(
	name string,
	port int,
	icli *client.Client,
	g *gorse.Client,
	servicer crypto.PublicKey,
	commission uint64,
	pending int,
) {
	s := rpc.NewServer()
	s.RegisterCodec(json.NewCodec(), "application/json")
	s.RegisterCodec(json.NewCodec(), "application/json;charset=UTF-8")

	service := &DiscoService{
		icli:       icli,
		g:          g,
		heartbeats: utils.NewBoundedBuffer[uint64](10, nil),
		servicer:   servicer,
		commission: commission,
		pending:    pending,
	}
	go service.updateHeartbeat()

	s.RegisterService(service, name)
	corsHandler := cors.New(cors.Options{
		AllowedOrigins:   []string{"*"},
		AllowCredentials: true,
	}).Handler(s)
	http.Handle("/rpc", corsHandler)
	fmt.Printf("listening for recommendation requests on port %d\n", port)
	http.ListenAndServe(fmt.Sprintf("localhost:%d", port), nil)
}
