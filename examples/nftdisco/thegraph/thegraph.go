// Copyright (C) 2023, Ava Labs, Inc. All rights reserved.
// See the file LICENSE for licensing terms.

package thegraph

import (
	"context"
	"encoding/json"
	"fmt"
	"io"
	"io/ioutil"
	"net/http"
	"strconv"
	"strings"
	"time"

	"github.com/ava-labs/hypersdk/codec"
	"github.com/ava-labs/indexvm/actions"
	"github.com/hasura/go-graphql-client"
	ipfs "github.com/ipfs/go-ipfs-api"
)

const (
	nftURL     = "https://api.thegraph.com/subgraphs/name/traderjoe-xyz/nft-contracts"
	queryLimit = 10
)

type TheGraphClient struct {
	c  *graphql.Client
	hc http.Client
	ic *ipfs.Shell

	shouldSkip func(string) (bool, error)
	failed     func(string) error
}

func New(
	ipfsAddress string,
	shouldSkip func(contract string) (bool, error),
	failed func(contract string) error,
) *TheGraphClient {
	t := http.DefaultTransport.(*http.Transport).Clone()
	t.MaxIdleConns = 100_000
	t.MaxConnsPerHost = 100_000
	ic := ipfs.NewShell(ipfsAddress)
	ic.SetTimeout(3 * time.Minute)
	return &TheGraphClient{
		c: graphql.NewClient(nftURL, nil),
		hc: http.Client{
			Transport: t,
			Timeout:   30 * time.Second,
		},
		ic:         ic,
		shouldSkip: shouldSkip,
		failed:     failed,
	}
}

type NFTQuery struct {
	NFTContracts []struct {
		ID        string `graphql:"id"`
		CreatedAt string `graphql:"createdAt"`
		Name      string `graphql:"name"`
		Symbol    string `graphql:"symbol"`
		NFTs      []struct {
			TokenURI string `graphql:"tokenURI"`
			TokenID  string `graphql:"tokenID"`
		} `graphql:"nfts"`
	} `graphql:"nftContracts(first: 10, orderBy:createdAt, orderDirection:asc, where:{createdAt_gt: $timestamp})"`
	// TODO: handle multiple deploys at same second
}

type NFT struct {
	Contract  string
	TokenID   int
	CreatedAt int64
	Symbol    string

	Name        string
	Description string
	Image       string
}

func (n *NFT) Pack() ([]byte, error) {
	p := codec.NewWriter(actions.MaxContentSize)
	p.PackString(n.Contract)
	p.PackInt(n.TokenID)
	p.PackInt64(n.CreatedAt)
	p.PackString(n.Symbol)
	p.PackString(n.Name)
	p.PackString(n.Description)
	p.PackString(n.Image)
	return p.Bytes(), p.Err()
}

func Unpack(b []byte) (*NFT, error) {
	var n NFT
	p := codec.NewReader(b, actions.MaxContentSize)
	n.Contract = p.UnpackString(true)
	n.TokenID = p.UnpackInt(false) // could be 0
	n.CreatedAt = p.UnpackInt64(true)
	n.Symbol = p.UnpackString(false)
	n.Name = p.UnpackString(true)
	n.Description = p.UnpackString(false)
	n.Image = p.UnpackString(true)
	return &n, p.Err()
}

func (n *NFT) ID() string {
	return fmt.Sprintf("%s-%d", n.Contract, n.TokenID)
}

type NFTMeta struct {
	Name        string `json:"name"`
	Description string `json:"description"`
	Image       string `json:"image"`
}

// Cat the content at the given path. Callers need to drain and close the returned reader after usage.
// Inspired by: https://github.com/ipfs/go-ipfs-api/blob/cb1fca1e60b1fb653ad10dd77f10c48b55fea867/shell.go#L162
func (c *TheGraphClient) Cat(ctx context.Context, path string) (io.ReadCloser, error) {
	resp, err := c.ic.Request("cat", path).Send(ctx)
	if err != nil {
		return nil, err
	}
	if resp.Error != nil {
		return nil, resp.Error
	}

	return resp.Output, nil
}

func fetchMetadata(c *TheGraphClient, tokenURI string) (string, *NFTMeta) {
	metaAddr := strings.TrimPrefix(tokenURI, "ipfs://")
	ctx, cancel := context.WithTimeout(context.Background(), 3*time.Minute)
	defer cancel()
	reader, err := c.Cat(ctx, metaAddr)
	if err != nil {
		fmt.Println("unable to get object", metaAddr, err)
		return "", nil
	}
	defer reader.Close()
	var meta NFTMeta
	r, err := ioutil.ReadAll(reader)
	if err != nil {
		return "", nil // skip entire contract
	}
	if err := json.Unmarshal(r, &meta); err != nil {
		fmt.Println("unable to marshal data", r, err)
		return "", nil // skip entire contract
	}
	return metaAddr, &meta
}

func parseURI(uri string) string {
	// Happy path
	if strings.HasPrefix(uri, "ipfs://") {
		return uri
	}

	// Attempt to overwrite (assume some URL)
	splits := strings.Split(uri, "ipfs/")
	if len(splits) == 2 {
		return fmt.Sprintf("ipfs://%s", splits[1])
	}
	return ""
}

