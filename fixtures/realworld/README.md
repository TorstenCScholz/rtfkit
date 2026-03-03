# Realworld RTF Corpus

This folder contains long-form, intentionally messy fixtures that approximate modern enterprise reports.

## Covered document features

- page layout controls (`\paperw`, margins, portrait and landscape section switches)
- headers/footers with page fields
- hyperlinks (web and mailto via `\field`)
- embedded image destinations (`\pict`)
- mixed prose, nested lists, and tables
- footnotes and unsupported destinations to validate strict-mode behavior
- explicit page breaks to guarantee 10+ page-equivalent content

## Contracts

- non-strict conversion must succeed (`exit 0`)
- strict conversion behavior is defined per fixture metadata:
  - strict-pass fixtures use `expected_strict_exit: 0`
  - warning-probe fixtures use `expected_strict_exit: 4` with required dropped-content reasons

Each fixture has a sibling `*.meta.json` contract consumed by `crates/rtfkit-cli/tests/realworld_tests.rs`.

## Regeneration

Legacy shell-generated fixtures:

```bash
./scripts/generate_realworld_fixtures.sh
```

Platform modernization fixtures with deterministic synthetic charts:

```bash
python3 scripts/generate_platform_modernization_fixtures.py
```

Notes:

- The platform modernization generator requires `matplotlib` (offline deterministic chart generation).
- Optional flags:
  - `--seed`
  - `--out-dir`
  - `--pages-showcase`
  - `--pages-warning-probe`

After regeneration, run:

```bash
cargo test -p rtfkit --test realworld_tests
```
