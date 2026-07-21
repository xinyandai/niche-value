# Contributing

Thanks for helping improve `niche-value`!

## One-time setup: git hooks

This repo ships shared git hooks in [`.githooks/`](.githooks). Enable them once
per clone:

```sh
git config core.hooksPath .githooks
```

(`core.hooksPath` is a local setting and is not shared automatically, so each
clone must run this once.)

### What the hooks do

- **pre-commit**
  - forbids direct commits to `main` / `master` (work on a feature branch);
  - runs `cargo fmt --all --check`;
  - runs `cargo clippy --all-features --all-targets -- -D warnings`.
- **pre-push**
  - runs `cargo test --all-features`.

Bypass in a pinch with `git commit --no-verify` / `git push --no-verify`.

## Workflow

`main` is protected. Make changes on a feature branch and open a pull request:

```sh
git switch -c my-change
# ... edit ...
git commit -m "..."      # pre-commit runs fmt + clippy
git push -u origin my-change   # pre-push runs the tests
gh pr create
```

## Checks run in CI

CI (`.github/workflows/ci.yml`) runs, and PRs must pass: tests (stable),
`build` on the MSRV (1.83), `rustfmt`, `clippy` (warnings denied), Miri, and
docs. Running the hooks locally keeps you ahead of CI.

## Manual commands

```sh
cargo fmt --all
cargo clippy --all-features --all-targets -- -D warnings
cargo test --all-features
cargo +nightly miri test --all-features   # requires: rustup +nightly component add miri
```
