#!/bin/bash

cd $(dirname $0)/..
git push origin `git subtree split --prefix static master`:refs/heads/gh-pages --force
