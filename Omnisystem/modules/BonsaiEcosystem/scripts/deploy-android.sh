#!/usr/bin/env bash
set -euo pipefail

echo "Deploy Android placeholder script"
if [ -d "android-runtime" ]; then
  (cd android-runtime && ./gradlew assembleRelease)
else
  echo "android-runtime not found"
  exit 1
fi
