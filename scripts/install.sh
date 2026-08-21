#!/usr/bin/env bash
set -e

REPO="ahmadshady747-create/LOCUS"
OS="$(uname -s)"
ARCH="$(uname -m)"

case "$OS" in
  Linux)
    TARGET="x86_64-unknown-linux-gnu"
    BIN_NAME="locus"
    ;;
  Darwin)
    if [ "$ARCH" = "arm64" ]; then
      TARGET="aarch64-apple-darwin"
    else
      TARGET="x86_64-apple-darwin"
    fi
    BIN_NAME="locus"
    ;;
  *)
    echo "Unsupported OS: $OS"
    exit 1
    ;;
esac

echo "==> Fetching latest release of LOCUS Engine for $TARGET..."
DOWNLOAD_URL="https://github.com/$REPO/releases/latest/download/$BIN_NAME"
INSTALL_DIR="/usr/local/bin"

if [ ! -w "$INSTALL_DIR" ]; then
  INSTALL_DIR="$HOME/.local/bin"
  mkdir -p "$INSTALL_DIR"
fi

curl -fsSL "$DOWNLOAD_URL" -o "$INSTALL_DIR/locus"
chmod +x "$INSTALL_DIR/locus"

echo "==> LOCUS Engine successfully installed to $INSTALL_DIR/locus"
echo "==> Run 'locus --help' or 'locus mcp' to get started!"