func cleanString(str string) string {
	n := strings.ReplaceAll(str, "™", "")
	n = strings.ReplaceAll(n, "\n\n", " ")
	n = strings.ReplaceAll(n, "\n", " ")
	n = strings.ReplaceAll(n, "®", "")
	n = strings.ReplaceAll(n, "\"", "")
	return n
}

func (q *NFTQuery) process(
	ctx context.Context,
	c *TheGraphClient,
) (map[string][]*NFT, int64, error) {
	l := len(q.NFTContracts)
	validNFTs := map[string][]*NFT{}
	next := int64(-1)
	fmt.Println("starting processing contract batch")
	for _, contract := range q.NFTContracts {
		if err := ctx.Err(); err != nil {
			return nil, -1, err
		}

		// Regardless of if we should skip the contract, we need to update the next
		// timestamp marker we are pulling from.
		ca, err := strconv.ParseInt(contract.CreatedAt, 10, 64)
		if err != nil {
			continue
		}
		next = ca

		if len(contract.Name) == 0 {
			fmt.Println("skipping contract with empty name:", contract.ID)
			continue
		}

		shouldSkip, err := c.shouldSkip(contract.ID)
		if err != nil {
			fmt.Println("cannot determine if should skip", "err:", err)
		}
		if shouldSkip {
			fmt.Printf(
				"skipping previously inaccessible contract (name=%s address=%s)\n",
				contract.Name,
				contract.ID,
			)
			continue
		}

		fmt.Printf("processing contract (name=\"%s\" address=%s)\n", contract.Name, contract.ID)
		var (
			tokenURIs = map[string]struct{}{}
			repeats   = 0
			nfts      = []*NFT{}
		)
		for i, nft := range contract.NFTs {
			if err := ctx.Err(); err != nil {
				return nil, -1, err
			}

			// Periodically log NFT download progress
			if i%10 == 0 && i > 0 {
				fmt.Printf("processed %d/%d NFTs\n", i, len(contract.NFTs))
			}

			// Sanity check token fields
			metaURI := parseURI(nft.TokenURI)
			if len(metaURI) == 0 {
				fmt.Println("skipping because meta not ipfs: ", nft.TokenURI)
				break // skip entire contract
			}
			if strings.HasSuffix(metaURI, "unrevealed") {
				fmt.Println("skipping because unrevealed")
				// TODO: trigger retry (joepegs used to hide)
				break // skip entire contract
			}
			s, err := strconv.Atoi(nft.TokenID)
			if err != nil {
				// TODO: figure out why it would not work?
				break // skip entire contract
			}

			// Some early collections use the same TokenURI for all NFTs. This makes
			// things look quite boring.
			_, ok := tokenURIs[metaURI]
			if ok {
				repeats++
				continue
			}
			tokenURIs[metaURI] = struct{}{}

			// Fetch metadata
			addr, meta := fetchMetadata(c, metaURI)
			if meta == nil {
				break // skip contract
			}

			// Ensure image is valid
			imageURI := parseURI(meta.Image)
			if len(imageURI) == 0 {
				fmt.Println("skipping because image not ipfs: ", meta.Image)
				break // skip entire contract
			}

			// Skip projects that refer to things as a "pack"
			if strings.Contains(strings.ToLower(meta.Name), "pack") {
				break
			}
			if strings.Contains(strings.ToLower(meta.Description), "pack") {
				break
			}

			// Pin metadata
			//
			// We can't pass context here because the client library doesn't
			// support it, so we check context one last time before kicking this off.
			if err := ctx.Err(); err != nil {
				return nil, -1, err
			}
			if err := c.ic.Pin(addr); err != nil {
				fmt.Printf("unable to pin metadata %s: %v\n", addr, err)
				break // skip entire contract
			}

			// TODO: Pin images ASYNC
			// if err := c.ic.Pin(strings.TrimPrefix(imageURI, "ipfs://")); err != nil {
			// 	fmt.Printf("unable to pin image %s: %v\n", imageURI, err)
			// 	break // skip entire contract
			// }

			// Add NFT
			nfts = append(nfts, &NFT{
				Contract:  contract.ID,
				Symbol:    contract.Symbol,
				TokenID:   s,
				CreatedAt: ca,

				Name:        cleanString(meta.Name),
				Description: cleanString(meta.Description),
				Image:       imageURI,
			})
		}
		if len(nfts)+repeats < len(contract.NFTs) {
			if err := c.failed(contract.ID); err != nil {
				fmt.Println("unable to persist failed contract:", err)
			}
			fmt.Println(
				"failed to parse contract:",
				contract.ID,
				"nfts",
				len(nfts),
				"repeats",
				repeats,
				"expected",
				len(contract.NFTs),
			)
		} else {
			// TODO: broadcast NFTs here instead of waiting for batch to complete
			validNFTs[contract.ID] = nfts
			fmt.Printf("proccessing %s completed successfully (valid=%d repeat=%d)\n", contract.ID, len(nfts), repeats)
		}
	}
	fmt.Println("finished processing contract batch", "next:", next)
	if l < queryLimit {
		next = -1
	}
	return validNFTs, next, nil
}

func (c *TheGraphClient) Fetch(
	ctx context.Context,
	timestamp int64,
) (map[string][]*NFT, int64, error) {
	var q NFTQuery
	variables := map[string]interface{}{
		"timestamp": timestamp,
	}
	err := c.c.Query(ctx, &q, variables)
	if err != nil {
		return nil, -1, err
	}
	return q.process(ctx, c)
}
