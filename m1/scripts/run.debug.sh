#!/bin/bash -e

if ! [[ "$0" =~ scripts/run.debug.sh ]]; then
  echo "must be run from the m1 directory"
  exit 255
fi

SUBNET_TIMEOUT=-1 ./scripts/tests.debug.sh 