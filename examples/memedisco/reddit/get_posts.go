// Copyright (C) 2023, Ava Labs, Inc. All rights reserved.
// See the file LICENSE for licensing terms.

package reddit

import (
	"encoding/json"
	"io/ioutil"

	ipfs "github.com/ipfs/go-ipfs-api"

	"github.com/ava-labs/indexvm/examples/memedisco/reddit/models"
)

// GetNPosts : Get (N) no. of posts from Reddit with Subreddit Name and Limit
func (c *Client) GetNPosts(
	subreddit string,
	filter string,
	after string,
	count int,
) ([]*models.Meme, string, error) {
	url := subredditAPIURL(subreddit, filter, after, count)
	body, _, err := c.authenticatedGet(url)
	if body != nil {
		defer body.Close()
	}
	if err != nil {
		return nil, "", err
	}
	contents, err := ioutil.ReadAll(body)
	if err != nil {
		return nil, "", err
	}

	// Parse response
	var redditResponse models.Response
	if err := json.Unmarshal(contents, &redditResponse); err != nil {
		return nil, "", err
	}

	memes := []*models.Meme{}
	for _, post := range redditResponse.Data.Children {
		url := post.Data.URL
		if url[len(url)-4:] != ".jpg" && url[len(url)-4:] != ".png" && url[len(url)-4:] != ".gif" {
			continue
		}
		// TODO: skip if not original or not enough likes
		memes = append(memes, &models.Meme{
			Name:      post.Data.Name,
			Title:     post.Data.Title,
			PostLink:  post.Data.GetShortLink(),
			SubReddit: post.Data.Subreddit,
			URL:       post.Data.URL,
			Author:    post.Data.Author,
			NSFW:      post.Data.Over18,
			Spoiler:   post.Data.Spoiler,
			Created:   int64(post.Data.CreatedUtc),
		})
	}
	return memes, redditResponse.Data.After, nil
}

func (c *Client) PinMeme(s *ipfs.Shell, meme *models.Meme) (string, error) {
	img, _, err := c.authenticatedGet(meme.URL)
	if err != nil {
		return "", err
	}
	defer img.Close()
	return s.Add(img)
}
