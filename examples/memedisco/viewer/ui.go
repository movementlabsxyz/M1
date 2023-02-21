// Copyright (C) 2023, Ava Labs, Inc. All rights reserved.
// See the file LICENSE for licensing terms.

package viewer

import (
	"context"
	"errors"
	"strconv"
	"time"

	"github.com/ava-labs/hypersdk/utils"
	"github.com/ava-labs/indexvm/examples/memedisco/reddit/models"
	"github.com/ava-labs/indexvm/examples/shared/gorse"
	"github.com/ava-labs/indexvm/examples/shared/ipfs"
	"github.com/briandowns/spinner"
	"github.com/inancgumus/screen"
	"github.com/manifoldco/promptui"
)

const memesPerPage = 5

func (v *Viewer) viewMeme(ctx context.Context, m *models.Meme, img []byte) bool {
	if err := ipfs.DisplayImage(img); err != nil {
		utils.Outf(
			"{{red}}can't view image:{{/}} %s %s %s\n",
			m.Title,
			m.Author,
			m.PostLink,
		)
		return false
	}
	utils.Outf("\n")
	utils.Outf("{{blue}}Title:{{/}} %s\n", m.Title)
	utils.Outf("{{blue}}Author:{{/}} %s\n", m.Author)
	utils.Outf("{{blue}}Sub:{{/}} %s\n", m.SubReddit)
	utils.Outf("{{blue}}Created:{{/}} %v\n", time.Unix(m.Created, 0).Local())
	utils.Outf("{{blue}}Post:{{/}} %s\n", m.PostLink)
	utils.Outf("\n")
	return true
}

func (v *Viewer) viewMemes(ctx context.Context) {
	next := 0
	for next < len(v.memes) {
		for i := next; i < len(v.memes) && i < next+memesPerPage; i++ {
			next = i
			utils.Outf("%d)\n", i)
			meme := v.memes[i]
			img, err := ipfs.FetchContent(ctx, v.ipfsAPI, meme.URL)
			if err != nil {
				utils.Outf(
					"{{red}}can't load image:{{/}} %s %s %s\n",
					meme.Title,
					meme.Author,
					meme.PostLink,
				)
				continue
			}
			v.viewMeme(ctx, meme, img)
		}
		next++

		// TODO: only show certain items if makes sense (only view filters/memes if
		// some saved)
		prompt := promptui.Select{
			Label: "Action",
			Items: []string{
				"Next Page",
				"Delete a meme",
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
		case "Delete an meme":
			validate := func(input string) error {
				if len(input) == 0 {
					return errors.New("input is empty")
				}
				selected, err := strconv.ParseInt(input, 10, 32)
				if err != nil {
					return errors.New("input is not a number")
				}
				if int(selected) >= len(v.memes) {
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
			memeToRemove := v.memes[selected]
			if err := RemoveMeme(ctx, v.db, memeToRemove.Name); err != nil {
				utils.Outf(
					"{{red}}can't remove meme:{{/}} %v\n",
					err,
				)
				return
			}
			v.memes = append(v.memes[:selected], v.memes[selected+1:]...)
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
				r = raw
				break
			case <-ctx.Done():
				s.Stop()
				return nil
			}
		}
		s.Stop()

		utils.Outf("\n")
		if !v.viewMeme(ctx, r.meme, r.img) {
			_ = v.submitRating(ctx, r.reply, gorse.Inaccessible)
			utils.Outf("{{yellow}}meme %s is inaccessible{{/}}", r.meme.Name)
			continue
		}

		// Wait for Action
		meme := r.meme
		for {
			prompt := promptui.Select{
				Label: "Action",
				Items: []string{
					"Rate",
					"Save",
					"View Saved memes",
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
					if err := ipfs.PinContent(ctx, v.ipfsAPI, meme.IPFS); err != nil {
						utils.Outf("{{red}}unable to pin image:{{/}} %s %v\n", meme.IPFS, err)
					}
				}

				// Send transaction to indexvm
				_ = v.submitRating(ctx, r.reply, gorse.FeedbackType(rating))
				cont = true
			case "Save":
				if err := ipfs.PinContent(ctx, v.ipfsAPI, meme.IPFS); err != nil {
					utils.Outf("{{red}}unable to pin image:{{/}} %s %v\n", meme.IPFS, err)
				}
				if err := StoreMeme(ctx, v.db, meme); err != nil {
					utils.Outf("{{red}}unable to save meme:{{/}} %+v %v\n", meme, err)
				}
				v.memes = append(v.memes, meme)
			case "View Saved memes":
				v.viewMemes(ctx)
				screen.Clear()
				screen.MoveTopLeft()
				v.viewMeme(ctx, r.meme, r.img)
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
						"{{red}}can't view image:{{/}} %s %s %s\n",
						meme.Title,
						meme.Author,
						meme.PostLink,
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
