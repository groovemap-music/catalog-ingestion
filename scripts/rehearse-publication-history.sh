#!/usr/bin/env bash
set -euo pipefail
umask 077

if [[ "$#" -ne 2 ]]; then
  echo "usage: $0 SOURCE_REPOSITORY ABSOLUTE_OUTPUT_DIRECTORY" >&2
  exit 2
fi

source_repository="$1"
output_directory="$2"
archive_commit="daf82a149aaa382b3cebbd4b43d3c82e53d4128e"
archive_repository="${PLANNING_ARCHIVE_REPO:-}"
script_directory="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
boundary_only_path="docs/superpowers/README.md"
boundary_only_oid="ca3b96ae188d756ef40549035cce987742e1ddcc"
boundary_only_sha256="fef85bb4804255946e49000752761e5480ded906d2109973d5e916e57e77925c"

if [[ "$(git -C "${source_repository}" rev-parse --is-inside-work-tree 2>/dev/null || true)" != "true" ]]; then
  echo "SOURCE_REPOSITORY must be a non-bare Git worktree." >&2
  exit 2
fi
if [[ "${output_directory}" != /* ]] || [[ -e "${output_directory}" ]]; then
  echo "ABSOLUTE_OUTPUT_DIRECTORY must be an absolute path that does not exist." >&2
  exit 2
fi
if [[ -z "${archive_repository}" ]] || ! git -C "${archive_repository}" cat-file -e "${archive_commit}^{commit}"; then
  echo "PLANNING_ARCHIVE_REPO must contain prerequisite commit ${archive_commit}." >&2
  exit 2
fi
for command in git git-filter-repo mise shasum; do
  if ! command -v "${command}" >/dev/null; then
    echo "Missing required command: ${command}" >&2
    exit 2
  fi
done

mkdir -m 700 "${output_directory}"
backup_repository="${output_directory}/private-remote-backup.git"
sanitized_repository="${output_directory}/sanitized-candidate.git"
candidate_commit="$(git -C "${source_repository}" rev-parse HEAD)"

git clone --quiet --mirror --no-local "${source_repository}" "${backup_repository}"
git clone --quiet --mirror --no-local "${source_repository}" "${sanitized_repository}"
git -C "${backup_repository}" for-each-ref --format='%(refname)\t%(objectname)\t%(*objectname)' \
  > "${output_directory}/refs-before.tsv"
git -C "${backup_repository}" rev-list --objects --all > "${output_directory}/object-graph-before.txt"

awk '
  $2 == ".planning" || index($2, ".planning/") == 1 ||
  $2 == "docs/superpowers" || index($2, "docs/superpowers/") == 1 ||
  $2 == "docs/specs" || index($2, "docs/specs/") == 1 { print }
' "${output_directory}/object-graph-before.txt" > "${output_directory}/private-objects-before.txt"
if [[ ! -s "${output_directory}/private-objects-before.txt" ]]; then
  echo "Expected private planning objects were not found in source history." >&2
  exit 1
fi
printf 'object_id\tdisposition\tarchive_commit\tpath\n' > "${output_directory}/removed-object-map.tsv"
while read -r object_id relative_path; do
  object_type="$(git -C "${backup_repository}" cat-file -t "${object_id}")"
  if [[ "${object_type}" == "tree" ]]; then
    disposition="structural-boundary"
    mapped_archive="-"
  elif [[ "${relative_path}" == "${boundary_only_path}" ]]; then
    digest="$(git -C "${backup_repository}" cat-file blob "${object_id}" | shasum -a 256 | awk '{print $1}')"
    if [[ "${object_id}" == "${boundary_only_oid}" ]] && [[ "${digest}" == "${boundary_only_sha256}" ]]; then
      disposition="reviewed-boundary-only"
    else
      disposition="unmapped-private"
    fi
    mapped_archive="-"
  else
    disposition="archive-mapped"
    mapped_archive="${archive_commit}"
  fi
  printf '%s\t%s\t%s\t%s\n' "${object_id}" "${disposition}" "${mapped_archive}" "${relative_path}" \
    >> "${output_directory}/removed-object-map.tsv"
done < "${output_directory}/private-objects-before.txt"

# The mirror contains every source ref. The candidate commit becomes the local publication main;
# neither update-ref nor filter-repo can reach or mutate the source repository or its remote.
git -C "${sanitized_repository}" update-ref refs/heads/main "${candidate_commit}"
git -C "${sanitized_repository}" filter-repo --force --invert-paths \
  --path .planning/ \
  --path docs/superpowers/ \
  --path docs/specs/

cp "${sanitized_repository}/filter-repo/commit-map" "${output_directory}/commit-map.tsv"
cp "${sanitized_repository}/filter-repo/ref-map" "${output_directory}/ref-map.tsv"
git -C "${sanitized_repository}" for-each-ref --format='%(refname)\t%(objectname)\t%(*objectname)' \
  > "${output_directory}/refs-after.tsv"
git -C "${sanitized_repository}" rev-list --objects --all > "${output_directory}/object-graph-after.txt"
git -C "${sanitized_repository}" fsck --full --strict > "${output_directory}/fsck.txt" 2>&1

mise exec -- python "${script_directory}/attest-publication-history.py" \
  --backup-repository "${backup_repository}" \
  --candidate-source "${source_repository}" \
  --candidate-commit "${candidate_commit}" \
  --sanitized-repository "${sanitized_repository}" \
  --archive-repository "${archive_repository}" \
  --output "${output_directory}/publication-attestation.json"

cat > "${output_directory}/CUTOVER-STATUS.txt" <<EOF
archive-prerequisite=${archive_commit}
source-head=${candidate_commit}
sanitized-head=$(git -C "${sanitized_repository}" rev-parse refs/heads/main)
remote-cutover-approved=false
public-visibility-approved=false
EOF

shasum -a 256 \
  "${output_directory}/refs-before.tsv" \
  "${output_directory}/refs-after.tsv" \
  "${output_directory}/commit-map.tsv" \
  "${output_directory}/ref-map.tsv" \
  "${output_directory}/removed-object-map.tsv" \
  "${output_directory}/object-graph-after.txt" \
  "${output_directory}/fsck.txt" \
  "${output_directory}/publication-attestation.json" \
  "${output_directory}/CUTOVER-STATUS.txt" \
  > "${output_directory}/SHA256SUMS"

find "${output_directory}" -type d -exec chmod 700 {} +
find "${output_directory}" -type f -exec chmod 600 {} +

echo "Publication-history rehearsal passed."
echo "Evidence: ${output_directory}"
echo "No remote was changed; cutover and visibility still require explicit approval."
