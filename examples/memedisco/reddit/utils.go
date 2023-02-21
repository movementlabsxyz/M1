// Copyright (C) 2023, Ava Labs, Inc. All rights reserved.
// See the file LICENSE for licensing terms.

package reddit

import (
	"fmt"
	"io"
	"net/http"
)

func (c *Client) authenticatedGet(url string) (io.ReadCloser, int, error) {
	req, _ := http.NewRequest("GET", url, nil)
	req.Header.Add("Authorization", "Bearer "+c.AccessToken)
	req.Header.Add("User-Agent", c.UserAgent)
	req.Header.Add("Accept", "*/*")
	req.Header.Add("Cache-Control", "no-cache")
	req.Header.Add("Host", "oauth.reddit.com")
	req.Header.Add("Connection", "keep-alive")
	req.Header.Add("cache-control", "no-cache")
	res, err := http.DefaultClient.Do(req)
	if err != nil {
		return nil, res.StatusCode, err
	}
	return res.Body, res.StatusCode, nil
}

// GetSubredditAPIURL : Returns API Reddit URL with Limit
func subredditAPIURL(subreddit string, filter string, after string, limit int) string {
	return fmt.Sprintf(
		"https://oauth.reddit.com/r/%s/%s?limit=%d&&after=%s",
		subreddit,
		filter,
		limit,
		after,
	)
}
