#!/usr/bin/env bash
set -euo pipefail

REGISTRY="registry.gabrielkaszewski.dev"
REPO="movies-diary"
DEFAULT_FEATURES="sqlite,sqlite-federation,nats"

features="${FEATURES:-$DEFAULT_FEATURES}"
tag="latest"

while [[ $# -gt 0 ]]; do
  case $1 in
    --features) features="$2"; shift 2 ;;
    --tag)      tag="$2";      shift 2 ;;
    *) echo "usage: $0 [--features F] [--tag T]" >&2; exit 1 ;;
  esac
done

image="${REGISTRY}/${REPO}:${tag}"

echo "building ${image}  features=${features}"
docker buildx build --platform linux/amd64 \
  --build-arg FEATURES="$features" \
  -t "$image" --push .
echo "pushed $image"
