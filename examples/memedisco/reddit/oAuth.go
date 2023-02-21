// Copyright (C) 2023, Ava Labs, Inc. All rights reserved.
// See the file LICENSE for licensing terms.

package reddit

import (
	"encoding/base64"
	"encoding/json"
	"io/ioutil"
	"net/http"
	"strings"

	"github.com/ava-labs/indexvm/examples/memedisco/reddit/models"
)

func (c *Client) refreshAccessToken() error {
	// Reddit URL to get access token
	url := "https://www.reddit.com/api/v1/access_token"

	// Set grant type to client_credentials as POST body
	payload := strings.NewReader("grant_type=client_credentials")
	req, err := http.NewRequest("POST", url, payload)
	if err != nil {
		return err
	}

	// Set Headers including the User Agent and the Authorization with the encoded credentials
	req.Header.Add("User-Agent", c.UserAgent)
	req.Header.Add("Authorization", "Basic "+c.encodeCredentials())
	req.Header.Add("Accept", "*/*")
	req.Header.Add("Cache-Control", "no-cache")
	req.Header.Add("Host", "www.reddit.com")
	req.Header.Add("Content-Type", "application/x-www-form-urlencoded")
	req.Header.Add("Accept-Encoding", "gzip, deflate")
	req.Header.Add("Content-Length", "29")
	req.Header.Add("Connection", "keep-alive")
	req.Header.Add("cache-control", "no-cache")

	// Make Request
	res, err := http.DefaultClient.Do(req)
	if err != nil {
		return err
	}

	// Close the response body
	defer res.Body.Close()

	// Read the response
	body, err := ioutil.ReadAll(res.Body)
	if err != nil {
		return err
	}
	var accessTokenBody models.AccessTokenBody
	if err := json.Unmarshal(body, &accessTokenBody); err != nil {
		return err
	}
	c.AccessToken = accessTokenBody.AccessToken
	return nil
}

func (c *Client) encodeCredentials() (encodedCredentials string) {
	data := c.ClientID + ":" + c.ClientSecret
	encodedCredentials = base64.StdEncoding.EncodeToString([]byte(data))
	return
}
