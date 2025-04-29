#!/bin/bash

SUBJECT="/C=US/ST=Test/L=Test/O=Test/OU=Test/CN=test.example.com"
VALIDITY_DAYS=3650  # 10 years
KEY_SIZE=2048

TEMP_DIR=$(mktemp -d)
PRIVATE_KEY="$TEMP_DIR/private.key"
CERTIFICATE="$TEMP_DIR/certificate.crt"
COMBINED="$TEMP_DIR/combined.pem"

# Generate private key
openssl genrsa -out "$PRIVATE_KEY" $KEY_SIZE 2>/dev/null

# Generate self-signed certificate without prompting
openssl req -new -x509 -key "$PRIVATE_KEY" -out "$CERTIFICATE" -days $VALIDITY_DAYS -subj "$SUBJECT" 2>/dev/null

# Concatenate certificate and private key
cat "$CERTIFICATE" "$PRIVATE_KEY" > "$COMBINED"

# Output the combined certificate and key
cat "$COMBINED"

# Clean up temporary files
rm -rf "$TEMP_DIR"
