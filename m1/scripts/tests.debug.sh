#!/bin/bash -e

if ! [[ "$0" =~ scripts/tests.debug.sh ]]; then
  echo "must be run from the m1 directory"
  exit 255
fi

./scripts/build.debug.sh \
&& VM_PLUGIN_PATH=$(pwd)/target/debug/subnet \
./scripts/tests.e2e.sh