#!/bin/bash

solana program dump TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA ./src/elf/token.so -u mainnet-beta
solana program dump ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL ./src/elf/associated_token.so -u mainnet-beta
solana slot -u mainnet-beta | xargs -I {} sed -i '' 's|//! Last updated at mainnet-beta slot height: .*|//! Last updated at mainnet-beta slot height: {}|' ./src/lib.rs
