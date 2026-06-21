#!/bin/bash

solana program dump TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb ./src/elf/token_2022.so -u mainnet-beta
solana slot -u mainnet-beta | xargs -I {} sed -i '' 's|//! Last updated at mainnet-beta slot height: .*|//! Last updated at mainnet-beta slot height: {}|' ./src/lib.rs
