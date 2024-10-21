#!/bin/bash

# Function to check if a command exists
command_exists() {
    command -v "$1" >/dev/null 2>&1
}

# Check if curl is installed
if ! command_exists curl; then
    echo "Error: curl is not installed. Please install curl and try again."
    exit 1
fi

# Check if tar is installed
if ! command_exists tar; then
    echo "Error: tar is not installed. Please install tar and try again."
    exit 1
fi

export NVM_DIR="$HOME/.nvm"
[ -s "$NVM_DIR/nvm.sh" ] && \. "$NVM_DIR/nvm.sh"  # This loads nvm

# Check if nvm is installed
if ! command -v nvm &> /dev/null; then
    echo "Error: nvm is not installed. Please install nvm via 'https://github.com/nvm-sh/nvm' and try again."
    exit 1
fi

nvm install --lts

# Function to get OS and architecture
get_os_arch() {
    local os=$(uname -s | tr '[:upper:]' '[:lower:]')
    local arch=$(uname -m)

    case "$os" in
        linux*)
            os="linux"
            ;;
        darwin*)
            os="darwin"
            ;;
        *)
            echo "Unsupported OS: $os"
            exit 1
            ;;
    esac

    case "$arch" in
        x86_64)
            arch="x86_64"
            ;;
        aarch64|arm64)
            arch="aarch64"
            ;;
        *)
            echo "Unsupported architecture: $arch"
            exit 1
            ;;
    esac

    echo "${os}-${arch}"
}

# Check if version is provided
if [ $# -eq 0 ]; then
    echo "Error: Please provide a version number."
    echo "Usage: $0 <version>"
    exit 1
fi

VERSION="$1"
OS_ARCH=$(get_os_arch)

# Base URL
BASE_URL="https://github.com/tee8z/doppler/releases/download"

get_cpu_architecture() {
    if [[ "$(uname)" == "Darwin" ]]; then
        echo $(uname -m)
    else
        echo $(uname -m)
    fi
}

system=$(uname | tr '[:upper:]' '[:lower:]')
arch=$(get_cpu_architecture)

if [[ "$system" == "linux" ]]; then
    filename="doppler-${arch}-unknown-linux-gnu.tar.xz"
elif [[ "$system" == "darwin" ]]; then
    if [[ "$arch" == "x86_64" ]]; then
        filename="doppler-x86_64-apple-darwin.tar.xz"
    elif [[ "$arch" == "arm64" ]] || [[ "$arch" == "aarch64" ]]; then
        filename="doppler-aarch64-apple-darwin.tar.xz"
    fi
else
    filename="doppler-${system}-${arch}.tar.xz"
fi

# Construct the full URL
URL="${BASE_URL}/${VERSION}/${filename}"

echo "Filename: $filename"

# Destination folder
DEST_FOLDER="$HOME/.doppler"

# Create the destination folder if it doesn't exist
mkdir -p "$DEST_FOLDER"

echo "Downloading Doppler @ ${URL}"

# Extract the base name from the filename (remove .tar.xz)
BASE_NAME=$(basename "$filename" .tar.xz)

# Set the destination folder
DEST_FOLDER="$HOME/.doppler/${VERSION}"

# Create the destination folder
mkdir -p "$DEST_FOLDER"

# Download and extract the file
#curl --proto '=https' --tlsv1.2 -LsSf "$URL" | tar -xJ --strip-components=1 -C "$DEST_FOLDER"
cp -r "$HOME/repos/doppler/target/distrib/doppler-x86_64-unknown-linux-gnu/"* "$DEST_FOLDER"

# Check if the extraction was successful
if [ $? -eq 0 ]; then
    echo "Successfully downloaded and extracted Doppler ${VERSION} to $DEST_FOLDER"
else
    echo "Error: Failed to download or extract Doppler ${VERSION}"
    exit 1
fi
# Path to the original configuration file
CONFIG_FILE="$HOME/.doppler/${VERSION}/build/ui_config/server.conf.ini"

# Read the content of the original file
if [ ! -f "$CONFIG_FILE" ]; then
    echo "Error: Configuration file not found at $CONFIG_FILE"
    exit 1
fi

CONFIG_CONTENT=$(cat "$CONFIG_FILE")

# Update the paths by replacing $DEST_FOLDER with its actual value
UPDATED_CONFIG=$(echo "$CONFIG_CONTENT" | sed "s|\\\$DEST_FOLDER|$DEST_FOLDER|g")

# Save the updated configuration back to the original file
echo "$UPDATED_CONFIG" > "$CONFIG_FILE"

echo "Configuration updated in $CONFIG_FILE"

# Display the changes (optional)
echo "Changes made:"
diff <(echo "$CONFIG_CONTENT") <(echo "$UPDATED_CONFIG")

if [ -f "$DEST_FOLDER/package.json" ]; then
    mv "$DEST_FOLDER/package.json" "$DEST_FOLDER/build/"
else
    echo "Warning: package.json not found in $DEST_FOLDER"
fi

if [ -f "$DEST_FOLDER/package-lock.json" ]; then
    mv "$DEST_FOLDER/package-lock.json" "$DEST_FOLDER/build/"
else
    echo "Warning: package-lock.json not found in $DEST_FOLDER"
fi

if [ -d "$DEST_FOLDER/build/" ]; then
    echo "Changing to $DEST_FOLDER/build/ and running npm install"
    (
        cd "$DEST_FOLDER/build/" && \
        npm install || echo "npm install failed"
    )
else
    echo "Error: $DEST_FOLDER/build/ does not exist"
fi

# Create .env file
cat << EOF > "$DEST_FOLDER/.env"
USERID=1000
GROUPID=1000
EOF

echo ".env file created in $DEST_FOLDER"

echo "Install completed."
