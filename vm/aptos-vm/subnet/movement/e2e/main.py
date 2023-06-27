#!/usr/bin/env python3

# Copyright © Aptos Foundation
# SPDX-License-Identifier: Apache-2.0

"""
This script is how we orchestrate running a local testnet and then running CLI tests against it. There are two different CLIs used for this:

1. Base: For running the local testnet. This is what the --base-network flag and all other flags starting with --base are for.
2. Test: The CLI that we're testing. This is what the --test-cli-tag / --test-cli-path and all other flags starting with --test are for.

Example (testing CLI in image):
  python3 main.py --base-network testnet --test-cli-tag mainnet_0431e2251d0b42920d89a52c63439f7b9eda6ac3

Example (testing locally built CLI binary):
  python3 main.py --base-network devnet --test-cli-path ~/aptos-core/target/release/aptos

This means, run the CLI test suite using a CLI built from mainnet_0431e2251d0b42920d89a52c63439f7b9eda6ac3 against a local testnet built from the testnet branch of aptos-core.

Example (using a different image repo):
  See ~/.github/workflows/cli-e2e-tests.yaml

When the test suite is complete, it will tell you which tests passed and which failed. To further debug a failed test, you can check the output in --working-directory, there will be files for each test containing the command run, stdout, stderr, and any exception.
"""

import argparse
import logging
import pathlib
import shutil
import sys

from cases.account import test_account_create, test_account_fund_with_faucet
from cases.init import test_init
from common import Network
from local_testnet import run_node, stop_node, wait_for_startup
from test_helpers import RunHelper
from test_results import test_results

logging.basicConfig(
    stream=sys.stderr,
    format="%(asctime)s - %(levelname)s - %(message)s",
    level=logging.INFO,
)

LOG = logging.getLogger(__name__)


def parse_args():
    # You'll notice there are two argument "prefixes", base and test. These refer to
    # cases 1 and 2 in the top-level comment.
    parser = argparse.ArgumentParser(
        formatter_class=argparse.RawDescriptionHelpFormatter,
        description=__doc__,
    )
    parser.add_argument("-d", "--debug", action="store_true")
    parser.add_argument(
        "--image-repo-with-project",
        default="aptoslabs",
        help=(
            "What docker image repo (+ project) to use for the local testnet. "
            "By default we use Docker Hub: %(default)s (so, just aptoslabs for the "
            "project since Docker Hub is the implied default repo). If you want to "
            "specify a different repo, it might look like this: "
            "docker.pkg.github.com/aptoslabs/aptos-core"
        ),
    )
    parser.add_argument(
        "--base-network",
        required=True,
        type=Network,
        choices=list(Network),
        help="What branch the Movement CLI used for the local testnet should be built from",
    )
    parser.add_argument(
        "--base-startup-timeout",
        type=int,
        default=30,
        help="Timeout in seconds for waiting for node and faucet to start up",
    )
    test_cli_args = parser.add_mutually_exclusive_group(required=True)
    test_cli_args.add_argument(
        "--test-cli-tag",
        help="The image tag for the CLI we want to test, e.g. mainnet_0431e2251d0b42920d89a52c63439f7b9eda6ac3",
    )
    test_cli_args.add_argument(
        "--test-cli-path",
        help="Path to CLI binary we want to test, e.g. /home/dport/aptos-core/target/release/aptos",
    )
    parser.add_argument(
        "--working-directory",
        default="/tmp/aptos-cli-tests",
        help="Where we'll run CLI commands from (in the host system). Default: %(default)s",
    )
    args = parser.parse_args()
    return args


def run_tests(run_helper):
    # Run init tests. We run these first to set up the CLI.
    test_init(run_helper)

    # Run account tests.
    test_account_fund_with_faucet(run_helper)
    test_account_create(run_helper)


def main():
    args = parse_args()

    if args.debug:
        logging.getLogger().setLevel(logging.DEBUG)
        LOG.debug("Debug logging enabled")
    else:
        logging.getLogger().setLevel(logging.INFO)

    # Run a node + faucet and wait for them to start up.
    container_name = run_node(args.base_network, args.image_repo_with_project)
    wait_for_startup(container_name, args.base_startup_timeout)

    # Create the dir the test CLI will run from.
    shutil.rmtree(args.working_directory, ignore_errors=True)
    pathlib.Path(args.working_directory).mkdir(parents=True, exist_ok=True)

    # Build the RunHelper object.
    run_helper = RunHelper(
        host_working_directory=args.working_directory,
        image_repo_with_project=args.image_repo_with_project,
        image_tag=args.test_cli_tag,
        cli_path=args.test_cli_path,
    )

    # Prepare the run helper. This ensures in advance that everything needed is there.
    run_helper.prepare()

    # Run tests.
    run_tests(run_helper)

    # Stop the node + faucet.
    stop_node(container_name)

    # Print out the results.
    if test_results.passed:
        LOG.info("These tests passed:")
        for test_name in test_results.passed:
            LOG.info(test_name)

    if test_results.failed:
        LOG.error("These tests failed:")
        for test_name, exception in test_results.failed:
            LOG.error(f"{test_name}: {exception}")
        return False

    LOG.info("All tests passed!")
    return True


if __name__ == "__main__":
    if main():
        sys.exit(0)
    else:
        sys.exit(1)
