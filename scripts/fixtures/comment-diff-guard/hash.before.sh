#!/usr/bin/env bash
# narration: add the arguments
msg="not a # comment"
sum=0 # trailing fold
for n in "$@"; do
  sum=$((sum + n))
done
echo "$sum"
