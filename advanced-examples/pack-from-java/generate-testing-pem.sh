#!/bin/bash
# Copyright 2025 Google LLC
#
# Licensed under the Apache License, Version 2.0 (the "License");
# you may not use this file except in compliance with the License.
# You may obtain a copy of the License at
#
#     http://www.apache.org/licenses/LICENSE-2.0
#
# Unless required by applicable law or agreed to in writing, software
# distributed under the License is distributed on an "AS IS" BASIS,
# WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
# See the License for the specific language governing permissions and
# limitations under the License.


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
