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
curl --proto '=https' --tlsv1.2 -LsSf "$URL" | tar -xJ --strip-components=1 -C "$DEST_FOLDER"

# Check if the extraction was successful
if [ $? -eq 0 ]; then
    echo "Successfully downloaded and extracted Doppler ${VERSION} to $DEST_FOLDER"
else
    echo "Error: Failed to download or extract Doppler ${VERSION}"
    exit 1
fi