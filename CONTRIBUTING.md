# Contributing to Zun

Thanks for contributing to Zun.

## Development Setup

```bash
git clone <repo-url>
cd Zun
cargo build
```

Run the baseline checks before opening a PR:

```bash
cargo fmt --check
cargo clippy --workspace --all-targets --all-features
cargo test --workspace
```

## Repository Layout

- `src/`: current interpreter implementation
- `crates/zun-lsp/`: future LSP crate
- `docs/`: guides and language references
- `tests/fixtures/`: executable test inputs

## Change Expectations

- Keep changes scoped to one problem.
- Add or update tests for behavior changes.
- Update docs when CLI behavior, language behavior, or contributor workflow changes.
- Avoid mixing refactors and feature additions in the same PR.
- If a change may break compatibility, include an RFC or a compatibility note.
- If a change adds or expands side effects, update `docs/spec/security-model.md`.

## Commit and PR Guidance

Preferred commit prefixes:

- `feat:`
- `fix:`
- `refactor:`
- `docs:`
- `test:`
- `chore:`

PRs should include:

- what changed
- why it changed
- how it was tested
- any compatibility or follow-up notes
- any deprecation plan if user-facing behavior is being replaced

## Review Checklist

Before requesting review, confirm:

- formatting passes
- clippy runs cleanly enough to review signal
- tests pass
- new behavior has documentation or examples when needed
- compatibility and changelog notes are updated when user-facing behavior changes
