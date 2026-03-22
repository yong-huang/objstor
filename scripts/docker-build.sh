#!/bin/bash
# ObjStor Docker Build Script

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"
cd "$PROJECT_DIR"

echo "🐳 ObjStor Docker Build Script"
echo "================================"
echo ""

# Parse arguments
IMAGE_NAME="${IMAGE_NAME:-objstor}"
IMAGE_TAG="${IMAGE_TAG:-latest}"
DOCKER_REGISTRY="${DOCKER_REGISTRY:-}"

# Full image name
if [ -n "$DOCKER_REGISTRY" ]; then
    FULL_IMAGE_NAME="$DOCKER_REGISTRY/$IMAGE_NAME:$IMAGE_TAG"
else
    FULL_IMAGE_NAME="$IMAGE_NAME:$IMAGE_TAG"
fi

echo "📦 Build Configuration:"
echo "  Image: $FULL_IMAGE_NAME"
echo "  Context: $PROJECT_DIR"
echo ""

# Build options
BUILD_ARGS=""
if [ "$NO_CACHE" = "true" ]; then
    BUILD_ARGS="--no-cache"
fi

if [ "$BUILDKIT_OUTPUT" != "" ]; then
    export DOCKER_BUILDKIT=1
fi

# Build the image
echo "🔨 Building Docker image..."
docker build \
    $BUILD_ARGS \
    --build-arg "CargoInstallFlags=--locked" \
    -t "$FULL_IMAGE_NAME" \
    .

echo ""
echo "✅ Build completed successfully!"
echo ""
echo "📝 Image Information:"
docker images "$IMAGE_NAME" "$IMAGE_TAG"
echo ""
echo "🚀 To run the container:"
echo "  docker run -d -p 8080:8080 -v objstor_data:/app/data $FULL_IMAGE_NAME"
echo ""
echo "📋 Or use docker-compose:"
echo "  docker-compose up -d"
