#!/usr/bin/env bash
set -e

REPO="ahmadshady747-create/LOCUS"
OS="$(uname -s)"
ARCH="$(uname -m)"

case "$OS" in
    Linux*)     PLATFORM="x86_64-unknown-linux-gnu"; ARTIFACT="locus";;
    Darwin*)
        if [ "$ARCH" = "arm64" ]; then
            PLATFORM="aarch64-apple-darwin"; ARTIFACT="locus"
        else
            PLATFORM="x86_64-apple-darwin"; ARTIFACT="locus"
        fi
        ;;
    MINGW*|MSYS*|CYGWIN*) PLATFORM="x86_64-pc-windows-msvc"; ARTIFACT="locus.exe";;
    *)          echo "Unsupported OS: $OS"; exit 1;;
esac

URL="https://github.com/${REPO}/releases/latest/download/${ARTIFACT}"
INSTALL_DIR="/usr/local/bin"

echo "⚡ Downloading locus-engine (${PLATFORM})..."
if [ -w "$INSTALL_DIR" ]; then
    curl -fsSL "$URL" -o "${INSTALL_DIR}/locus"
    chmod +x "${INSTALL_DIR}/locus"
else
    sudo curl -fsSL "$URL" -o "${INSTALL_DIR}/locus"
    sudo chmod +x "${INSTALL_DIR}/locus"
fi

echo "✅ locus-engine successfully installed to ${INSTALL_DIR}/locus"
locus --help || true
