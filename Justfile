set shell := ["bash", "-euo", "pipefail", "-c"]

default:
    @just --list

setup:
    cargo fetch --locked

check: format-check lint test contract-check repository-check build-check license-check secret-scan bump-preview

format:
    cargo fmt --all

format-check:
    cargo fmt --all -- --check

lint:
    cargo clippy --all-targets --all-features --locked -- -D warnings

test:
    cargo test --all-features --locked

contract:
    python contracts/generate.py

contract-check:
    python contracts/generate.py --check

repository-check:
    python scripts/check-repository.py

build:
    cargo build --release --locked

build-check:
    cargo check --all-targets --all-features --locked

license-check:
    cargo deny --log-level error check licenses bans sources

secret-scan:
    gitleaks git --redact --no-banner
    gitleaks dir . --redact --no-banner

audit:
    cargo audit

image:
    docker build --pull --tag catalog-ingestion:local .

bump-preview:
    uvx --from commitizen==4.9.1 cz bump --dry-run --changelog --yes --check-consistency

# Update Cargo metadata and changelog only; do not commit, tag, push, or publish.
bump:
    uvx --from commitizen==4.9.1 cz bump --version-files-only --changelog --yes --check-consistency

release-dry-run: check
    bash scripts/release-dry-run.sh
