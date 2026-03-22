#!/bin/bash
# ObjStor Docker Push Script

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"
cd "$PROJECT_DIR"

echo "📤 ObjStor Docker Push Script"
echo "=============================="
echo ""

# Check if registry is set
if [ -z "$DOCKER_REGISTRY" ]; then
    echo "❌ Error: DOCKER_REGISTRY environment variable is not set"
    echo ""
    echo "Usage:"
    echo "  export DOCKER_REGISTRY=registry.example.com"
    echo "  ./scripts/docker-push.sh"
    exit 1
fi

# Parse arguments
IMAGE_NAME="${IMAGE_NAME:-objstor}"
IMAGE_TAG="${IMAGE_TAG:-latest}"
FULL_IMAGE_NAME="$DOCKER_REGISTRY/$IMAGE_NAME:$IMAGE_TAG"

echo "📦 Push Configuration:"
echo "  Registry: $DOCKER_REGISTRY"
echo "  Image: $IMAGE_NAME:$IMAGE_TAG"
echo "  Target: $FULL_IMAGE_NAME"
echo ""

# Check if image exists locally
if ! docker images "$DOCKER_REGISTRY/$IMAGE_NAME" "$IMAGE_TAG" | grep -q "$IMAGE_TAG"; then
    echo "❌ Error: Image $FULL_IMAGE_NAME not found locally"
    echo ""
    echo "Please build the image first:"
    echo "  ./scripts/docker-build.sh"
    exit 1
fi

# Tag image if using different tag
if [ -n "$ADDITIONAL_TAGS" ]; then
    echo "🏷️  Adding additional tags..."
    for tag in $ADDITIONAL_TAGS; do
        docker tag "$FULL_IMAGE_NAME" "$DOCKER_REGISTRY/$IMAGE_NAME:$tag"
        echo "  Tagged: $DOCKER_REGISTRY/$IMAGE_NAME:$tag"
    done
    echo ""
fi

# Push image
echo "📤 Pushing image to registry..."
docker push "$FULL_IMAGE_NAME"

# Push additional tags if specified
if [ -n "$ADDITIONAL_TAGS" ]; then
    for tag in $ADDITIONAL_TAGS; do
        echo "📤 Pushing tag: $tag"
        docker push "$DOCKER_REGISTRY/$IMAGE_NAME:$tag"
    done
fi

echo ""
echo "✅ Push completed successfully!"
echo ""
echo "📋 Image is now available at: $FULL_IMAGE_NAME"
