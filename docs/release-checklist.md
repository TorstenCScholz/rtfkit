# Release Checklist for rtfkit

This document provides a comprehensive checklist for releasing rtfkit. Follow these steps to ensure a smooth, reliable release process.

## Table of Contents

1. [Pre-Release Verification](#pre-release-verification)
2. [Version Bump Checklist](#version-bump-checklist)
3. [Release Process](#release-process)
4. [Post-Release Verification](#post-release-verification)
5. [Artifact Verification](#artifact-verification)
6. [Troubleshooting](#troubleshooting)

---

## Pre-Release Verification

### 1. Code Quality Checks

- [ ] **All tests pass locally**
  ```bash
  cargo test --all --all-features
  ```

- [ ] **No clippy warnings**
  ```bash
  cargo clippy --all-targets --all-features -- -D warnings
  ```

- [ ] **Code is properly formatted**
  ```bash
  cargo fmt --all -- --check
  ```

- [ ] **Documentation builds without warnings**
  ```bash
  cargo doc --no-deps
  ```

### 2. CI Pipeline Verification

- [ ] **All CI jobs pass on main/master branch**
  - Check GitHub Actions: https://github.com/TorstenCScholz/rtfkit/actions
  - Verify all matrix jobs (Ubuntu, macOS, Windows) pass
  - Verify smoke tests pass

- [ ] **No pending or failed PRs that should be included**

### 3. Integration Testing

- [ ] **Run smoke tests locally on built binary**
  ```bash
  # Build release binary
  cargo build --release -p rtfkit
  
  # Run smoke tests (Unix)
  ./scripts/smoke_test.sh ./target/release/rtfkit
  
  # Run smoke tests (Windows PowerShell)
  ./scripts/smoke_test.ps1 -BinaryPath ./target/release/rtfkit.exe
  ```

- [ ] **Test HTML output**
  ```bash
  # Test HTML conversion
  ./target/release/rtfkit convert fixtures/text_simple_paragraph.rtf --to html -o test.html
  
  # Verify HTML is well-formed
  cat test.html  # Should show valid HTML5
  
  # Test HTML with tables
  ./target/release/rtfkit convert fixtures/table_simple_2x2.rtf --to html -o table.html
  ```

- [ ] **Test with real-world RTF files**
  - Test with complex documents containing tables
  - Test with documents containing lists
  - Test with documents containing Unicode content
  - Test with malformed RTF files (error handling)
  - Test both DOCX and HTML output formats

### 4. Dependency Audit

- [ ] **Check for security vulnerabilities**
  ```bash
  cargo audit
  ```

- [ ] **Review dependency updates**
  ```bash
  cargo outdated
  ```

---

## Version Bump Checklist

### 1. Update Version Numbers

- [ ] **Update workspace version in `Cargo.toml`**
  ```toml
  [workspace.package]
  version = "X.Y.Z"  # Update this
  ```

- [ ] **Verify version propagates to all crates**
  - Check `crates/rtfkit-cli/Cargo.toml` uses `version.workspace = true`
  - Check `crates/rtfkit-core/Cargo.toml` uses `version.workspace = true`
  - Check `crates/rtfkit-docx/Cargo.toml` uses `version.workspace = true`

### 2. Update Changelog

- [ ] **Add release notes to `CHANGELOG.md`**
  
  Follow the format:
  ```markdown
  ## [X.Y.Z] - YYYY-MM-DD

  ### Added
  - New feature descriptions

  ### Changed
  - Changes to existing functionality

  ### Fixed
  - Bug fixes

  ### Removed
  - Deprecated features removed
  ```

- [ ] **Ensure all significant changes are documented**
- [ ] **Include migration notes if breaking changes exist**

### 3. Update Documentation

- [ ] **Update README.md if necessary**
  - New features
  - Changed CLI arguments
  - Updated examples

- [ ] **Update any outdated documentation**

---

## Release Process

### 1. Final Preparation

- [ ] **Create a release branch**
  ```bash
  git checkout -b release/vX.Y.Z
  ```

- [ ] **Commit version bump changes**
  ```bash
  git add -A
  git commit -m "chore: bump version to X.Y.Z"
  ```

- [ ] **Push branch and create PR**
  ```bash
  git push origin release/vX.Y.Z
  ```
  - Create PR targeting main/master
  - Wait for CI to pass

### 2. Merge and Tag

- [ ] **Merge PR to main/master**
  ```bash
  git checkout main
  git pull origin main
  git merge --no-ff release/vX.Y.Z
  ```

- [ ] **Create annotated tag**
  ```bash
  git tag -a vX.Y.Z -m "Release vX.Y.Z

  Summary of changes:
  - Change 1
  - Change 2
  ..."
  ```

- [ ] **Push tag to trigger release workflow**
  ```bash
  git push origin main
  git push origin vX.Y.Z
  ```

### 3. Monitor Release

- [ ] **Monitor GitHub Actions release workflow**
  - URL: https://github.com/TorstenCScholz/rtfkit/actions/workflows/release.yml
  - Verify all build jobs complete
  - Verify smoke tests pass
  - Verify release is created

---

## Post-Release Verification

### 1. Verify GitHub Release

- [ ] **Check release page**
  - URL: https://github.com/TorstenCScholz/rtfkit/releases/tag/vX.Y.Z
  - Verify all artifacts are attached
  - Verify release notes are correct
  - Verify checksums are present

### 2. Verify Artifacts

- [ ] **Download and verify each artifact**

  See [Artifact Verification](#artifact-verification) below.

### 3. Announce Release

- [ ] **Update repository description if needed**
- [ ] **Announce on relevant channels**
  - GitHub Discussions
  - Social media (if applicable)
  - Project website (if applicable)

---

## Artifact Verification

### Expected Artifacts

For each release, the following artifacts should be present:

| Platform | Architecture | Artifact Name |
|----------|-------------|---------------|
| Linux | x86_64 | `rtfkit-x86_64-unknown-linux-gnu.tar.gz` |
| Linux | ARM64 | `rtfkit-aarch64-unknown-linux-gnu.tar.gz` |
| macOS | Intel | `rtfkit-x86_64-apple-darwin.tar.gz` |
| macOS | Apple Silicon | `rtfkit-aarch64-apple-darwin.tar.gz` |
| Windows | x86_64 | `rtfkit-x86_64-pc-windows-msvc.zip` |

### Verification Steps

For each artifact:

1. **Download the artifact**
   ```bash
   # Example for Linux x86_64
   curl -LO https://github.com/TorstenCScholz/rtfkit/releases/download/vX.Y.Z/rtfkit-x86_64-unknown-linux-gnu.tar.gz
   ```

2. **Verify checksum**
   ```bash
   # Download checksum
   curl -LO https://github.com/TorstenCScholz/rtfkit/releases/download/vX.Y.Z/rtfkit-x86_64-unknown-linux-gnu.tar.gz.sha256
   
   # Verify (Linux/macOS)
   sha256sum -c rtfkit-x86_64-unknown-linux-gnu.tar.gz.sha256
   
   # Or verify against SHA256SUMS
   curl -LO https://github.com/TorstenCScholz/rtfkit/releases/download/vX.Y.Z/SHA256SUMS
   sha256sum -c SHA256SUMS
   ```

3. **Extract and test binary**
   ```bash
   # Extract (Unix)
   tar xzf rtfkit-x86_64-unknown-linux-gnu.tar.gz
   
   # Test version
   ./rtfkit --version
   
   # Run smoke test
   ./scripts/smoke_test.sh ./rtfkit
   ```

4. **Test basic conversion**
   ```bash
   # Create test RTF
   echo '{\rtf1\ansi Hello World}' > test.rtf
   
   # Convert to DOCX
   ./rtfkit convert test.rtf --output test.docx
   
   # Verify output
   unzip -l test.docx  # Should show word/document.xml
   
   # Convert to HTML
   ./rtfkit convert test.rtf --to html --output test.html
   
   # Verify HTML output
   cat test.html  # Should show valid HTML5 with Hello World
   ```

### HTML-Specific Verification

For HTML output testing:

1. **Test HTML output determinism**
   ```bash
   # Run conversion twice and compare
   ./rtfkit convert fixtures/text_simple_paragraph.rtf --to html -o test1.html
   ./rtfkit convert fixtures/text_simple_paragraph.rtf --to html -o test2.html
   diff test1.html test2.html  # Should be identical
   ```

2. **Test HTML snapshot tests**
   ```bash
   # Update HTML golden snapshots if needed
   UPDATE_GOLDEN=1 cargo test -p rtfkit --test golden_tests -- html
   
   # Verify HTML snapshots are valid
   for f in golden_html/*.html; do
     # Basic well-formedness check
     grep -q "<!doctype html>" "$f" && echo "$f: OK"
   done
   ```

3. **Test HTML with complex content**
   ```bash
   # Test tables with merges
   ./rtfkit convert fixtures/table_mixed_merge.rtf --to html -o merge.html
   
   # Test nested lists
   ./rtfkit convert fixtures/list_nested_two_levels.rtf --to html -o nested.html
   
   # Verify structure
   grep -q "<table" merge.html
   grep -q "<ul>" nested.html
   ```

### Windows-Specific Verification

```powershell
# Download artifact
Invoke-WebRequest -Uri "https://github.com/TorstenCScholz/rtfkit/releases/download/vX.Y.Z/rtfkit-x86_64-pc-windows-msvc.zip" -OutFile "rtfkit.zip"

# Verify checksum
$hash = (Get-FileHash -Algorithm SHA256 "rtfkit.zip").Hash
# Compare with expected hash from .sha256 file

# Extract
Expand-Archive -Path "rtfkit.zip" -DestinationPath "."

# Test
.\rtfkit.exe --version

# Run smoke test
.\scripts\smoke_test.ps1 -BinaryPath .\rtfkit.exe
```

---

## Troubleshooting

### Release Workflow Failed

1. **Check workflow logs**
   - Go to Actions tab in GitHub
   - Find the failed workflow run
   - Examine the failed step's logs

2. **Common issues**
   - **Build failure**: Check for platform-specific code issues
   - **Test failure**: Run tests locally on the failing platform
   - **Artifact upload failure**: Check artifact size limits

### Missing Artifacts

1. **Verify build matrix completed**
   - Check all matrix jobs in the workflow
   - Some platforms may have failed silently

2. **Re-run failed jobs**
   - Use GitHub Actions "Re-run jobs" feature

### Checksum Mismatch

1. **Verify download integrity**
   - Re-download the artifact
   - Check for proxy/CDN issues

2. **Report issue**
   - If checksums don't match after re-download, this is a critical issue
   - Delete the release and re-run the workflow

### Binary Doesn't Run

1. **Check architecture**
   - Ensure you downloaded the correct artifact for your platform
   - `uname -m` on Unix shows your architecture

2. **Check permissions (Unix)**
   ```bash
   chmod +x rtfkit
   ```

3. **Check dependencies (Linux)**
   ```bash
   ldd rtfkit  # Check for missing shared libraries
   ```

---

## Quick Reference

### Version Numbering

Follow [Semantic Versioning](https://semver.org/):

- **MAJOR**: Breaking changes
- **MINOR**: New features, backward compatible
- **PATCH**: Bug fixes, backward compatible

Pre-release versions:
- `v1.0.0-alpha.1` - Alpha release (internal testing)
- `v1.0.0-beta.1` - Beta release (external testing)
- `v1.0.0-rc.1` - Release candidate (final testing)

### Release Types

| Type | Tag Format | Prerelease? | Target Audience |
|------|------------|-------------|-----------------|
| Alpha | `vX.Y.Z-alpha.N` | Yes | Internal testing |
| Beta | `vX.Y.Z-beta.N` | Yes | External testers |
| RC | `vX.Y.Z-rc.N` | Yes | Final validation |
| Stable | `vX.Y.Z` | No | General public |

### Useful Commands

```bash
# Check current version
cargo metadata --no-deps --format-version 1 | jq -r '.packages[0].version'

# List all tags
git tag -l

# Delete local tag
git tag -d vX.Y.Z

# Delete remote tag
git push origin --delete vX.Y.Z

# List recent releases (GitHub CLI)
gh release list --limit 10

# Create release manually (GitHub CLI)
gh release create vX.Y.Z --title "vX.Y.Z" --notes-file RELEASE_NOTES.md ./artifacts/*
```

---

## Checklist Summary

Copy this for each release:

```
## Release vX.Y.Z Checklist

### Pre-Release
- [ ] All tests pass
- [ ] No clippy warnings
- [ ] Code formatted
- [ ] CI passes on main
- [ ] Smoke tests pass locally
- [ ] HTML output tests pass
- [ ] Security audit clean

### Version Bump
- [ ] Version updated in Cargo.toml
- [ ] CHANGELOG.md updated
- [ ] Documentation updated

### Release
- [ ] Release branch created
- [ ] PR merged to main
- [ ] Tag created and pushed
- [ ] Release workflow completed

### Post-Release
- [ ] GitHub release verified
- [ ] Artifacts downloaded and tested
- [ ] Checksums verified
- [ ] HTML output verified on all platforms
```
