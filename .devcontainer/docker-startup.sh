#!/bin/bash
set -e

# Check if dockerd is already running
if ! pgrep -x dockerd > /dev/null; then
    echo "Starting Docker daemon..."
    sudo dockerd --log-level=error > /tmp/dockerd.log 2>&1 &
    sleep 2
fi

# Wait for Docker to be ready
until docker ps > /dev/null 2>&1; do
    echo "Waiting for Docker..."
    sleep 1
done

echo "Docker is ready!"
