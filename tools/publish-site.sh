#!/bin/bash

cd $(dirname $0)/..
make 
git add -f static/*
git commit -am "Publishes site update."
git push origin `git subtree split --prefix static master`:refs/heads/gh-pages --force
