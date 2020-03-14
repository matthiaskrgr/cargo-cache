#!/bin/bash
set -e

cp --reflink=auto --force --recursive /home/matthias/.cargo/ /home/matthias/.cargo_bkp/
