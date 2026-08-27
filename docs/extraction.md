# History-preserving extraction

The source was the migration branch `wt/bead/issue/discogsography-2kpm.10` at
`e5b83fd5e56a2dfd00089b307fe6f2bd5904c245` in the unchanged monorepo
`/Users/Robert/workspaces/github/SimplicityGuy/discogsography`.

The reproducible extraction was performed in a disposable clone:

```bash
git clone --no-local --single-branch \
  --branch wt/bead/issue/discogsography-2kpm.10 \
  /Users/Robert/workspaces/github/SimplicityGuy/discogsography \
  catalog-ingestion
cd catalog-ingestion
git filter-repo --force \
  --path extractor/ \
  --path Cargo.lock \
  --path LICENSE \
  --path docs/state-marker-system.md \
  --path tests/extractor/test_dockerfile_uid.py \
  --path-rename extractor/: \
  --path-rename tests/extractor/:tests/repository/
git branch -M main
```

The filtered contract-boundary source commit is
`9d029e5eb5e4602ac9be5d9c6a087c4481dc24ff`; it corresponds to source commit
`3219ebf55e9bed9a66e4d54222139fa878e4268a`. The initial filtered history contains
292 commits and no tags. Migration scaffolding is added as one later commit.

The current tree is MIT licensed by owner decision. Historical revisions preserve their
then-applicable license text. The original monorepo and its refs were not rewritten or
deleted.

