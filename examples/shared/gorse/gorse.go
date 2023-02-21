// Copyright (C) 2023, Ava Labs, Inc. All rights reserved.
// See the file LICENSE for licensing terms.

package gorse

// Inspired by: https://github.com/gorse-io/gorse/blob/master/client/client.go

import (
	"context"
	"encoding/binary"
	"encoding/json"
	"errors"
	"fmt"
	"io"
	"net/http"
	"strings"
	"time"
)

type FeedbackType string

const (
	Like         FeedbackType = "like"
	Dislike      FeedbackType = "dislike"
	Skip         FeedbackType = "skip"
	Junk         FeedbackType = "junk"
	Inaccessible FeedbackType = "inaccessible"
	Abuse        FeedbackType = "abuse"
)

var FeedbackTypes = []FeedbackType{Like, Dislike, Skip, Junk, Inaccessible, Abuse}

func (f FeedbackType) Value() []byte {
	switch f {
	case Like:
		return binary.BigEndian.AppendUint16(nil, 0)
	case Dislike:
		return binary.BigEndian.AppendUint16(nil, 1)
	case Skip:
		return binary.BigEndian.AppendUint16(nil, 2)
	case Junk:
		return binary.BigEndian.AppendUint16(nil, 3)
	case Inaccessible:
		return binary.BigEndian.AppendUint16(nil, 4)
	case Abuse:
		return binary.BigEndian.AppendUint16(nil, 5)
	default:
		panic("invalid feedback type")
	}
}

func ParseFeedback(f []byte) (FeedbackType, error) {
	if len(f) != 2 {
		return "", errors.New("invalid length")
	}
	val := binary.BigEndian.Uint16(f)
	switch val {
	case 0:
		return Like, nil
	case 1:
		return Dislike, nil
	case 2:
		return Skip, nil
	case 3:
		return Junk, nil
	case 4:
		return Inaccessible, nil
	case 5:
		return Abuse, nil
	}
	return "", errors.New("invalid feedback type")
}

type Client struct {
	url string
	key string
	c   *http.Client
}

type Feedback struct {
	FeedbackType string `json:"FeedbackType"`
	UserId       string `json:"UserId"`
	ItemId       string `json:"ItemId"`
	Timestamp    string `json:"Timestamp"`
}

type ErrorMessage string

func (e ErrorMessage) Error() string {
	return string(e)
}

type RowAffected struct {
	RowAffected int `json:"RowAffected"`
}

type User struct {
	UserId    string   `json:"UserId"`
	Labels    []string `json:"Labels"`
	Subscribe []string `json:"Subscribe"`
	Comment   string   `json:"Comment"`
}

type Item struct {
	ItemId     string   `json:"ItemId"`
	IsHidden   bool     `json:"IsHidden"`
	Labels     []string `json:"Labels"`
	Categories []string `json:"Categories"`
	Timestamp  string   `json:"Timestamp"`
	Comment    string   `json:"Comment"`
}

type LatestItem struct {
	Id    string `json:"Id"`
	Score int64  `json:"Score"` // timestamp
}

func New(url string, key string, timeout time.Duration) *Client {
	return &Client{
		url: url,
		key: key,
		c: &http.Client{
			Timeout: timeout,
		},
	}
}

func request[Response any, Body any](
	ctx context.Context,
	c *Client,
	method, url string,
	body Body,
) (result Response, err error) {
	bodyByte, marshalErr := json.Marshal(body)
	if marshalErr != nil {
		return result, marshalErr
	}
	var req *http.Request
	req, err = http.NewRequestWithContext(ctx, method, url, strings.NewReader(string(bodyByte)))
	if err != nil {
		return result, err
	}
	req.Header.Set("X-API-Key", c.key)
	req.Header.Set("Content-Type", "application/json")
	resp, err := c.c.Do(req)
	if err != nil {
		return result, err
	}
	defer resp.Body.Close()
	buf := new(strings.Builder)
	_, err = io.Copy(buf, resp.Body)
	if err != nil {
		return result, err
	}
	if resp.StatusCode != http.StatusOK {
		return result, ErrorMessage(buf.String())
	}
	err = json.Unmarshal([]byte(buf.String()), &result)
	if err != nil {
		return result, err
	}
	return result, err
}

