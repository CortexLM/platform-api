#!/bin/bash

set -e

echo "🔒 Obtaining Let's Encrypt certificate using DNS challenge"
echo ""

# Obtain certificate with manual DNS challenge
certbot certonly --manual \
  --preferred-challenges dns \
  -d api.platform.network \
  --email admin@platform.network \
  --agree-tos \
  --manual-public-ip-logging-ok

echo ""
echo "✅ Certificate obtained!"
echo ""
echo "Certificates location:"
echo "  Certificate: /etc/letsencrypt/live/api.platform.network/fullchain.pem"
echo "  Private Key: /etc/letsencrypt/live/api.platform.network/privkey.pem"
echo ""
echo "To use with platform-api-server:"
echo "  sudo ln -s /etc/letsencrypt/live/api.platform.network/fullchain.pem certs/cert.pem"
echo "  sudo ln -s /etc/letsencrypt/live/api.platform.network/privkey.pem certs/key.pem"
echo ""
echo "Run platform-api-server with:"
echo "  ./target/release/platform-api-server --tls-cert certs/cert.pem --tls-key certs/key.pem --port 443"

