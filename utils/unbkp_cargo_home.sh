#!/bin/bash
set -e
if [[ -d "$HOME/.cargo" && "$HOME/.cargo_bkp" ]]; then
	rm -rf $HOME/.cargo/
	cp -rf --reflink=auto $HOME/.cargo_bkp $HOME/.cargo
fi
