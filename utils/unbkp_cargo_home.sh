#!/bin/bash
set -e
if [[ -d "/home/matthias/.cargo" && "/home/matthias/.cargo_bkp" ]]; then
	rm -rf /home/matthias/.cargo/
	cp -rf --reflink=auto /home/matthias/.cargo_bkp /home/matthias/.cargo
fi