func (c *Client) Unrated(ctx context.Context, user string) ([]*Feedback, error) {
	return request[[]*Feedback, any](
		ctx,
		c,
		"GET",
		c.url+fmt.Sprintf("/api/user/"+user+"/feedback/seen"),
		nil,
	)
}

func (c *Client) DeleteSeen(ctx context.Context, user string, item string) error {
	_, err := request[RowAffected, any](
		ctx,
		c,
		"DELETE",
		c.url+fmt.Sprintf("/api/feedback/seen/%s/%s", user, item),
		nil,
	)
	return err
}

func (c *Client) Recommend(
	ctx context.Context,
	user string,
	schema string,
) (string, string, error) {
	recommendations, err := request[[]string, any](
		ctx,
		c,
		"GET",
		c.url+fmt.Sprintf("/api/recommend/%s/%s?n=1&write-back-type=seen", user, schema),
		nil,
	)
	if err != nil {
		return "", "", err
	}
	if len(recommendations) != 1 {
		return "", "", errors.New("no recommendations")
	}
	itemId := recommendations[0]
	content, err := c.Get(ctx, recommendations[0])
	return itemId, content, err
}

func (c *Client) Get(ctx context.Context, item string) (string, error) {
	i, err := request[Item, any](ctx, c, "GET", c.url+fmt.Sprintf("/api/item/%s", item), nil)
	if err != nil {
		return "", err
	}
	return i.Comment, nil
}

func (c *Client) Rate(ctx context.Context, user string, item string, feedback FeedbackType) error {
	match := false
	for _, allowed := range FeedbackTypes {
		if allowed == feedback {
			match = true
			break
		}
	}
	if !match {
		return fmt.Errorf("%s is not a valid type of feedback", feedback)
	}
	_, err := request[RowAffected](ctx, c, "POST", c.url+"/api/feedback", []Feedback{
		{
			FeedbackType: string(feedback),
			UserId:       user,
			ItemId:       item,
			Timestamp:    time.Now().UTC().Format(time.RFC3339),
		},
	})
	return err
}

func (c *Client) Has(ctx context.Context, item string) (bool, error) {
	_, err := request[Item, any](ctx, c, "GET", c.url+fmt.Sprintf("/api/item/%s", item), nil)
	if err == nil {
		return true, nil
	}
	if strings.Contains(err.Error(), "item not found") {
		return false, nil
	}
	return false, err
}

// WARNING: Overwrites item if exists
func (c *Client) Insert(
	ctx context.Context,
	item string,
	comment string,
	schema string,
	searcher string,
) error {
	_, err := request[RowAffected](ctx, c, "POST", c.url+"/api/item", Item{
		ItemId:     item,
		Comment:    comment,
		Timestamp:  time.Now().UTC().Format(time.RFC3339),
		Categories: []string{schema},
		Labels:     []string{fmt.Sprintf("searcher:%s", searcher)},
	})
	return err
}

func (c *Client) Latest(ctx context.Context, schema string) (string, int64, string, error) {
	items, err := request[[]*LatestItem, any](
		ctx,
		c,
		"GET",
		c.url+fmt.Sprintf("/api/latest/%s?n=1", schema),
		nil,
	)
	if err != nil {
		return "", -1, "", err
	}
	if len(items) == 0 {
		return "", 0, "", nil
	}
	item := items[0]
	content, err := c.Get(ctx, item.Id)
	if err != nil {
		return "", -1, "", err
	}
	return items[0].Id, items[0].Score, content, nil
}
