SHELL := /usr/bin/env bash
NIGHTLY_TOOLCHAIN := nightly-2026-04-11
SOLANA_VERSION := 4.0.0


.PHONY: \
	nightly-version \
	solana-version \
	format \
	format-test-programs \
	format-fix \
	format-fix-test-programs \
	clippy \
	clippy-test-programs \
	clippy-fix \
	clippy-fix-test-programs \
	build \
	build-test-programs \
	build-test-elfs \
	test \
	audit \
	check-features \
	prepublish \
	verify-crate-owners \
	package \
	publish

# Advisories to ignore for audit.
# - RUSTSEC-2022-0093: ed25519-dalek: Double Public Key Signing Function Oracle Attack
# - RUSTSEC-2024-0344: curve25519-dalek
# - RUSTSEC-2024-0376: Remotely exploitable Denial of Service in Tonic
# - RUSTSEC-2024-0421: idna accepts Punycode labels that do not produce any non-ASCII when decoded
# - RUSTSEC-2025-0009: Some AES functions may panic when overflow checking is enabled
IGNORE_SECS := \
	RUSTSEC-2022-0093 \
	RUSTSEC-2024-0344 \
	RUSTSEC-2024-0376 \
	RUSTSEC-2024-0421 \
	RUSTSEC-2025-0009

# Print the nightly toolchain version for CI
nightly-version:
	@echo $(NIGHTLY_TOOLCHAIN)

# Print the Solana version for CI
solana-version:
	@echo $(SOLANA_VERSION)

format:
	@cargo +$(NIGHTLY_TOOLCHAIN) fmt --all -- --check

format-test-programs:
	@cargo +$(NIGHTLY_TOOLCHAIN) fmt --manifest-path test-programs/Cargo.toml --all -- --check

format-fix:
	@cargo +$(NIGHTLY_TOOLCHAIN) fmt --all

format-fix-test-programs:
	@cargo +$(NIGHTLY_TOOLCHAIN) fmt --manifest-path test-programs/Cargo.toml --all

clippy:
	@cargo +$(NIGHTLY_TOOLCHAIN) clippy --all --all-features --all-targets -- -D warnings

clippy-test-programs:
	@cargo +$(NIGHTLY_TOOLCHAIN) clippy --manifest-path test-programs/Cargo.toml --all --all-features --all-targets -- -D warnings

clippy-fix:
	@cargo +$(NIGHTLY_TOOLCHAIN) clippy --all --all-features --all-targets --fix --allow-dirty --allow-staged -- -D warnings

clippy-fix-test-programs:
	@cargo +$(NIGHTLY_TOOLCHAIN) clippy --manifest-path test-programs/Cargo.toml --all --all-features --all-targets --fix --allow-dirty --allow-staged -- -D warnings

build:
	@cargo build

build-test-programs:
	@cargo build-sbf --manifest-path test-programs/Cargo.toml --sbf-out-dir target/deploy

build-test-elfs:
	@set -e; \
	OUT_DIR=target/deploy; \
	TMP_DIR=target/tmp; \
	mkdir -p $$OUT_DIR $$TMP_DIR; \
	for ARCH in v0 v1 v2 v3; do \
		echo "Building test_program_cpi_target ($$ARCH)..."; \
		cargo build-sbf --manifest-path test-programs/cpi-target/Cargo.toml --arch $$ARCH --sbf-out-dir $$TMP_DIR; \
		mv $$TMP_DIR/test_program_cpi_target.so $$OUT_DIR/test_program_cpi_target_$${ARCH}.so; \
	done; \
	rm -rf $$TMP_DIR

test:
	@$(MAKE) build-test-programs
	@$(MAKE) build-test-elfs
	@cargo test --manifest-path test-programs/Cargo.toml --all-features
	@cargo test --all-features

audit:
	@cargo audit $(addprefix --ignore ,$(IGNORE_SECS))

check-features:
	@cargo hack --feature-powerset --no-dev-deps check

prepublish:
	@$(MAKE) format
	@$(MAKE) clippy
	@$(MAKE) build
	@$(MAKE) check-features
	@$(MAKE) test

verify-crate-owners:
	@./scripts/verify-crate-owners.sh

# Package all workspace crates into target/package/*.crate (so attestation can
# sign them before publish).
package:
	@cargo package --workspace $(ARGS)

DRY_RUN ?=
publish:
	@cargo publish --workspace $(DRY_RUN) $(ARGS)
