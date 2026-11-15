#!/bin/sh
set -eu
cp browser/index.html browser/app.js browser/settings.js browser/worker.js output/browser/
node scripts/build-spatial-options.cjs
