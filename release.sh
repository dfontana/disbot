#!/bin/bash
set -e

SHA=${1:0:8}
MESSAGE=${2}

if [ "$#" -ne 2 ]; then
  echo "Must provide SHA to tag & a message"
  exit 1
fi


TAG="v$SHA"
echo git tag $TAG $SHA -m\"$MESSAGE\"
echo git push origin $SHA
