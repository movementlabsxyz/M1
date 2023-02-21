# examples

This package contains a number of example tools that interact with the `indexvm`. If you
create your own example tool, open a PR!

## Tools
### Avalanche NFT Searcher
What better place to start indexing the decentralized web than with NFTs? The first programatic `Searcher`
we implemented scrapes [TheGraph](https://thegraph.com/hosted-service/subgraph/traderjoe-xyz/nft-contracts) for all
Avalanche NFTs and uploads their metadata directly to the `indexvm` (this saves
one hop for any clients that would otherwise need to query IPFS).

To reduce the latency of fetching any Avalanche NFT metadata or image, the
`Searcher` also pins any IPFS content it indexes to a locally running IPFS
node.

### Meme (Reddit) Searcher
The second programatic `Searcher` we implemented scrapes the most popular meme
subreddits, uploads meme metadata to the `indexvm`, and pins the meme to
a locally running IPFS node.

### Generic Recommendation Server
Now that we can put all this content on-chain, what now? We next implemented
a recommendation engine using [Gorse](https://github.com/gorse-io/gorse). This
recommendation engine can serve suggestions for any user for any on-chain
`Schema` (NFTs, Memes, etc.). This engine is also configured as a proper
`Servicer` and will only serve a limited number of recommendations before
requiring users to pay a commission in their ratings (this is optional).

#### Ingestor
This recommendation engine relies on a generic ingestor that can parse data
on-chain and persist it into Gorse. You can add your own `Schema` parsing to
this mechanism if you write your own `Searcher`.

#### Replayer
When running a local network, you may spend days uploading content to the
`indexvm` and then decide to modify the functionality of the chain (which
may require resetting the chain). The replayer persists all uploaded content
in a separate database and can then replay it back onto a new chain.

### CLI-Based Content Viewer + Voter
Last but not least, we built a CLI that can view and rate NFTs and memes. This
CLI uses IPFS as a library (no external node required) and pins any content
that the user rates as "good". Additionally, the user can auto-rate certain
types of content (like all items in an NFT collection) and can save content
that they can look at later.

### Spammer
For throughput testing, we also included a spam tool that seeks to submit as
many index transactions as possible to a running network. This can be run
concurrently with legitimate uploads to stress test the system in different
scenarios.

## E2E NFT Demo
This E2E demo will walk you through:
1) Creating your own local `indexvm` Subnet
2) Indexing Avalanche NFTs
3) Recommending Avalanche NFTs
4) Viewing Avalanche NFTs Recommendations
5) Rating Avalanche NFTs

