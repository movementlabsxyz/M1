// Copyright (C) 2023, Ava Labs, Inc. All rights reserved.
// See the file LICENSE for licensing terms.

package reddit

import (
	"errors"
	"net/http"
	"os"
	"time"
)

type Client struct {
	AccessToken  string
	ClientID     string
	ClientSecret string
	UserAgent    string

	hc http.Client
}

func New(userAgent string) (*Client, error) {
	// Get Reddit Client Credentials from the environment variables
	clientID := os.Getenv("REDDIT_CLIENT_ID")
	clientSecret := os.Getenv("REDDIT_CLIENT_SECRET")
	if clientID == "" || clientSecret == "" {
		return nil, errors.New("REDDIT_CLIENT_ID and/or REDDIT_CLIENT_SECRET have not been set")
	}

	// Create reusable http client
	t := http.DefaultTransport.(*http.Transport).Clone()
	t.MaxIdleConns = 100_000
	t.MaxConnsPerHost = 100_000
	c := &Client{
		ClientID:     clientID,
		ClientSecret: clientSecret,
		UserAgent:    userAgent,
		hc: http.Client{
			Transport: t,
			Timeout:   30 * time.Second,
		},
	}
	return c, c.refreshAccessToken()
}
