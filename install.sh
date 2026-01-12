#!/bin/bash

set -e

# EnvHub Install Script
# Inspired by bun, rustup, etc.

GITHUB_REPO="sontallive/envhub"
INSTALL_DIR="$HOME/.envhub"
BIN_DIR="$INSTALL_DIR/bin"
CONFIG_DIR="$HOME/.config/envhub"

# Colors
reset="\033[0m"
cyan="\033[36m"
green="\033[32m"
red="\033[31m"
dim="\033[2m"

echo -e "${cyan}Checking system...${reset}"

# Detect OS and Architecture
OS=$(uname -s | tr '[:upper:]' '[:lower:]')
ARCH=$(uname -m)

case "$OS" in
  darwin)
    case "$ARCH" in
      x86_64) TARGET="x86_64-apple-darwin" ;;
      arm64) TARGET="aarch64-apple-darwin" ;;
      *) echo -e "${red}Unsupported architecture: $ARCH${reset}"; exit 1 ;;
    esac
    ;;
  linux)
    case "$ARCH" in
      x86_64) TARGET="x86_64-unknown-linux-gnu" ;;
      *) echo -e "${red}Unsupported architecture: $ARCH${reset}"; exit 1 ;;
    esac
    ;;
  *)
    echo -e "${red}Unsupported OS: $OS${reset}"
    echo "Windows users should download the binary from GitHub Releases manually."
    exit 1
    ;;
esac

echo -e "Target: ${dim}$TARGET${reset}"

# Create directories
mkdir -p "$BIN_DIR"
mkdir -p "$CONFIG_DIR"

# Get latest release tag
echo -e "${cyan}Fetching latest version...${reset}"
LATEST_TAG=$(curl -s "https://api.github.com/repos/$GITHUB_REPO/releases/latest" | grep '"tag_name":' | sed -E 's/.*"([^"]+)".*/\1/')

if [ -z "$LATEST_TAG" ]; then
    echo -e "${red}Failed to fetch latest release. Please check your connection or repo name.${reset}"
    exit 1
fi

echo -e "Version: ${green}$LATEST_TAG${reset}"

# Download and unpack
URL="https://github.com/$GITHUB_REPO/releases/download/$LATEST_TAG/envhub-$TARGET.tar.gz"
echo -e "${cyan}Downloading...${reset}"
curl -L "$URL" -o "/tmp/envhub.tar.gz"

echo -e "${cyan}Installing...${reset}"
tar -xzf "/tmp/envhub.tar.gz" -C "$BIN_DIR"
rm "/tmp/envhub.tar.gz"

# Alias or Link for easy access
# We'll call the tui 'envhub' for the user
if [ -f "$BIN_DIR/envhub-tui" ]; then
    ln -sf "$BIN_DIR/envhub-tui" "$BIN_DIR/envhub"
fi

echo -e "${green}EnvHub installed successfully!${reset}"
echo ""

# Path instruction
case "$SHELL" in
  */zsh)  SHELL_CONFIG="$HOME/.zshrc" ;;
  */bash) SHELL_CONFIG="$HOME/.bashrc" ;;
  *)      SHELL_CONFIG="$HOME/.profile" ;;
esac

if [[ ":$PATH:" != *":$BIN_DIR:"* ]]; then
    echo -e "${cyan}Adding EnvHub to your PATH...${reset}"
    echo "export PATH=\"\$HOME/.envhub/bin:\$PATH\"" >> "$SHELL_CONFIG"
    echo -e "Added to ${dim}$SHELL_CONFIG${reset}. Please restart your terminal or run:"
    echo -e "${green}source $SHELL_CONFIG${reset}"
else
    echo -e "EnvHub is already in your PATH."
fi

echo ""
echo -e "Run ${green}envhub${reset} to get started!"
