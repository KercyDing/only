#!/bin/sh
set -e

REPO="KercyDing/only"
VERSION="${ONLY_VERSION:-latest}"
INSTALL_DIR="${ONLY_INSTALL_DIR:-}"

OS="$(uname -s)"
ARCH="$(uname -m)"

case "$OS" in
    Linux*)
        case "$ARCH" in
            x86_64 | amd64)
                BINARY="only-linux-amd64"
                ;;
            *)
                echo "Error: unsupported Linux architecture: $ARCH"
                exit 1
                ;;
        esac
        ;;
    Darwin*)
        case "$ARCH" in
            x86_64 | amd64)
                BINARY="only-darwin-amd64"
                ;;
            arm64 | aarch64)
                BINARY="only-darwin-arm64"
                ;;
            *)
                echo "Error: unsupported macOS architecture: $ARCH"
                exit 1
                ;;
        esac
        ;;
    *)
        echo "Error: unsupported operating system: $OS"
        exit 1
        ;;
esac

if [ "$VERSION" = "latest" ]; then
    DOWNLOAD_URL="https://github.com/$REPO/releases/latest/download/$BINARY"
else
    DOWNLOAD_URL="https://github.com/$REPO/releases/download/$VERSION/$BINARY"
fi

if [ -z "$INSTALL_DIR" ]; then
    if [ -w "/usr/local/bin" ]; then
        INSTALL_DIR="/usr/local/bin"
    else
        INSTALL_DIR="$HOME/.local/bin"
    fi
fi

mkdir -p "$INSTALL_DIR"

TEMP_FILE="$(mktemp)"
cleanup() {
    if [ -n "$TEMP_FILE" ] && [ -f "$TEMP_FILE" ]; then
        rm -f "$TEMP_FILE"
    fi
}
trap cleanup EXIT INT TERM

echo "Downloading only for $OS $ARCH..."
if command -v curl >/dev/null 2>&1; then
    curl -fsSL "$DOWNLOAD_URL" -o "$TEMP_FILE"
elif command -v wget >/dev/null 2>&1; then
    wget -q "$DOWNLOAD_URL" -O "$TEMP_FILE"
else
    echo "Error: curl or wget is required"
    exit 1
fi

chmod +x "$TEMP_FILE"

echo "Installing only to $INSTALL_DIR..."
mv "$TEMP_FILE" "$INSTALL_DIR/only"
TEMP_FILE=""

case ":$PATH:" in
    *":$INSTALL_DIR:"*) ;;
    *)
        SHELL_NAME="$(basename "${SHELL:-sh}")"
        case "$SHELL_NAME" in
            bash)
                SHELL_RC="$HOME/.bashrc"
                PATH_LINE="export PATH=\"$INSTALL_DIR:\$PATH\""
                ;;
            zsh)
                SHELL_RC="$HOME/.zshrc"
                PATH_LINE="export PATH=\"$INSTALL_DIR:\$PATH\""
                ;;
            fish)
                SHELL_RC="$HOME/.config/fish/config.fish"
                PATH_LINE="set -gx PATH \"$INSTALL_DIR\" \$PATH"
                ;;
            *)
                SHELL_RC="$HOME/.profile"
                PATH_LINE="export PATH=\"$INSTALL_DIR:\$PATH\""
                ;;
        esac

        mkdir -p "$(dirname "$SHELL_RC")"
        if [ ! -f "$SHELL_RC" ] || ! grep -Fq "$INSTALL_DIR" "$SHELL_RC"; then
            {
                echo ""
                echo "# only path"
                echo "$PATH_LINE"
            } >> "$SHELL_RC"
            echo "Added $INSTALL_DIR to PATH in $SHELL_RC"
        fi
        ;;
esac

echo "only installed successfully!"
"$INSTALL_DIR/only" --version
