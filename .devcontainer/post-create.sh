#!/usr/bin/env bash
set -euo pipefail

echo "Dev container: Linux build only (pnpm build:linux). Use native hosts or CI for macOS/Windows."

corepack enable
corepack install
pnpm install --frozen-lockfile
