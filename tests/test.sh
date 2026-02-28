#!/bin/bash
OUTPUT_FOLDER="./RandomFiles"

# Create directory if it doesn't exist
mkdir -p "$OUTPUT_FOLDER"

# Sizes in MB (Linux tools work nicely with MB blocks)
START_SIZE_MB=1024      # 1GB
END_SIZE_MB=5120        # 5GB
STEP_MB=500

for ((size=START_SIZE_MB; size<=END_SIZE_MB; size+=STEP_MB))
do
    # Generate random filename
    RANDOM_NAME="$(tr -dc a-z0-9 </dev/urandom | head -c 8).bin"
    FILE_PATH="$OUTPUT_FOLDER/$RANDOM_NAME"

    # Create empty file of specified size (fast, sparse file)
    truncate -s "${size}M" "$FILE_PATH"

    echo "Created $RANDOM_NAME with size $size MB"
done

echo "Done!"