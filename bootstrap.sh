#!/bin/sh
# since Blaze is self-hosted, this script is used in order to install Blaze without Blaze.

# launch with USE_SUDO if you have permission issues about npm link and you use sudo on your system.

set -e

root=$(realpath $(dirname $0))

sudo=""
if [ -n "${USE_SUDO+x}" ]; then
    sudo="sudo"
fi

echo "Compiling Node devkit"
cd "$root/node/devkit"
npm install
npm run build

echo "Compiling Node bridge"
cd "$root/node/bridge"
npm install
npm link ../devkit
npm run build

echo "Compiling JSON schemas"
cd "$root/schemas"
npm install
npm run build
npm start

echo "Compiling and installing Blaze CLI"
cd $root
BLAZE_NODE_BRIDGE_BUNDLE_PATH="$root/node/bridge/dist/main.js" \
    BLAZE_JSON_SCHEMAS_LOCATION="$root/schemas/schemas" \
    cargo +nightly install -Z bindeps --path "$root/cli"
