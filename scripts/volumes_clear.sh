#!/bin/bash
# Remove all data directories
rm -rfv ./data

# Remove the doppler.db file if it exists
if [ -f doppler.db ]; then
    echo "Removing doppler.db file..."
    rm -fv doppler.db
fi
