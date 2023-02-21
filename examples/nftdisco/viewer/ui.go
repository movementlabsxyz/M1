// Copyright (C) 2023, Ava Labs, Inc. All rights reserved.
// See the file LICENSE for licensing terms.

package viewer

import (
	"context"
	"errors"
	"fmt"
	"strconv"
	"strings"
	"time"

	"github.com/ava-labs/hypersdk/utils"
	"github.com/ava-labs/indexvm/examples/nftdisco/thegraph"
	"github.com/ava-labs/indexvm/examples/shared/gorse"
	"github.com/ava-labs/indexvm/examples/shared/ipfs"
	"github.com/briandowns/spinner"
	"github.com/inancgumus/screen"
	"github.com/manifoldco/promptui"
	"github.com/skratchdot/open-golang/open"
)

const (
	filtersPerPage = 20
	nftsPerPage    = 5
)

func (v *Viewer) viewNFT(ctx context.Context, nft *thegraph.NFT, img []byte) bool {
	if err := ipfs.DisplayImage(img); err != nil {
		utils.Outf(
			"{{red}}can't view image:{{/}} %s %d %s\n",
			nft.Contract,
			nft.TokenID,
			nft.Image,
		)
		return false
	}
	utils.Outf("\n")
	utils.Outf("{{blue}}Name:{{/}} %s\n", nft.Name)
	utils.Outf("{{blue}}Description:{{/}} %s\n", nft.Description)
	utils.Outf("{{blue}}Symbol:{{/}} %s\n", nft.Symbol)
	utils.Outf("{{blue}}Created At:{{/}} %v\n", time.Unix(int64(nft.CreatedAt), 0).Local())
	utils.Outf("{{blue}}Contract:{{/}} %s\n", nft.Contract)
	utils.Outf("{{blue}}TokenID:{{/}} %d\n", nft.TokenID)
	utils.Outf("{{green}}issued:{{/}} %d\n", v.issued)
	utils.Outf("\n")
	return true
}

func (v *Viewer) viewFilters(ctx context.Context) {
	next := 0
	for next < len(v.filters) {
		for i := next; i < len(v.filters) && i < next+filtersPerPage; i++ {
			next = i
			utils.Outf("%d)\n", i)
			item := v.filters[i]
			utils.Outf("\n")
			if len(item.Name) > 0 {
				utils.Outf("{{blue}}Name:{{/}} %s\n", item.Name)
			}
			if len(item.Contract) > 0 {
				utils.Outf("{{blue}}Contract:{{/}} %s\n", item.Contract)
			}
			if len(item.Description) > 0 {
				utils.Outf("{{blue}}Description:{{/}} %s\n", item.Description)
			}
			utils.Outf("{{blue}}Created At:{{/}} %v\n", time.Unix(0, item.CreatedAt).Local())
			utils.Outf("\n")
		}
		next++

		prompt := promptui.Select{
			Label: "Action",
			Items: []string{
				"Next Page",
				"Delete a Filter",
				"Exit",
			},
			Size: 15,
		}
		_, result, err := prompt.Run()
		if err != nil {
			return
		}
		switch result {
		case "Next Page":
		case "Delete a Filter":
			validate := func(input string) error {
				if len(input) == 0 {
					return errors.New("input is empty")
				}
				selected, err := strconv.ParseInt(input, 10, 32)
				if err != nil {
					return errors.New("input is not a number")
				}
				if int(selected) >= len(v.filters) {
					return errors.New("input is out of range")
				}
				return nil
			}
			promptText := promptui.Prompt{
				Label:    "Choice",
				Validate: validate,
			}
			choice, err := promptText.Run()
			if err != nil {
				return
			}
			selected, err := strconv.ParseInt(choice, 10, 32)
			if err != nil {
				return
			}
			filterToRemove := v.filters[selected]
			if err := RemoveFilter(ctx, v.db, filterToRemove.CreatedAt); err != nil {
				utils.Outf(
					"{{red}}can't remove filter:{{/}} %v\n",
					err,
				)
				return
			}
			v.filters = append(v.filters[:selected], v.filters[selected+1:]...)
			return
		case "Exit":
			return
		}
	}
}

