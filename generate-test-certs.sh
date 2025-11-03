#!/bin/bash

set -e

echo "🔒 Generating test TLS certificates for api.platform.network"

# Create certs directory
mkdir -p certs

# Generate private key
openssl genrsa -out certs/key.pem 2048

# Generate certificate signing request
openssl req -new -key certs/key.pem -out certs/cert.csr -subj "/CN=api.platform.network/O=Platform/C=US"

# Generate self-signed certificate (valid for 365 days)
openssl x509 -req -days 365 -in certs/cert.csr -signkey certs/key.pem -out certs/cert.pem

# Clean up CSR
rm certs/cert.csr

echo "✅ Certificates generated:"
echo "   - certs/cert.pem (certificate)"
echo "   - certs/key.pem (private key)"
echo ""
echo "🔒 To use with platform-api-server:"
echo "   ./target/release/platform-api-server --tls-cert certs/cert.pem --tls-key certs/key.pem"

