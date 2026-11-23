#!/usr/bin/env bash
# Download every paper listed in papers.tsv into pdfs/ and record SHA-256 checksums.
# PDFs are third-party content and are never committed; rerun this script to reproduce locally.
set -euo pipefail
cd "$(dirname "$0")"
mkdir -p pdfs
: > checksums.sha256.tmp
tail -n +2 papers.tsv | while IFS=$'\t' read -r id filename url source topic license; do
  out="pdfs/$filename"
  if [[ ! -s "$out" ]]; then
    echo "fetching $id"
    curl -fsSL --retry 2 -A "housing-challenge-research/1.0" -o "$out" "$url" || { echo "FAILED $id ($url)" >&2; rm -f "$out"; continue; }
    sleep 3  # be polite to arXiv and other hosts
  fi
  if [[ "$(head -c 4 "$out")" != "%PDF" ]]; then
    echo "NOT A PDF $id ($url)" >&2; rm -f "$out"; continue
  fi
  (cd pdfs && shasum -a 256 "$filename") >> checksums.sha256.tmp
done
sort -k2 checksums.sha256.tmp > checksums.sha256 && rm checksums.sha256.tmp
echo "accessed $(date -u +%Y-%m-%d)" > ACCESS_DATE
