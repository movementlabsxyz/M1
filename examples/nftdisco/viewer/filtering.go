// Copyright (C) 2023, Ava Labs, Inc. All rights reserved.
// See the file LICENSE for licensing terms.

package viewer

import (
	"fmt"
	"strings"

	"github.com/ava-labs/indexvm/examples/nftdisco/thegraph"
	"github.com/ava-labs/indexvm/examples/shared/gorse"
)

type filter struct {
	Name        string `json:"name"`
	Contract    string `json:"contract"`
	Description string `json:"description"`
	CreatedAt   int64  `json:"createdAt"`

	Rating gorse.FeedbackType `json:"rating"`
}

func (v *Viewer) processRec(n *thegraph.NFT) gorse.FeedbackType {
	ln := strings.ToLower(n.Name)
	lc := strings.ToLower(n.Contract)
	ld := strings.ToLower(n.Description)

	v.filtersL.Lock()
	v.filtersL.Unlock()
	for _, f := range v.filters {
		if len(f.Name) > 0 {
			if strings.Contains(ln, f.Name) {
				v.autoRateLogs <- fmt.Sprintf(
					"{{yellow}}auto-rating %s with %s:{{/}} name (%s) matches %s\n",
					lc,
					f.Rating,
					ln,
					f.Name,
				)
				return f.Rating
			}
		} else if len(f.Contract) > 0 {
			if strings.Contains(lc, f.Contract) {
				v.autoRateLogs <- fmt.Sprintf("{{yellow}}auto-rating %s with %s:{{/}} contract match\n", lc, f.Rating)
				return f.Rating
			}
		} else if len(f.Description) > 0 {
			if strings.Contains(ld, f.Description) {
				v.autoRateLogs <- fmt.Sprintf("{{yellow}}auto-rating %s with %s:{{/}} description (%s) matches %s\n", lc, f.Rating, ld, f.Description)
				return f.Rating
			}
		} else {
			panic("invalid filter")
		}
	}
	return ""
}
