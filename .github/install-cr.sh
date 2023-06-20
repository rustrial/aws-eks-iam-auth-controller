#!/bin/bash

set -e

version=v1.5.0
curl -sSLo /tmp/cr.tar.gz "https://github.com/helm/chart-releaser/releases/download/$version/chart-releaser_${version#v}_linux_amd64.tar.gz"
tar -xzvf /tmp/cr.tar.gz -C "/tmp"
rm -f /tmp/cr.tar.gz

mkdir -p .cr-release-packages

mkdir .cr-index