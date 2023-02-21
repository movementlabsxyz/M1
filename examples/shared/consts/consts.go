// Copyright (C) 2023, Ava Labs, Inc. All rights reserved.
// See the file LICENSE for licensing terms.

package consts

import "github.com/ava-labs/hypersdk/utils"

const (
	NFTDataSchema  = "nft-data-v0.0.1"
	MemeDataSchema = "meme-data-v0.0.1"
	SpamDataSchema = "spam-data-v0.0.1" // can ignore
	RatingSchema   = "rating-v0.0.1"    // generic
)

var (
	NFTDataSchemaID  = utils.ToID([]byte(NFTDataSchema))
	MemeDataSchemaID = utils.ToID([]byte(MemeDataSchema))
	SpamDataSchemaID = utils.ToID([]byte(SpamDataSchema))
	RatingSchemaID   = utils.ToID([]byte(RatingSchema))
)