func (v *Viewer) viewNFTs(ctx context.Context) {
	next := 0
	for next < len(v.nfts) {
		for i := next; i < len(v.nfts) && i < next+nftsPerPage; i++ {
			next = i
			utils.Outf("%d)\n", i)
			nft := v.nfts[i]
			img, err := ipfs.FetchContent(ctx, v.ipfsAPI, nft.Image)
			if err != nil {
				utils.Outf(
					"{{red}}can't load image:{{/}} %s %d %s\n",
					nft.Contract,
					nft.TokenID,
					nft.Image,
				)
				continue
			}
			v.viewNFT(ctx, nft, img)
		}
		next++

		// TODO: only show certain items if makes sense (only view filters/nfts if
		// some saved)
		prompt := promptui.Select{
			Label: "Action",
			Items: []string{
				"Next Page",
				"Delete an NFT",
				"Exit",
			},
			Size: 15,
		}
		_, result, err := prompt.Run()
		if err != nil {
			return
		}
		switch result {
		case "Next Page":
		case "Delete an NFT":
			validate := func(input string) error {
				if len(input) == 0 {
					return errors.New("input is empty")
				}
				selected, err := strconv.ParseInt(input, 10, 32)
				if err != nil {
					return errors.New("input is not a number")
				}
				if int(selected) >= len(v.nfts) {
					return errors.New("input is out of range")
				}
				return nil
			}
			promptText := promptui.Prompt{
				Label:    "Choice",
				Validate: validate,
			}
			choice, err := promptText.Run()
			if err != nil {
				return
			}
			selected, err := strconv.ParseInt(choice, 10, 32)
			if err != nil {
				return
			}
			nftToRemove := v.nfts[selected]
			if err := RemoveNFT(ctx, v.db, nftToRemove.ID()); err != nil {
				utils.Outf(
					"{{red}}can't remove NFT:{{/}} %v\n",
					err,
				)
				return
			}
			v.nfts = append(v.nfts[:selected], v.nfts[selected+1:]...)
			return
		case "Exit":
			return
		}
	}
}

