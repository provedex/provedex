# Summary

What changes and why. Link the issue this closes (e.g. `Closes #42`) if there is one.

# Checklist

- [ ] `cargo fmt --all -- --check` clean
- [ ] `cargo clippy --workspace --all-targets --all-features -- -D warnings` clean
- [ ] `cargo test --workspace --all-features` passes (unit, integration, doctests)
- [ ] If a public API in `provedex-core` changed: rustdoc updated and at least one runnable doctest added or updated
- [ ] Commit message follows Conventional Commits (`feat:`, `fix:`, `chore:`, `docs:`, `refactor:`, `test:`, `ci:`, `perf:`, `build:`)
- [ ] No co-author trailer from an AI tool
- [ ] No new files outside the structure in `README.md` / `TECHNICAL_PLAN.md` (open a discussion first if you need to add one)
- [ ] Diff is one feature or fix; unrelated cleanups are split into separate PRs

# Notes for the reviewer

Anything non-obvious about the diff. Trade-offs, follow-ups, or test-coverage gaps.
