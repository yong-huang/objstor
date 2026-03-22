#!/bin/bash
# Simple benchmark script for ObjStor

set -e

ENDPOINT="http://localhost:8080"
BUCKET="benchmark-bucket"
FILE_SIZES=("1K" "1M" "10M")
FILE_COUNT=100

echo "ObjStor Benchmark"
echo "=================="
echo ""

# Create bucket
echo "Creating bucket: $BUCKET"
aws s3 mb s3://$BUCKET --endpoint-url $ENDPOINT 2>/dev/null || echo "Bucket may already exist"
echo ""

# Benchmark different file sizes
for SIZE in "${FILE_SIZES[@]}"; do
    echo "Benchmarking $SIZE files..."

    case $SIZE in
        1K)  BLOCK_SIZE=1024;;
        1M)  BLOCK_SIZE=1048576;;
        10M) BLOCK_SIZE=10485760;;
    esac

    # Create test file
    dd if=/dev/zero of=/tmp/test-$SIZE bs=$BLOCK_SIZE count=1 2>/dev/null

    # Upload benchmark
    START=$(date +%s%N)
    for i in $(seq 1 $FILE_COUNT); do
        aws s3 cp /tmp/test-$SIZE s3://$BUCKET/test-$SIZE-$i --endpoint-url $ENDPOINT --quiet
    done
    END=$(date +%s%N)

    UPLOAD_TIME=$((($END - $START) / 1000000))
    UPLOAD_RATE=$(echo "scale=2; $FILE_COUNT * $BLOCK_SIZE / 1024 / 1024 / $UPLOAD_TIME" | bc)

    echo "  Upload: $FILE_COUNT files in ${UPLOAD_TIME}ms (${UPLOAD_RATE} MB/s)"

    # Download benchmark
    START=$(date +%s%N)
    for i in $(seq 1 $FILE_COUNT); do
        aws s3 cp s3://$BUCKET/test-$SIZE-$i /tmp/test-$SIZE-down --endpoint-url $ENDPOINT --quiet
    done
    END=$(date +%s%N)

    DOWNLOAD_TIME=$((($END - $START) / 1000000))
    DOWNLOAD_RATE=$(echo "scale=2; $FILE_COUNT * $BLOCK_SIZE / 1024 / 1024 / $DOWNLOAD_TIME" | bc)

    echo "  Download: $FILE_COUNT files in ${DOWNLOAD_TIME}ms (${DOWNLOAD_RATE} MB/s)"
    echo ""

    # Cleanup
    rm -f /tmp/test-$SIZE /tmp/test-$SIZE-down
done

# Cleanup
echo "Cleaning up..."
aws s3 rb s3://$BUCKET --endpoint-url $ENDPOINT --force

echo "Benchmark complete!"
