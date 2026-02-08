#!/bin/bash
set -e

cd "$(dirname "$0")/.."

echo "Building Docker image..."
docker build -t localhost:32000/pdf-table-extractor:latest .

echo "Pushing to local registry..."
docker push localhost:32000/pdf-table-extractor:latest

echo "Restarting deployment..."
microk8s kubectl rollout restart deployment/pdf-table-extractor

echo "Waiting for rollout..."
microk8s kubectl rollout status deployment/pdf-table-extractor

echo "Done! Service available at pdf-table-extractor:80"
