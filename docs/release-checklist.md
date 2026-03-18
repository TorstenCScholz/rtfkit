# Release Checklist for rtfkit

This checklist is for publishing stable `rtfkit` releases and keeping the public release documentation aligned with the actual shipped feature set.

## Pre-release verification

### Documentation gate

Before tagging a release, verify:

- `README.md` describes the current public product story and usage examples
- `CHANGELOG.md` contains a release entry for the version being published
- `docs/feature-support.md` reflects the current supported feature set
- `docs/rtf-feature-overview.md` reflects the current support summary and limitations
- `docs/warning-reference.md` covers every `Warning` enum variant (`bash scripts/check_warning_docs.sh`)
- `docs/reference/pdf-output.md` and `docs/feature-support.md` stay consistent on PDF claims
- any new guaranteed behavior is backed by contract tests

If a change affects feature support, warning semantics, strict mode, or public CLI behavior, update the documentation before releasing.

### Code quality checks

- **All tests pass locally**
  ```bash
  cargo test --all --all-features
  ```
- **No clippy warnings**
  ```bash
  cargo clippy --all-targets --all-features -- -D warnings
  ```
- **Code is formatted**
  ```bash
  cargo fmt --all -- --check
  ```
- **Docs build cleanly**
  ```bash
  cargo doc --no-deps
  ```

### CI verification

- Confirm the main branch CI is green
- Confirm there are no release-blocking fixes waiting to land

### Integration checks

- **Build the release binary**
  ```bash
  cargo build --release -p rtfkit
  ```
- **Run smoke tests**
  ```bash
  ./scripts/smoke_test.sh ./target/release/rtfkit
  ```
- **Validate representative conversions**
  ```bash
  ./target/release/rtfkit convert fixtures/text_simple_paragraph.rtf -o test.docx
  ./target/release/rtfkit convert fixtures/text_simple_paragraph.rtf --to html -o test.html
  ./target/release/rtfkit convert fixtures/text_simple_paragraph.rtf --to pdf -o test.pdf
  ```
- **Exercise real-world fixtures**
  - tables
  - lists
  - Unicode text
  - malformed RTF
  - images

### Dependency review

- **Security audit**
  ```bash
  cargo audit
  ```
- **Dependency review**
  ```bash
  cargo outdated
  ```

## Version and release notes

- Update workspace and binding version numbers
- Ensure `CHANGELOG.md` contains the new release section
- Ensure `README.md` and support docs match the shipped feature set
- Make sure release notes describe the release in user-facing terms rather than internal implementation sequencing

## Release process

### Final preparation

- Create a release branch if desired
- Commit version and documentation updates
- Open and merge the release PR after CI passes

### Tag and publish

- Create an annotated tag:
  ```bash
  git tag -a vX.Y.Z -m "Release vX.Y.Z"
  ```
- Push the tag:
  ```bash
  git push origin vX.Y.Z
  ```
- Monitor `.github/workflows/release.yml`

## Post-release verification

- Verify the GitHub release page exists and the notes look correct
- Verify binaries and Python artifacts are attached
- Verify checksums are present
- Download and smoke-test at least one artifact
- Confirm the public docs still match the released behavior

## Expected artifacts

Stable releases should include:

| Platform | Architecture | Artifact |
|----------|--------------|----------|
| Linux | x86_64 | `rtfkit-x86_64-unknown-linux-gnu.tar.gz` |
| Linux | ARM64 | `rtfkit-aarch64-unknown-linux-gnu.tar.gz` |
| macOS | Intel | `rtfkit-x86_64-apple-darwin.tar.gz` |
| macOS | Apple Silicon | `rtfkit-aarch64-apple-darwin.tar.gz` |
| Windows | x86_64 | `rtfkit-x86_64-pc-windows-msvc.zip` |
| Python | wheel/sdist | artifacts from `bindings/python` |

## Artifact verification

For each downloaded artifact:

1. Verify the checksum
2. Confirm the binary starts and prints its version
3. Run a smoke test
4. Convert at least one simple RTF file and one representative richer fixture

Example:

```bash
curl -LO https://github.com/TorstenCScholz/rtfkit/releases/download/vX.Y.Z/rtfkit-x86_64-unknown-linux-gnu.tar.gz
curl -LO https://github.com/TorstenCScholz/rtfkit/releases/download/vX.Y.Z/SHA256SUMS
sha256sum -c SHA256SUMS
tar xzf rtfkit-x86_64-unknown-linux-gnu.tar.gz
./rtfkit --version
./scripts/smoke_test.sh ./rtfkit
```

## Quick summary

Use this as a final gate:

- tests pass
- lint passes
- formatting passes
- docs are current
- changelog is current
- release artifacts build
- smoke tests pass
- release notes are public-facing and feature-focused