Running this demo requires opening a number of terminal windows and assumes
basic familiarity with `bash` and `docker`. Please follow each step closely
as missing any step may cause the demo to not work properly. In the future,
we hope to codify this process in a [Docker Compose
file](https://docs.docker.com/compose/compose-file/compose-file-v3/).

### Step 1: Launch Your Subnet
The first step to running this demo is to launch your own `indexvm` Subnet. You
can do so by running the following commands from this location (will move you to
the base `indexvm` folder first):
```bash
cd ..;
./scripts/run.sh;
```

By default, this allocates all funds on the network to
`index1rvzhmceq997zntgvravfagsks6w0ryud3rylh4cdvayry0dl97nsqrawg5`. The private
key for this address is
`0x323b1d8f4eed5f0da9da93071b034f2dce9d2d22692c172f3cb252a64ddfafd01b057de320297c29ad0c1f589ea216869cf1938d88c9fbd70d6748323dbf2fa7`.
For convenience, this key has is also stored at `examples/nftdisco/demo.pk`.

When the Subnet is up and running (may take a few minutes), it will log the
following:
```txt
cluster is ready!
endpoint: /ext/bc/2QqEkALmvXzNhYXXuTGV9STzeMZ98CfZWXqtFxetJ4AeCiSThq
logsDir: /var/folders/w0/dkzpklzn03s8x37n36n397f80000gn/T/network-runner-root-data_20230220_095435
pid: 62967
uris:
- http://127.0.0.1:9650
- http://127.0.0.1:9652
- http://127.0.0.1:9654
- http://127.0.0.1:9656
- http://127.0.0.1:9658
avalanche-network-runner is running in the background...

use the following command to terminate:

killall avalanche-network-runner
```

### Step 2: Start Recommendation Engine
In another teminal window, you should now bring up the recommendation engine
(Gorse) infrastructure. You can do so by running the following commands from
this location:
```bash
cd shared/docker;
docker compose up;
```

You can view information about the recommendation engine by visiting
http://localhost:8088/overview. You can view information about the IPFS
node running on your computer by visiting http://localhost:5001/webui.
If you cannot reach this page, then you did not spin up the
infrastructure correctly.

### Step 3: Start Ingestor
In another terminal window, you should now start the ingestor. This tool will
listen to activity on-chain and persist it in the recommendation engine. You
can start this by running the following commands from this location:
```bash
cd shared;
go run cmd/shared/main.go ingest --commission=0.000001 --indexvm-endpoint=http://localhost:9650/ext/bc/2QqEkALmvXzNhYXXuTGV9STzeMZ98CfZWXqtFxetJ4AeCiSThq --gorse-endpoint=http://localhost:8088 --servicer=index1rvzhmceq997zntgvravfagsks6w0ryud3rylh4cdvayry0dl97nsqrawg5;
```

The ingestor will log the following message when it is running properly:
```txt
listening for new transactions...
```

### Step 4: Start Indexing Avalanche NFTs
In another terminal window, you can now start the Avalanche NFT searcher. This
tool will scrape TheGraph for all Avalanche NFTs and upload them to the
`indexvm`. The ingestor you just started will record these in Gorse. You can
start this by running the following commands from this location (starts
ingestion from December 1st, 2022):
```bash
cd nftdisco;
go run cmd/nftdisco/main.go searcher --start-timestamp=1669881600 --indexvm-endpoint=http://localhost:9650/ext/bc/2QqEkALmvXzNhYXXuTGV9STzeMZ98CfZWXqtFxetJ4AeCiSThq --ipfs-endpoint=http://localhost:5001 --gorse-endpoint=http://localhost:8088 --private-key-path=demo.pk;
```

_If you re-run the above command after ingesting some content and want to
continue where you left off, remove the `start-timestamp` flag._

The searcher will log the following message when it is running properly:
```txt
loaded searcher: index1rvzhmceq997zntgvravfagsks6w0ryud3rylh4cdvayry0dl97nsqrawg5
starting to fetch NFTs from timestamp: 1669881600
```

The searcher will automatically skip any NFTs where fetching their metadata
takes longer than 3 minutes. The searcher will also skip any NFTs that it was
not previously able to index (instead of waiting for the full timeout).

_Because we use a local IPFS node to query all NFT information, you can make
the searcher much faster by using an external IPFS service. We did not include
this modification in this demo because we did not feel it was in the spirit of
the `indexvm` (which is all about evolving away from centralized infrastructure
providers)._

### Step 5: Start the Servicer
In another terminal window, you can now start the servicer. This tool will
recommend Avalanche NFTs to any user willing to pay a commission to us. You can
specify both the `commission` required for each recommendation and the number
of `pending` ratings to allow before we begin re-serving previous
recommendations. In production, it may also be wise to add some sort of
ReCaptcha or to require a deposit before serving recommendations to avoid DoS.
You can start the servicer by running the following commands from this location:
```bash
cd shared;
go run cmd/shared/main.go servicer --commission=0.000001 --pending=30 --indexvm-endpoint=http://localhost:9650/ext/bc/2QqEkALmvXzNhYXXuTGV9STzeMZ98CfZWXqtFxetJ4AeCiSThq --gorse-endpoint=http://localhost:8088 --servicer=index1rvzhmceq997zntgvravfagsks6w0ryud3rylh4cdvayry0dl97nsqrawg5
```

The servicer will log the following message when it is running properly:
```txt
listening for recommendation requests on port 10000
```

### Step 6: View NFTs
In another terminal window, you can now start viewing uploaded NFTs. We use
`iTerm2's` [inline images protocol](https://iterm2.com/documentation-images.html) to
show NFTs right in the terminal. You can rate NFTs, save NFTs, and even
auto-rate NFTs based on some preference. The recommender system will learn what
you like and start to provide better recommendations to other users (which you
can use just be creating a new PK and transfering funds). You can start the viewer
by running the following commands from this location:
```bash
cd nftdisco;
go run cmd/nftdisco/main.go viewer --indexvm-endpoint=http://localhost:9650/ext/bc/2QqEkALmvXzNhYXXuTGV9STzeMZ98CfZWXqtFxetJ4AeCiSThq --servicer-endpoint=http://localhost:10000/rpc --private-key-path=demo.pk;
```

The viewer will log the following message when it is running properly (it runs
an embedded IPFS node):
```txt
loaded identity: index1rvzhmceq997zntgvravfagsks6w0ryud3rylh4cdvayry0dl97nsqrawg5
loaded balance: 999.999998
started IPFS node: 12D3KooWRAoXed4PUeF1qyUeBrVjFbXKDTfKpyuzk8nerFwdFEgM

local addrs: [/ip6/::1/udp/4002/quic /ip4/10.0.0.170/tcp/4002 /ip4/127.0.0.1/tcp/4002 /ip6/::1/tcp/4002 /ip4/10.0.0.170/udp/4002/quic /ip4/127.0.0.1/udp/4002/quic]
```

**YOU MUST WAIT FOR SOME NFTs TO BE INDEXED BEFORE YOU CAN BE SERVED
RECOMMENDATIONS.** If you have not indexed enough NFTs yet, you will see the
following error message:
```txt
Exited: failed to decode client response: no recommendations  http://localhost:10000/rpc
```

_You may notice that we are using the same private key across multiple
services, yet all transactions are being processed successfully. This is
because `hypersdk` transactions do not have nonces and we don't need to
synchronize ordering amongst these different tools._
