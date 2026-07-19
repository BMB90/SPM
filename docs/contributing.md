# Contributing

## Workflow

1. Branch from `main`.
2. Keep commits scoped to one logical change; `cargo fmt` and
   `cargo clippy --workspace` should be clean before committing Rust
   changes, `npx tsc -b` clean before committing frontend changes.
3. Add or update tests for behavior you change — see
   `docs/developer-guide.md`'s Testing section for where unit vs.
   integration tests live.
4. Update the relevant doc under `docs/` in the same change (schema
   changes → `database-schema.md`, new endpoints → `api.md`, new
   collectors → `collector-architecture.md`).
5. Open a PR with a description of *why*, not just *what* — the diff
   already shows what changed.

## Commit messages

Conventional-commit-style prefixes are appreciated but not enforced:
`feat:`, `fix:`, `docs:`, `refactor:`, `test:`, `chore:`. Subject line
≤ 72 chars; body explains motivation/tradeoffs when they're not obvious
from the diff.

## Code review expectations

- No `unsafe` without a `// SAFETY:` comment explaining the invariant
  being upheld (collectors doing WinAPI/COM FFI are the main place this
  applies — see `spm-collector-windows/src/signature.rs` and `etw.rs`).
- No silently-swallowed errors in collectors beyond the documented
  fail-soft pattern (log + skip, never fabricate a value).
- New REST endpoints follow the existing pagination
  (`?limit=&offset=` → `{items, total, limit, offset}`) and error shape
  (`{"error": "..."}`) conventions in `spm-api/src/routes.rs`.

## Adding a frontend test runner

None is configured yet. If you're adding one, Vitest is the natural fit
(same Vite config, minimal setup) — add it as a devDependency, a
`vitest.config.ts` (or extend `vite.config.ts`), and a `test` script in
`package.json`; document the choice here once it lands.

## Reporting bugs / proposing features

Open an issue describing: what you expected, what happened, your OS
(and whether you were running elevated — several collectors behave
differently), and the output of `cargo run -p spm-cli -- capture
--no-enrich` if the issue is capture-related (the `--no-enrich` flag
skips hashing/signature checks, so it's a faster repro loop).

## License

By contributing, you agree your contribution is licensed under this
project's MIT license (see `LICENSE`).
