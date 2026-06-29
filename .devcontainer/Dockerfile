# Ubuntu 22.04 dev image aligned with .github/workflows/release.yml (build-linux job).
# Intended for Linux bundle builds only (deb/AppImage); not for macOS or Windows targets.
FROM mcr.microsoft.com/devcontainers/base:ubuntu-22.04

LABEL org.opencontainers.image.description="Linux-only build environment for STFC Mod Launcher (deb/AppImage)"
LABEL devcontainer.platform="linux"

ARG DEBIAN_FRONTEND=noninteractive

RUN apt-get update \
	&& apt-get install -y --no-install-recommends \
		libwebkit2gtk-4.1-dev \
		libappindicator3-dev \
		librsvg2-dev \
		patchelf \
		libfuse2 \
	&& apt-get clean -y \
	&& rm -rf /var/lib/apt/lists/*
