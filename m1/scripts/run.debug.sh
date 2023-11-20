#!/bin/bash -e

if ! [[ "$0" =~ scripts/run.debug.sh ]]; then
  echo "must be run from the m1 directory"
  exit 255
fi

SUBNET_TIMEOUT=300000 ./scripts/tests.debug.sh 