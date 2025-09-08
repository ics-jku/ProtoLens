#!/bin/bash
set -e

echo "Building frontend"
cd PLW
npm install
npm run build

cd ..

echo ""
echo "Copying frontend"
cd PLS
cp -r ../PLW/dist ./

echo ""
echo "Building server"
cargo build -r
cargo run -r
