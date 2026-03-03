# AGENTS.md

Guidance for automated coding agents working in this repository.

## Scope

- Repository: `rtfkit` (Rust workspace)
- Primary languages: Rust and Python bindings
- Primary deliverables: RTF parsing/conversion to DOCX, HTML, PDF

## First Steps

1. Read `README.md` for user-facing behavior and CLI contract.
2. Read `CONTRIBUTING.md` for test and fixture workflow.
3. Prefer minimal, targeted changes over broad refactors.

## Repository Map

- `crates/rtfkit-core`: parser, IR, limits, warnings
- `crates/rtfkit-cli`: `rtfkit` binary and integration tests
- `crates/rtfkit-docx`: DOCX writer
- `crates/rtfkit-html`: HTML writer
- `crates/rtfkit-render-typst`: PDF rendering
- `crates/rtfkit-style-tokens`: style profile tokens
- `bindings/python`: Python package and tests
- `fixtures/`: `.rtf` fixtures
- `golden/`: IR snapshot outputs (`.json`)
- `golden_html/`: HTML snapshot outputs (`.html`)
- `docs/`: architecture, specs, policies, references

## Build and Test Commands

Run from repository root unless noted.

```sh
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
cargo test --all
```

Useful targeted commands:

```sh
cargo test --test cli_contract_tests
cargo test --test limits_tests
cargo test --test determinism_tests
```

Python bindings (from `bindings/python`):

```sh
pip install -e .
pytest
```

## Fixture-First Workflow (Preferred)

For parser/conversion behavior changes:

1. Add or update an `.rtf` fixture in `fixtures/`.
2. Add or update tests that fail before the fix.
3. Update snapshots when intended:

```sh
UPDATE_GOLDEN=1 cargo test -p rtfkit --test golden_tests
```

4. Review generated files in `golden/` (and `golden_html/` when relevant).
5. Ensure non-updated tests continue to pass.

Do not hand-edit snapshot files unless there is a strong reason.

## Safety and Limits

When touching limits or fail-closed behavior:

- Update definitions in `crates/rtfkit-core/src/limits.rs` and enforcement points.
- Add near-limit and over-limit tests.
- Preserve fail-closed semantics (no partial-success behavior after hard failures).
- Update related documentation in `docs/limits-policy.md` and warning docs.

## Change Discipline

- Keep public behavior stable unless intentionally changing contract.
- Avoid renaming files/modules without a clear need.
- Update docs for user-visible changes (`README.md`, `CHANGELOG.md`, docs references).
- Prefer existing patterns over introducing new abstractions for small changes.

## Commit/PR Readiness Checklist

- Formatting passes.
- Clippy has no warnings.
- Tests pass for affected areas (or full suite for broad changes).
- Fixtures/snapshots updated and reviewed when behavior changes.
- Docs/changelog updated when user-visible behavior changes.

## Agent Output Expectations

When reporting work:

- State what changed and why.
- List exact commands run.
- Note any skipped checks and why.
- Call out follow-up risks (performance, compatibility, determinism) if applicable.
