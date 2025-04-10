#!/bin/bash

# Advisories to ignore.
advisories=(
  # ed25519-dalek: Double Public Key Signing Function Oracle Attack
  #
  # Remove once repo upgrades to ed25519-dalek v2
  "RUSTSEC-2022-0093"

  # curve25519-dalek
  #
  # Remove once repo upgrades to curve25519-dalek v4
  "RUSTSEC-2024-0344"

  # Crate:     tonic
  # Version:   0.9.2
  # Title:     Remotely exploitable Denial of Service in Tonic
  # Date:      2024-10-01
  # ID:        RUSTSEC-2024-0376
  # URL:       https:#rustsec.org/advisories/RUSTSEC-2024-0376
  # Solution:  Upgrade to >=0.12.3
  "RUSTSEC-2024-0376"

  # Crate:     ring
  # Version:   0.16.20
  # Title:     Some AES functions may panic when overflow checking is enabled.
  # Date:      2025-03-06
  # ID:        RUSTSEC-2025-0009
  # URL:       https:#rustsec.org/advisories/RUSTSEC-2025-0009
  # Solution:  Upgrade to >=0.17.12
  # Dependency tree:
  # ring 0.16.20
  #
  # Crate:     ring
  # Version:   0.17.3
  # Title:     Some AES functions may panic when overflow checking is enabled.
  # Date:      2025-03-06
  # ID:        RUSTSEC-2025-0009
  # URL:       https:#rustsec.org/advisories/RUSTSEC-2025-0009
  # Solution:  Upgrade to >=0.17.12
  # Dependency tree:
  # ring 0.17.3
  "RUSTSEC-2025-0009"
)

ignore_flags=()
for advisory in "${advisories[@]}"; do
  ignore_flags+=("--ignore" "$advisory")
done

cargo audit "${ignore_flags[@]}"