#!/bin/bash
set -e

BOOKMARKS_FILE="/mnt/c/Users/guill/AppData/Local/Microsoft/Edge/User Data/Default/Bookmarks"
TEMP_OUTPUT="/tmp/edge_bookmarks_normalized.json"
MAX_RETRIES=5
RETRY_DELAY=2

echo "Starting normalization of Microsoft Edge bookmarks..."

# Run normalization
echo "Running normalization..."
cargo run -- bookmarks normalize --input "$BOOKMARKS_FILE" --output "$TEMP_OUTPUT" --emit-events

# Verify the output was created
if [ ! -f "$TEMP_OUTPUT" ]; then
    echo "Error: Normalized output not created"
    exit 1
fi

echo "Normalization completed. Attempting to write back to original file..."

# Retry loop for writing back
for i in $(seq 1 $MAX_RETRIES); do
    echo "Attempt $i of $MAX_RETRIES: Writing to $BOOKMARKS_FILE"
    
    # Try to copy with force
    if cp -f "$TEMP_OUTPUT" "$BOOKMARKS_FILE"; then
        echo "Successfully wrote to $BOOKMARKS_FILE"
        
        # Verify the file was written correctly
        if cmp -s "$TEMP_OUTPUT" "$BOOKMARKS_FILE"; then
            echo "File write verified successfully"
            
            # Validate the written file
            if cargo run -- bookmarks validate --input "$BOOKMARKS_FILE" >/dev/null 2>&1; then
                echo "Validation successful!"
                rm "$TEMP_OUTPUT"
                echo "Normalization completed successfully"
                exit 0
            else
                echo "Warning: Validation failed, but file was written"
                exit 1
            fi
        else
            echo "Warning: File comparison failed"
        fi
    else
        echo "Write attempt $i failed"
        if [ $i -lt $MAX_RETRIES ]; then
            echo "Waiting $RETRY_DELAY seconds before retry..."
            sleep $RETRY_DELAY
        fi
    fi
done

echo "Failed to write after $MAX_RETRIES attempts"
echo "You may need to close Microsoft Edge completely"
echo "Temporary output saved at: $TEMP_OUTPUT"
exit 1
