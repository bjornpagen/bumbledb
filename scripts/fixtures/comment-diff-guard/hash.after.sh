#!/usr/bin/env bash
msg="not a # comment"
sum=0
for n in "$@"; do
  sum=$((sum + n))
done
echo "$sum"
