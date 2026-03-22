#!/bin/bash
# CLI test script for ObjStor

set -e

echo "🧪 ObjStor CLI Integration Tests"
echo "================================="
echo ""

# Configuration
ENDPOINT="${ENDPOINT:-http://localhost:8080}"
BUCKET="test-cli-bucket-$$"
TEST_FILE="/tmp/objstor-test-$$"
ACCESS_KEY="test-access-key"
SECRET_KEY="test-secret-key"

# Cleanup function
cleanup() {
    echo ""
    echo "🧹 Cleaning up..."

    # Delete test objects
    aws s3 rm s3://$BUCKET/test-file.txt \
        --endpoint-url $ENDPOINT \
        --access-key-id $ACCESS_KEY \
        --secret-access-key $SECRET_KEY \
        2>/dev/null || true

    # Delete test bucket
    aws s3 rb s3://$BUCKET \
        --endpoint-url $ENDPOINT \
        --access-key-id $ACCESS_KEY \
        --secret-access-key $SECRET_KEY \
        --force 2>/dev/null || true

    # Delete test file
    rm -f "$TEST_FILE"

    echo "Cleanup complete"
}

# Set trap for cleanup
trap cleanup EXIT

# Check if AWS CLI is available
if ! command -v aws &> /dev/null; then
    echo "❌ AWS CLI not found. Please install it first:"
    echo "   macOS: brew install awscli"
    echo "   Linux: apt-get install awscli"
    exit 1
fi

# Check if server is running
echo "🔍 Checking server status..."
if ! curl -s "$ENDPOINT/health" > /dev/null 2>&1; then
    echo "❌ Server is not running at $ENDPOINT"
    echo "   Please start it first: cargo run"
    exit 1
fi
echo "✅ Server is running"
echo ""

# Create test file
echo "📝 Creating test file..."
echo "This is a test file for ObjStor" > "$TEST_FILE"

# Test 1: List buckets (should be empty or show existing)
echo "📋 Test 1: List buckets"
aws s3 ls \
    --endpoint-url $ENDPOINT \
    --access-key-id $ACCESS_KEY \
    --secret-access-key $SECRET_KEY \
    || echo "Failed to list buckets"
echo ""

# Test 2: Create bucket
echo "🪣 Test 2: Create bucket"
aws s3 mb s3://$BUCKET \
    --endpoint-url $ENDPOINT \
    --access-key-id $ACCESS_KEY \
    --secret-access-key $SECRET_KEY \
    || { echo "❌ Failed to create bucket"; exit 1; }
echo "✅ Bucket created: $BUCKET"
echo ""

# Test 3: List buckets again
echo "📋 Test 3: Verify bucket in list"
aws s3 ls \
    --endpoint-url $ENDPOINT \
    --access-key-id $ACCESS_KEY \
    --secret-access-key $SECRET_KEY | grep -q "$BUCKET" || { echo "❌ Bucket not found in list"; exit 1; }
echo "✅ Bucket found in list"
echo ""

# Test 4: Upload file
echo "📤 Test 4: Upload file"
aws s3 cp "$TEST_FILE" s3://$BUCKET/test-file.txt \
    --endpoint-url $ENDPOINT \
    --access-key-id $ACCESS_KEY \
    --secret-access-key $SECRET_KEY \
    || { echo "❌ Failed to upload file"; exit 1; }
echo "✅ File uploaded"
echo ""

# Test 5: List objects
echo "📋 Test 5: List objects"
aws s3 ls s3://$BUCKET \
    --endpoint-url $ENDPOINT \
    --access-key-id $ACCESS_KEY \
    --secret-access-key $SECRET_KEY | grep -q "test-file.txt" || { echo "❌ File not found in list"; exit 1; }
echo "✅ File found in bucket"
echo ""

# Test 6: Download file
echo "📥 Test 6: Download file"
DOWNLOAD_FILE="/tmp/downloaded-$$"
aws s3 cp s3://$BUCKET/test-file.txt "$DOWNLOAD_FILE" \
    --endpoint-url $ENDPOINT \
    --access-key-id $ACCESS_KEY \
    --secret-access-key $SECRET_KEY \
    || { echo "❌ Failed to download file"; exit 1; }

# Verify content
if ! cmp -s "$TEST_FILE" "$DOWNLOAD_FILE"; then
    echo "❌ Downloaded file content doesn't match"
    rm -f "$DOWNLOAD_FILE"
    exit 1
fi
rm -f "$DOWNLOAD_FILE"
echo "✅ File downloaded and verified"
echo ""

# Test 7: Head object (metadata)
echo "🔍 Test 7: Get object metadata"
METADATA=$(aws s3api head-object \
    --bucket $BUCKET \
    --key test-file.txt \
    --endpoint-url $ENDPOINT \
    --access-key-id $ACCESS_KEY \
    --secret-access-key $SECRET_KEY \
    2>&1)

if echo "$METADATA" | grep -q "ETag"; then
    echo "✅ Object metadata retrieved"
else
    echo "❌ Failed to get object metadata"
fi
echo ""

# Test 8: Delete object
echo "🗑️  Test 8: Delete object"
aws s3 rm s3://$BUCKET/test-file.txt \
    --endpoint-url $ENDPOINT \
    --access-key-id $ACCESS_KEY \
    --secret-access-key $SECRET_KEY \
    || { echo "❌ Failed to delete object"; exit 1; }
echo "✅ Object deleted"
echo ""

# Test 9: Verify object is gone
echo "🔍 Test 9: Verify object deletion"
if aws s3 ls s3://$BUCKET \
    --endpoint-url $ENDPOINT \
    --access-key-id $ACCESS_KEY \
    --secret-access-key $SECRET_KEY | grep -q "test-file.txt"; then
    echo "❌ File still exists after deletion"
    exit 1
fi
echo "✅ Object successfully deleted"
echo ""

# Test 10: Delete bucket
echo "🗑️  Test 10: Delete bucket"
aws s3 rb s3://$BUCKET \
    --endpoint-url $ENDPOINT \
    --access-key-id $ACCESS_KEY \
    --secret-access-key $SECRET_KEY \
    || { echo "❌ Failed to delete bucket"; exit 1; }
echo "✅ Bucket deleted"
echo ""

# Test 11: Verify bucket is gone
echo "🔍 Test 11: Verify bucket deletion"
if aws s3 ls \
    --endpoint-url $ENDPOINT \
    --access-key-id $ACCESS_KEY \
    --secret-access-key $SECRET_KEY | grep -q "$BUCKET"; then
    echo "❌ Bucket still exists after deletion"
    exit 1
fi
echo "✅ Bucket successfully deleted"
echo ""

echo "======================================"
echo "✅ All CLI tests passed!"
echo "======================================"
