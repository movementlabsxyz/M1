// Copyright (C) 2023, Ava Labs, Inc. All rights reserved.
// See the file LICENSE for licensing terms.

package models

import (
	"github.com/ava-labs/hypersdk/codec"
	"github.com/ava-labs/indexvm/actions"
)

// Meme : Basic structure of a Meme Response
type Meme struct {
	Name      string `json:"name"`
	PostLink  string `json:"postLink"`
	SubReddit string `json:"subreddit"`
	Title     string `json:"title"`
	URL       string `json:"url"`
	Author    string `json:"author"`
	NSFW      bool   `json:"nsfw"`
	Spoiler   bool   `json:"spoiler"`
	Created   int64  `json:"created"`
	IPFS      string `json:"ipfs"`
}

func (m *Meme) Pack() ([]byte, error) {
	p := codec.NewWriter(actions.MaxContentSize)
	p.PackString(m.Name)
	p.PackString(m.PostLink)
	p.PackString(m.SubReddit)
	p.PackString(m.Title)
	p.PackString(m.URL)
	p.PackString(m.Author)
	p.PackBool(m.NSFW)
	p.PackBool(m.Spoiler)
	p.PackInt64(m.Created)
	p.PackString(m.IPFS)
	return p.Bytes(), p.Err()
}

func Unpack(b []byte) (*Meme, error) {
	var m Meme
	p := codec.NewReader(b, actions.MaxContentSize)
	m.Name = p.UnpackString(true)
	m.PostLink = p.UnpackString(true)
	m.SubReddit = p.UnpackString(true)
	m.Title = p.UnpackString(true)
	m.URL = p.UnpackString(true)
	m.Author = p.UnpackString(true)
	m.NSFW = p.UnpackBool()
	m.Spoiler = p.UnpackBool()
	m.Created = p.UnpackInt64(true)
	m.IPFS = p.UnpackString(true)
	return &m, p.Err()
}
