#!/bin/bash
set -e

cp --reflink=auto --force --recursive $HOME/.cargo/ $HOME/.cargo_bkp/
