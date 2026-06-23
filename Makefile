SHELL := /usr/bin/env bash
NIGHTLY_TOOLCHAIN := nightly-2025-10-07
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
	package \
	publish

# Crates to publish, in dependency order
PUBLISH_CRATES := \
	mollusk-svm-error \
	mollusk-svm-fuzz-fs \
	mollusk-svm-fuzz-fixture \
	mollusk-svm-fuzz-fixture-firedancer \
	mollusk-svm-result \
	mollusk-svm \
	mollusk-svm-bencher \
	mollusk-svm-programs-memo \
	mollusk-svm-programs-token-2022 \
	mollusk-svm-programs-token \
	mollusk-svm-cli

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

# Package crates into target/package/*.crate (so attestation can sign them
# before publish). Uses multi-package mode to resolve workspace inter-deps.
package:
	@cargo package $(addprefix -p ,$(PUBLISH_CRATES)) $(ARGS)

# Publish crates in dependency order
publish:
	@set -e && set -u && set -o pipefail && \
	for crate in $(PUBLISH_CRATES); do \
		echo "Publishing $$crate..." && \
		cargo publish -p $$crate --token $$TOKEN $(ARGS) && \
		echo "$$crate published successfully!" && \
		sleep 5; \
	done && \
	echo "All crates published successfully!"
