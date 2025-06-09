NIGHTLY_TOOLCHAIN := nightly-2024-11-22
SOLANA_VERSION := 2.2.0

.PHONY: audit build-test-programs prepublish publish format format-check clippy test check-features nightly-version solana-version

# Print the nightly toolchain version for CI
nightly-version:
	@echo $(NIGHTLY_TOOLCHAIN)

# Print the Solana version for CI
solana-version:
	@echo $(SOLANA_VERSION)

# Security audit with ignored advisories
audit:
	@cargo audit \
		--ignore RUSTSEC-2022-0093 \
		--ignore RUSTSEC-2024-0344 \
		--ignore RUSTSEC-2024-0421 \
		--ignore RUSTSEC-2024-0376 \
		--ignore RUSTSEC-2025-0009
# RUSTSEC-2022-0093: ed25519-dalek: Double Public Key Signing Function Oracle Attack
# RUSTSEC-2024-0344: curve25519-dalek
# RUSTSEC-2024-0421: idna accepts Punycode labels that do not produce any non-ASCII when decoded
# RUSTSEC-2024-0376: Remotely exploitable Denial of Service in Tonic
# RUSTSEC-2025-0009: Some AES functions may panic when overflow checking is enabled

# Build test programs
build-test-programs:
	@cargo build-sbf --manifest-path test-programs/cpi-target/Cargo.toml
	@cargo build-sbf --manifest-path test-programs/primary/Cargo.toml

# Pre-publish checks
prepublish:
	@agave-install init $(SOLANA_VERSION)
	@rm -rf target
	@cargo build
	@$(MAKE) build-test-programs
	@$(MAKE) format-check
	@$(MAKE) clippy
	@$(MAKE) check-features
	@$(MAKE) test

# Publish crates in order
publish:
	@set -e && set -u && set -o pipefail && \
	CRATES=( \
		"mollusk-svm-error" \
		"mollusk-svm-keys" \
		"mollusk-svm-fuzz-fs" \
		"mollusk-svm-fuzz-fixture" \
		"mollusk-svm-fuzz-fixture-firedancer" \
		"mollusk-svm-result" \
		"mollusk-svm" \
		"mollusk-svm-bencher" \
		"mollusk-svm-programs-memo" \
		"mollusk-svm-programs-token" \
		"mollusk-svm-cli" \
	) && \
	for crate in "$${CRATES[@]}"; do \
		echo "Publishing $$crate..." && \
		cargo publish -p $$crate --token $$TOKEN $(ARGS) && \
		echo "$$crate published successfully!" && \
		sleep 5; \
	done && \
	echo "All crates published successfully!"

# Format code
format:
	@cargo +$(NIGHTLY_TOOLCHAIN) fmt --all

# Check formatting
format-check:
	@cargo +$(NIGHTLY_TOOLCHAIN) fmt --all -- --check

# Run clippy linter
clippy:
	@cargo +$(NIGHTLY_TOOLCHAIN) clippy --all --all-features --all-targets -- -D warnings

# Check all feature combinations with cargo-hack
check-features:
	@cargo hack check --feature-powerset --no-dev-deps

# Run tests
test:
	@cargo test --all-features