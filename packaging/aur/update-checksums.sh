#!/bin/bash
# update-checksums.sh — fill in real b2sums in the AUR PKGBUILDs.
#
# Run this AFTER pushing the release tag to GitHub, BEFORE publishing to AUR:
#   ./update-checksums.sh          # uses pkgver from each PKGBUILD
#
# Requires: curl, b2sum. Refuses to leave b2sums=('SKIP') behind.

set -euo pipefail
cd "$(dirname "$0")"

for pkg in blunux-ai-agent blunux-wa-bridge; do
    pkgbuild="$pkg/PKGBUILD"
    pkgver=$(grep -oP '^pkgver=\K.*' "$pkgbuild")
    url="https://github.com/nidoit/blunux2SB/archive/refs/tags/v${pkgver}.tar.gz"

    echo "── $pkg v$pkgver ──"
    tarball=$(mktemp)
    trap 'rm -f "$tarball"' EXIT

    if ! curl -sfL --max-time 300 -o "$tarball" "$url"; then
        echo "ERROR: cannot download $url" >&2
        echo "       Did you push the v$pkgver tag to GitHub?" >&2
        exit 1
    fi

    sum=$(b2sum "$tarball" | cut -d' ' -f1)
    sed -i "s/^b2sums=.*/b2sums=('$sum')/" "$pkgbuild"
    rm -f "$tarball"
    echo "   b2sums updated: ${sum:0:16}..."
done

echo
echo "Done. Verify with: git diff -- packaging/aur"
