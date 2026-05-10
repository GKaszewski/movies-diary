#!/usr/bin/env bash
set -euo pipefail

IMAGE="registry.gabrielkaszewski.dev/movies-diary:latest"

docker buildx build --platform linux/amd64 \
  --build-arg FEATURES=sqlite,sqlite-federation,nats \
  -t "$IMAGE" --push .
echo "pushed $IMAGE"
