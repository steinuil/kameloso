#!/bin/sh
docker build . -t kameloso/cross-armv7
docker run --rm -v "$(pwd)":/app kameloso/cross-armv7