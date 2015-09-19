#!/bin/bash

cd $(dirname $0)/..

make
git add static -f
git commit -am "Publishes to gh-pages."
git push origin `git subtree split --prefix static master`:refs/heads/gh-pages --force
git reset --hard HEAD~1
