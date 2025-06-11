#!/bin/bash

# Create certificates directory
mkdir -p certs

# Generate private key for blockchain service (port 3001)
openssl genrsa -out certs/blockchain-key.pem 2048

# Generate certificate for blockchain service
openssl req -new -x509 -key certs/blockchain-key.pem -out certs/blockchain-cert.pem -days 365 -subj "/C=US/ST=CA/L=SF/O=Fleet/OU=Blockchain/CN=localhost"

# Generate private key for host service (port 3000)
openssl genrsa -out certs/host-key.pem 2048

# Generate certificate for host service  
openssl req -new -x509 -key certs/host-key.pem -out certs/host-cert.pem -days 365 -subj "/C=US/ST=CA/L=SF/O=Fleet/OU=Host/CN=localhost"

echo "Certificates generated successfully!"
echo "Blockchain cert: certs/blockchain-cert.pem"
echo "Blockchain key: certs/blockchain-key.pem"
echo "Host cert: certs/host-cert.pem" 
echo "Host key: certs/host-key.pem"
