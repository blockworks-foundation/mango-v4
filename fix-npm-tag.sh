#!/usr/bin/env bash

set -ex pipefail

git tag npm-$TAG $TAG; git tag -d $TAG; git push --tags