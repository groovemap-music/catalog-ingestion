#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
dist_dir="${repo_root}/dist"

mkdir -p "${dist_dir}"
find "${dist_dir}" -mindepth 1 -maxdepth 1 -delete
cargo build --release --locked
cp "${repo_root}/target/release/extractor" "${dist_dir}/catalog-ingestion"
tar -C "${dist_dir}" -czf "${dist_dir}/catalog-ingestion.tar.gz" catalog-ingestion
shasum -a 256 "${dist_dir}/catalog-ingestion.tar.gz" > "${dist_dir}/SHA256SUMS"
just --justfile "${repo_root}/Justfile" cyclonedx
mv "${repo_root}/extractor.cdx.json" "${dist_dir}/catalog-ingestion.cdx.json"
python "${repo_root}/scripts/write-third-party-notices.py"
test -s "${dist_dir}/SHA256SUMS"
test -s "${dist_dir}/catalog-ingestion.cdx.json"
test -s "${dist_dir}/THIRD_PARTY_NOTICES.json"
