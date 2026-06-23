#!/usr/bin/env bash
#
# Verify that every publishable crate in the workspace already exists on
# crates.io and is owned by the trusted Anza team.
#
# Adapted from ci/check-crates.sh in anza-xyz/agave.

set -euo pipefail

readonly VERIFIED_OWNER="${VERIFIED_OWNER:-anza-team}"
readonly USER_AGENT="Anza (https://github.com/anza-xyz/mollusk)"

crate_owners() {
	local crate="$1"
	curl -fsS -A "$USER_AGENT" "https://crates.io/api/v1/crates/${crate}/owners" \
		| jq -r '.users[].login'
}

main() {
	local crates
	mapfile -t crates < <(
		cargo metadata --no-deps --format-version 1 \
			| jq -r '.packages[] | select(.publish == null or (.publish | length > 0)) | .name'
	)

	if [[ ${#crates[@]} -eq 0 ]]; then
		echo "error: no publishable crates found in workspace" >&2
		exit 1
	fi

	local crate owners failed=()
	for crate in "${crates[@]}"; do
		if ! owners="$(crate_owners "$crate")"; then
			echo "  ✗ ${crate}: not found on crates.io (needs a devops bootstrap)" >&2
			failed+=("$crate")
		elif grep -qxF "$VERIFIED_OWNER" <<<"$owners"; then
			echo "  ✓ ${crate}"
		else
			echo "  ✗ ${crate}: not owned by ${VERIFIED_OWNER} (owners: ${owners//$'\n'/, })" >&2
			failed+=("$crate")
		fi
	done

	if [[ ${#failed[@]} -gt 0 ]]; then
		echo "error: ${#failed[@]} crate(s) failed ownership verification: ${failed[*]}" >&2
		exit 1
	fi

	echo "verified ${#crates[@]} publishable crates owned by ${VERIFIED_OWNER}"
}

main "$@"
