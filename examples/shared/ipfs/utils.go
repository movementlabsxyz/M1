// Copyright (C) 2023, Ava Labs, Inc. All rights reserved.
// See the file LICENSE for licensing terms.

package ipfs

import (
	"context"
	"encoding/base64"
	"fmt"
	"io/ioutil"
	"net/url"
	"strings"

	files "github.com/ipfs/go-ipfs-files"
	icore "github.com/ipfs/interface-go-ipfs-core"
	ipath "github.com/ipfs/interface-go-ipfs-core/path"
)

func FetchContent(ctx context.Context, ipfsAPI icore.CoreAPI, uri string) ([]byte, error) {
	contentAddr := strings.TrimPrefix(uri, "ipfs://")
	cleanContentAddr, err := url.QueryUnescape(
		contentAddr,
	) // sometimes NFTs encode IPFS as URL-escaped
	if err != nil {
		return nil, err
	}
	p := ipath.New(cleanContentAddr)
	closer, err := ipfsAPI.Unixfs().Get(ctx, p)
	if err != nil {
		return nil, err
	}
	defer closer.Close()

	// Parse file
	var file files.File
	switch f := closer.(type) {
	case files.File:
		file = f
	case files.Directory:
		return nil, icore.ErrIsDir
	default:
		return nil, icore.ErrNotSupported
	}

	return ioutil.ReadAll(file)
}

func PinContent(ctx context.Context, ipfsAPI icore.CoreAPI, uri string) error {
	contentAddr := strings.TrimPrefix(uri, "ipfs://")
	cleanContentAddr, err := url.QueryUnescape(
		contentAddr,
	) // sometimes NFTs encode IPFS as URL-escaped
	if err != nil {
		return err
	}
	p := ipath.New(cleanContentAddr)
	return ipfsAPI.Pin().Add(ctx, p)
}

// Inspired by: https://github.com/olivere/iterm2-imagetools/blob/main/cmd/imgcat/imgcat.go
func DisplayImage(b []byte) error {
	fmt.Print("\033]1337;")
	fmt.Printf("File=inline=1")
	fmt.Printf(";height=30")
	fmt.Print("preserveAspectRatio=1")
	fmt.Print(":")
	fmt.Printf("%s", base64.StdEncoding.EncodeToString(b))
	fmt.Print("\a\n")
	return nil
}
