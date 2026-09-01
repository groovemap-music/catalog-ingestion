set shell := ["bash", "-euo", "pipefail", "-c"]

cargo_audit_tool := "cargo:cargo-audit@0.22.2"
cargo_cyclonedx_tool := "cargo:cargo-cyclonedx@0.5.9"
cargo_deny_tool := "cargo:cargo-deny@0.20.2"
cargo_llvm_cov_tool := "cargo:cargo-llvm-cov@0.9.0"

default:
    @just --list

bootstrap:
    mise install --jobs 1
    mise install --jobs 1 {{cargo_audit_tool}} {{cargo_cyclonedx_tool}} {{cargo_deny_tool}} {{cargo_llvm_cov_tool}}

setup: bootstrap
    cargo fetch --locked

check: bootstrap format-check lint test contract-check repository-check repository-tests build-check license-check secret-scan bump-preview

format:
    cargo fmt --all

format-check:
    cargo fmt --all -- --check

lint:
    cargo clippy --all-targets --all-features --locked -- -D warnings

test:
    cargo test --all-features --locked

coverage: bootstrap
    cargo clean
    if [[ "$(uname -s)" == Linux ]]; then export RUSTFLAGS="${RUSTFLAGS:+${RUSTFLAGS} }-C link-arg=-fuse-ld=bfd"; fi; mise exec {{cargo_llvm_cov_tool}} -- cargo llvm-cov --all-features --locked --lcov --output-path lcov.info

contract:
    python contracts/generate.py

contract-check:
    python contracts/generate.py --check

repository-check:
    python scripts/check-repository.py

publication-history-test:
    PYTHONDONTWRITEBYTECODE=1 python -m unittest discover -s tests -p 'test_publication_history.py'

repository-tests:
    PYTHONDONTWRITEBYTECODE=1 python -m unittest discover -s tests -p 'test_*.py'

history-rehearsal source-repository output-directory:
    PLANNING_ARCHIVE_REPO="${PLANNING_ARCHIVE_REPO}" bash scripts/rehearse-publication-history.sh "{{ source-repository }}" "{{ output-directory }}"

build:
    cargo build --release --locked

install-check:
    test -x target/release/extractor
    install_root="$(mktemp -d)"; trap 'rm -rf "${install_root}"' EXIT; install -m 0755 target/release/extractor "${install_root}/catalog-ingestion"; "${install_root}/catalog-ingestion" --help >/dev/null

build-check:
    cargo check --all-targets --all-features --locked

license-check: bootstrap
    mise exec {{cargo_deny_tool}} -- cargo deny --log-level error check licenses bans sources

secret-scan:
    gitleaks git --redact --no-banner
    gitleaks dir . --redact --no-banner

audit: bootstrap
    mise exec {{cargo_audit_tool}} -- cargo audit

cyclonedx: bootstrap
    mise exec {{cargo_cyclonedx_tool}} -- cargo cyclonedx --format json

image:
    docker build --pull --tag catalog-ingestion:local .

bump-preview:
    python scripts/check_bump_preview.py

# Update Cargo metadata and changelog only; do not commit, tag, push, or publish.
bump:
    uvx --from commitizen==4.9.1 cz bump --version-files-only --changelog --yes --check-consistency

release-dry-run: check
    bash scripts/release-dry-run.sh
