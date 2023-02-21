// Copyright (C) 2023, Ava Labs, Inc. All rights reserved.
// See the file LICENSE for licensing terms.

package ipfs

import (
	"context"
	"fmt"
	"io"
	"os"
	"path/filepath"

	icore "github.com/ipfs/interface-go-ipfs-core"
	"github.com/ipfs/interface-go-ipfs-core/options"

	"github.com/ipfs/kubo/config"
	"github.com/ipfs/kubo/core"
	"github.com/ipfs/kubo/core/coreapi"
	libp2p "github.com/ipfs/kubo/core/node/libp2p"
	"github.com/ipfs/kubo/core/node/libp2p/fd"
	"github.com/ipfs/kubo/plugin/loader" // This package is needed so that all the preloaded plugins are loaded automatically
	"github.com/ipfs/kubo/repo/fsrepo"
)

func setupPlugins(externalPluginsPath string) (*loader.PluginLoader, error) {
	// Load any external plugins if available on externalPluginsPath
	plugins, err := loader.NewPluginLoader(filepath.Join(externalPluginsPath, "plugins"))
	if err != nil {
		return nil, fmt.Errorf("error loading plugins: %s", err)
	}
	// Load preloaded and external plugins
	if err := plugins.Initialize(); err != nil {
		return nil, fmt.Errorf("error initializing plugins: %s", err)
	}
	if err := plugins.Inject(); err != nil {
		return nil, fmt.Errorf("error initializing plugins: %s", err)
	}
	return plugins, nil
}

// Creates an IPFS node and returns its coreAPI
func createNode(ctx context.Context, repoPath string) (*core.IpfsNode, error) {
	// Open the repo
	repo, err := fsrepo.Open(repoPath)
	if err != nil {
		return nil, err
	}

	cfg, err := repo.Config()
	if err != nil {
		return nil, err
	}

	// Construct the node
	nodeOptions := &core.BuildCfg{
		Online:    true,
		Permanent: true,
		// Routing: libp2p.DHTOption, // This option sets the node to be a full DHT node (both fetching and storing DHT Records)
		// Routing: libp2p.DHTClientOption, // This option sets the node to be a client DHT node (only fetching records)
		Routing: libp2p.ConstructDefaultRouting(
			cfg.Identity.PeerID,
			cfg.Addresses.Swarm,
			cfg.Identity.PrivKey,
		),
		Repo: repo,
	}

	return core.NewNode(ctx, nodeOptions)
}

func preparePath(path string) error {
	_, err := os.Stat(path)
	if os.IsNotExist(err) {
		if err := os.Mkdir(path, 0o775); err != nil {
			return err
		}
	}
	if fsrepo.IsInitialized(path) {
		return nil
	}
	identity, err := config.CreateIdentity(io.Discard, []options.KeyGenerateOption{
		options.Key.Type(options.Ed25519Key),
	})
	if err != nil {
		return err
	}
	conf, err := config.InitWithIdentity(identity)
	if err != nil {
		return err
	}
	// Move off non-default ports to avoid any conflict with other services
	conf.Addresses.API = []string{"/ip4/127.0.0.1/tcp/5002"}
	conf.Addresses.Gateway = []string{"/ip4/127.0.0.1/tcp/9080"}
	conf.Addresses.Swarm = []string{
		"/ip4/0.0.0.0/tcp/4002",
		"/ip6/::/tcp/4002",
		"/ip4/0.0.0.0/udp/4002/quic",
		"/ip6/::/udp/4002/quic",
	}
	// Set reasonable memory limits
	conf.Swarm.ResourceMgr.Enabled = config.True
	conf.Swarm.ResourceMgr.MaxMemory.WithDefault("4GB")
	conf.Swarm.ResourceMgr.MaxFileDescriptors.WithDefault(int64(fd.GetNumFDs()) / 2)
	if fsrepo.Init(path, conf); err != nil {
		return nil
	}
	return nil
}

func New(ctx context.Context, path string) (icore.CoreAPI, *core.IpfsNode, error) {
	plugins, err := setupPlugins("")
	if err != nil {
		return nil, nil, err
	}
	if err := preparePath(path); err != nil {
		return nil, nil, err
	}
	node, err := createNode(ctx, path)
	if err != nil {
		return nil, nil, err
	}
	if err := plugins.Start(node); err != nil {
		return nil, nil, err
	}
	api, err := coreapi.NewCoreAPI(node)
	return api, node, err
}
