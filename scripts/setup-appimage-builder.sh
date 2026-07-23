#!/usr/bin/env bash
# Set up an ubuntu-22.04 Toolbx container matching the CI runner so that
# `pnpm build:linux` (deb + AppImage) can be reproduced locally.
#
# Why a container: the AppImage bundler shells out to `patchelf` and copies
# `/usr/bin/xdg-open` into the image, both of which the Fedora host lacks or
# the minimal Toolbx image omits. ubuntu-22.04 is the exact CI runner image.
#
# The host's node/pnpm/cargo are reused inside the container (their glibc
# requirements, GLIBC_2.17/2.28, are well below Ubuntu 22.04's glibc 2.35),
# sharing ~/.cargo as a build cache.
#
# Usage:
#   scripts/setup-appimage-builder.sh          # create + provision (idempotent)
#
# Then build with:
#   toolbox enter tauri-ci-22.04
#   cd /run/host/more_storage/repos/stfc-mod-new-launcher
#   export CI=true CARGO_TARGET_DIR="$PWD/src-tauri/target-ubuntu22"
#   pnpm build:linux
set -euo pipefail

CONTAINER="tauri-ci-22.04"
DISTRO="ubuntu"
RELEASE="22.04"

# CI dependency list (from .github/workflows/release.yml) plus the tools the
# AppImage bundler itself needs (patchelf, xdg-utils) and basic build tooling.
# Defined inline in the install step below (single source of truth).

echo "==> Checking for Toolbx container '${CONTAINER}'..."
if toolbox list --containers | awk '{print $2}' | grep -qx "$CONTAINER"; then
	echo "    already exists; skipping create"
else
	echo "==> Creating Toolbx container (${DISTRO}:${RELEASE})..."
	toolbox create --assumeyes --distro "$DISTRO" --release "$RELEASE" "$CONTAINER"
fi

# toolbox run inherits the host CWD, which isn't visible inside the container
# when the repo lives outside $HOME (e.g. /more_storage). Run from $HOME to
# avoid the harmless "directory not found in container" chdir noise.
run_in_container() {
	( cd ~ && toolbox run -c "$CONTAINER" bash -lc "$1" )
}

echo "==> Installing build dependencies inside '${CONTAINER}'..."
run_in_container '
	set -e
	sudo apt-get update -qq
	# Idempotent: apt-get install is a no-op for already-installed packages.
	sudo DEBIAN_FRONTEND=noninteractive apt-get install -y --no-install-recommends \
		libwebkit2gtk-4.1-dev libappindicator3-dev librsvg2-dev patchelf libfuse2 \
		xdg-utils build-essential pkg-config curl file ca-certificates >/dev/null
	# The host ~/.bashrc sources /etc/profile.d/lang.sh (a Fedora locale script
	# absent on Ubuntu); an empty stub keeps the shared shell rc portable and
	# silences the spurious "No such file" warning under toolbox run.
	sudo touch /etc/profile.d/lang.sh
'

echo "==> Verifying host toolchain is usable inside the container..."
run_in_container '
	echo -n "    node:    "; node --version
	echo -n "    pnpm:    "; pnpm --version
	echo -n "    cargo:   "; cargo --version
	# patchelf + xdg-open are the two the AppImage bundler strictly needs;
	# Tauri fetches its own linuxdeploy/squashfs into ~/.cache/tauri.
	for t in patchelf xdg-open pkg-config; do
		printf "    %-8s " "$t"; command -v "$t" || echo "MISSING"
	done
	echo -n "    repo:    "
	ls -d /run/host/more_storage/repos/stfc-mod-new-launcher >/dev/null 2>&1 \
		&& echo "visible at /run/host/more_storage/repos/stfc-mod-new-launcher" \
		|| echo "NOT FOUND"
'

echo
echo "==> Done. To build the AppImage:"
echo "    toolbox enter ${CONTAINER}"
echo "    cd /run/host/more_storage/repos/stfc-mod-new-launcher"
echo "    export CI=true CARGO_TARGET_DIR=\"\$PWD/src-tauri/target-ubuntu22\""
echo "    pnpm build:linux"
