// Copyright (C) 2023, Ava Labs, Inc. All rights reserved.
// See the file LICENSE for licensing terms.

package servicer

import "errors"

var (
	ErrMissingHeartbeat = errors.New("missing heartbeat")
	ErrInvalidRating    = errors.New("invalid rating")
)
