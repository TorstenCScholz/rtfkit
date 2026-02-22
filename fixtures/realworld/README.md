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
- strict conversion must fail (`exit 4`) because unsupported destination content is dropped

Each fixture has a sibling `*.meta.json` contract consumed by `crates/rtfkit-cli/tests/realworld_tests.rs`.

## Regeneration

Fixtures are deterministic and can be regenerated with:

```bash
./scripts/generate_realworld_fixtures.sh
```

After regeneration, run:

```bash
cargo test -p rtfkit --test realworld_tests
```
