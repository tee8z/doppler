#!/bin/bash

# Function to convert hexadecimal to binary and then decode Base64 to ASCII
convert_hex_to_ascii() {
    hex_string="$1"
    for byte in $(echo $hex_string | fold -w2); do
        echo -n -e "\x$byte"
    done
    echo
}

# Check if an argument was provided
if [ -z "$1" ]; then
    echo "Usage: $0 <hex_string>"
    exit 1
fi

# Call the function with the provided argument
convert_hex_to_ascii "$1"