// Copyright (C) 2023, Ava Labs, Inc. All rights reserved.
// See the file LICENSE for licensing terms.

package client

import (
	"context"
	"encoding/binary"
	"sync"
	"time"

	"github.com/ava-labs/avalanchego/ids"
	"github.com/ava-labs/hypersdk/crypto"
	"github.com/ava-labs/hypersdk/requester"
	"github.com/ava-labs/indexvm/examples/shared/servicer"
)

type Client struct {
	requester  *requester.EndpointRequester
	privateKey crypto.PrivateKey

	heartbeatL sync.RWMutex
	heartbeat  uint64
	signature  crypto.Signature
}

func New(endpoint string, privateKey crypto.PrivateKey) *Client {
	c := &Client{requester: requester.New(endpoint, "shared"), privateKey: privateKey}
	go c.updateHeartbeat()
	return c
}

func (c *Client) setHeartbeat() {
	heartbeat, err := c.GetHeartbeat()
	if err != nil {
		panic(err)
	}

	c.heartbeatL.Lock()
	defer c.heartbeatL.Unlock()
	c.heartbeat = heartbeat
	c.signature = crypto.Sign(binary.BigEndian.AppendUint64(nil, c.heartbeat), c.privateKey)
}

func (c *Client) updateHeartbeat() {
	ticker := time.NewTicker(30 * time.Second)
	defer ticker.Stop()

	c.setHeartbeat()
	for {
		select {
		case <-ticker.C:
			c.setHeartbeat()
		}
	}
}

func (c *Client) GetHeartbeat() (uint64, error) {
	resp := new(servicer.GetHeartbeatReply)
	err := c.requester.SendRequest(
		context.TODO(),
		"GetHeartbeat",
		nil,
		resp,
	)
	if err != nil {
		return 0, err
	}
	return resp.Heartbeat, nil
}

func (c *Client) GetRecommendation(schema ids.ID) (*servicer.GetRecommendationReply, error) {
	c.heartbeatL.RLock()
	args := &servicer.GetRecommendationArgs{
		Heartbeat: c.heartbeat,
		Actor:     c.privateKey.PublicKey(),
		Signature: c.signature,

		Schema: schema,
	}
	c.heartbeatL.RUnlock()

	resp := new(servicer.GetRecommendationReply)
	err := c.requester.SendRequest(
		context.TODO(),
		"GetRecommendation",
		args,
		resp,
	)
	return resp, err
}

func (c *Client) GetUnrated() (*servicer.GetUnratedReply, error) {
	c.heartbeatL.Lock()
	args := &servicer.GetUnratedArgs{
		Heartbeat: c.heartbeat,
		Actor:     c.privateKey.PublicKey(),
		Signature: c.signature,
	}
	c.heartbeatL.Unlock()

	resp := new(servicer.GetUnratedReply)
	err := c.requester.SendRequest(
		context.TODO(),
		"GetUnrated",
		args,
		resp,
	)
	return resp, err
}