func (v *Viewer) runUI(ctx context.Context) error {
	var started bool
	for {
		if started {
			screen.Clear()
			screen.MoveTopLeft()
		}
		started = true

		// Download next recommendation
		s := spinner.New(spinner.CharSets[21], 50*time.Millisecond)
		s.Prefix = "loading next recommendation "
		s.Color("bold")
		s.Start()
		var r *rec
		for r == nil {
			select {
			case raw := <-v.recs:
				act := v.processRec(raw.nft) // can add filter after processed but before viewed
				if len(act) > 0 {
					_ = v.submitRating(ctx, raw.reply, act)
					continue
				}
				r = raw
				break
			case l := <-v.autoRateLogs:
				utils.Outf(l)
			case <-ctx.Done():
				s.Stop()
				return nil
			}
		}
		s.Stop()

		utils.Outf("\n")
		if !v.viewNFT(ctx, r.nft, r.img) {
			_ = v.submitRating(ctx, r.reply, gorse.Inaccessible)
			utils.Outf("{{yellow}}nft %s is inaccessible{{/}}", r.nft.ID)
			continue
		}

		// Wait for Action
		nft := r.nft
		for {
			prompt := promptui.Select{
				Label: "Action",
				Items: []string{
					"Rate",
					"View on Marketplace",
					"Save",
					"View Saved NFTs",
					"Add Filter",
					"View Filters",
					"View Balance",
					"View IPFS Peers",
					"Reload Image",
					"Exit",
				},
				Size: 15,
			}
			_, result, err := prompt.Run()
			if err != nil {
				return err
			}

			var cont bool
			switch result {
			case "Rate":
				// Collect feedback
				prompt := promptui.Select{
					Label: "Rating",
					Items: gorse.FeedbackTypes,
					Size:  10,
				}
				_, rating, err := prompt.Run()
				if err != nil {
					return err
				}

				// Pin good content for benefit of network
				if rating == string(gorse.Like) {
					if err := ipfs.PinContent(ctx, v.ipfsAPI, nft.Image); err != nil {
						utils.Outf("{{red}}unable to pin image:{{/}} %s %v\n", nft.Image, err)
					}
				}

				// Send transaction to indexvm
				_ = v.submitRating(ctx, r.reply, gorse.FeedbackType(rating))
				cont = true
			case "Save":
				if err := ipfs.PinContent(ctx, v.ipfsAPI, nft.Image); err != nil {
					utils.Outf("{{red}}unable to pin image:{{/}} %s %v\n", nft.Image, err)
				}
				if err := StoreNFT(ctx, v.db, nft); err != nil {
					utils.Outf("{{red}}unable to save nft:{{/}} %+v %v\n", nft, err)
				}
				v.nfts = append(v.nfts, nft)
			case "View Saved NFTs":
				v.viewNFTs(ctx)
				screen.Clear()
				screen.MoveTopLeft()
				v.viewNFT(ctx, r.nft, r.img)
			case "View on Marketplace":
				prompt := promptui.Select{
					Label: "Marketplace",
					Items: []string{"Joepegs", "OpenSea", "Kalao", "Campfire"},
				}
				_, marketplace, err := prompt.Run()
				if err != nil {
					return err
				}
				var link string
				switch marketplace {
				case "Joepegs":
					link = fmt.Sprintf(
						"https://joepegs.com/item/%s/%d",
						nft.Contract,
						nft.TokenID,
					)
				case "OpenSea":
					link = fmt.Sprintf(
						"https://opensea.io/assets/avalanche/%s/%d",
						nft.Contract,
						nft.TokenID,
					)
				case "Kalao":
					link = fmt.Sprintf(
						"https://marketplace.kalao.io/nft/%s_%d",
						nft.Contract,
						nft.TokenID,
					)
				case "Campfire":
					link = fmt.Sprintf(
						"https://campfire.exchange/collections/%s/%d",
						nft.Contract,
						nft.TokenID,
					)
				}
				if err := open.Run(link); err != nil {
					return err
				}
			case "Add Filter":
				promptFilter := promptui.Select{
					Label: "Filter",
					Items: []string{"Contract", "Name", "Description"},
				}
				_, filterType, err := promptFilter.Run()
				if err != nil {
					return err
				}

				var f filter
				if filterType == "Name" || filterType == "Description" {
					validate := func(input string) error {
						if len(input) == 0 {
							return errors.New("input is empty")
						}
						return nil
					}
					promptText := promptui.Prompt{
						Label:    "Contains",
						Validate: validate,
					}
					contains, err := promptText.Run()
					if err != nil {
						return err
					}
					if filterType == "Name" {
						f.Name = strings.ToLower(contains)
					} else {
						f.Description = strings.ToLower(contains)
					}
				} else {
					f.Contract = strings.ToLower(nft.Contract)
				}

				promptRating := promptui.Select{
					Label: "Automatic Rating",
					Items: gorse.FeedbackTypes,
					Size:  10,
				}
				_, rating, err := promptRating.Run()
				if err != nil {
					return err
				}
				f.Rating = gorse.FeedbackType(rating)
				f.CreatedAt = time.Now().UnixNano()
				if err := StoreFilter(ctx, v.db, &f); err != nil {
					return err
				}
				v.filtersL.Lock()
				v.filters = append(v.filters, &f)
				v.filtersL.Unlock()

				// Process this item
				act := v.processRec(r.nft)
				if len(act) > 0 {
					_ = v.submitRating(ctx, r.reply, act)
					cont = true
				}
			case "View Filters":
				v.viewFilters(ctx)
				screen.Clear()
				screen.MoveTopLeft()
				v.viewNFT(ctx, r.nft, r.img)
			case "View Balance":
				unlocked, locked, _, err := v.icli.Balance(ctx, v.addr)
				if err != nil {
					return err
				}
				utils.Outf(
					"{{yellow}}address:{{/}} %s {{yellow}}unlocked:{{/}} %d {{yellow}}locked:{{/}} %d\n",
					v.addr,
					unlocked,
					locked,
				)
			case "View IPFS Peers":
				peers, err := v.ipfsAPI.Swarm().Peers(ctx)
				if err != nil {
					return err
				}
				utils.Outf("{{yellow}}IPFS peers:{{/}} %d\n", len(peers))
			case "Reload Image":
				if err := ipfs.DisplayImage(r.img); err != nil {
					utils.Outf(
						"{{red}}can't view image:{{/}} %s %d %s\n",
						nft.Contract,
						nft.TokenID,
						nft.Image,
					)
					continue
				}
			case "Exit":
				return context.Canceled
			}
			if cont {
				break
			}
		}
	}
}
