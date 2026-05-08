#!/usr/bin/env bash
set -euo pipefail

IMAGE="registry.gabrielkaszewski.dev/movies-diary:latest"

docker buildx build --platform linux/amd64 -t "$IMAGE" --push .
echo "pushed $IMAGE"
