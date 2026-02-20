# Task: OSS RTF Tooling (CLI-first) – `rtfkit` (RTF → DOCX zuerst, später HTML/PDF)

## Kontext

Wir bauen ein Open-Source CLI-Tool zur **RTF-Konvertierung ohne LibreOffice/Word-Abhängigkeiten**.

Ziele:

* reproduzierbare, deterministische Konvertierungen
* gute Developer Experience (Binary-only, Debugbarkeit, stabile Tests)
* modular genug, um später weitere Targets (HTML, PDF) anzuhängen

Nicht Ziel ist vollständige RTF-Spec-Abdeckung, sondern:

> “90 % realer Dokumente korrekt, robust und reproduzierbar konvertieren.”

Das Tool soll langfristig Teil einer kleinen OSS-Tool-Suite werden (ähnlich wie bereits gestartete CLI-Utilities).

---

## High-Level Ziel

### v0.1 (MVP)

* Stabiler CLI-Converter: **RTF → DOCX**
* Klar definierter IR-Layer als Fundament
* Golden Tests + reproduzierbare Builds
* Plattformunabhängige Binaries

### v0.2+

* Ausbau IR Coverage:

  * Listen
  * Tabellen
  * Bilder (nur Inline, minimal)
* Optional zusätzliches Target:

  * RTF → HTML (selbe IR)

### v0.3+

* RTF → PDF ohne Office-Backends
  via Rendering-Backend (z. B. Typst)

Wichtig: Architektur so bauen, dass neue Targets **Writer-only** sind.

---

# Scope v0.1 (MVP)

## 1. CLI

### Kommando

```bash
rtfkit convert --to docx <input.rtf> -o <output.docx>
```

### Flags

| Flag                    | Zweck                                 |                                     |
| ----------------------- | ------------------------------------- | ----------------------------------- |
| `--format text          | json`                                 | Ausgabeformat für Conversion Report |
| `--emit-ir <file.json>` | Serialisiert IR zur Analyse/Debugging |                                     |
| `--strict`              | Fail bei unsupported Features         |                                     |

### Verhalten

* stdout: nur Report (kein Binärmüll)
* stderr: Fehler / Warnings (optional strukturiert)

### Exit Codes

| Code | Bedeutung                       |
| ---- | ------------------------------- |
| 0    | Erfolg                          |
| 2    | Parse-Fehler / invalid RTF      |
| 3    | Conversion-Fehler (Writer / IO) |
| 4    | Strict Mode verletzt            |

---

## 2. Parser + IR

### Ziel

RTF wird in ein internes, stabiles **Intermediate Representation (IR)** überführt, das:

* deterministisch serialisierbar ist
* unabhängig vom Output-Format funktioniert
* leicht testbar ist

---

### IR-Struktur (v0.1)

```rust
Document {
  blocks: Vec<Block>
}

enum Block {
  Paragraph(Paragraph)
}

Paragraph {
  alignment: Alignment,
  runs: Vec<Run>
}

Run {
  text: String,
  bold: bool,
  italic: bool,
  underline: bool,
  font_size: Option<f32>,
  color: Option<Color>
}
```

### Unterstützte Features (MVP)

* Plain Text
* Absätze:

  * `\par`
  * `\line`
* Inline Styles:

  * bold (`\b`)
  * italic (`\i`)
  * underline (`\ul`)
* Paragraph alignment:

  * left / center / right
  * justify optional
* Unicode Handling:

  * escaped chars
  * Sonderzeichen müssen stabil im IR landen

---

### Parser-Strategie

**Stateful Interpreter statt “RTF AST fidelity”**

* Tokenizer / Parser-Lib liefert Events
* Interpreter baut IR via:

  * Style State Machine
  * Group Stack (`{}` Push/Pop)
  * Current Style Snapshot

Ziel: robuste, verständliche Logik statt Spec-Komplettheit.

---

### Reporting Layer

Parser erzeugt strukturierte Reports:

```rust
Report {
  warnings: Vec<Warning>,
  stats: Stats
}
```

**Warnings Beispiele**

* unsupported control word
* unknown destination
* dropped content

**Stats Beispiele**

* paragraph count
* run count
* bytes processed
* duration

Wichtig für:

* Strict Mode
* CLI JSON Output
* Debugging

---

## 3. DOCX Writer

### Ziel

IR → valides `.docx`, das:

* in Word und LibreOffice öffnet
* Text + Basisformatierung korrekt darstellt
* keine externen Abhängigkeiten benötigt

### Anforderungen

* Nutzung bestehender Rust-Library (Evaluation Teil der Task)
* Mapping:

| IR        | DOCX   |
| --------- | ------ |
| Paragraph | `w:p`  |
| Run       | `w:r`  |
| bold      | `w:b`  |
| italic    | `w:i`  |
| underline | `w:u`  |
| alignment | `w:jc` |

Fonts/Fidelity:

* Default Font reicht
* Keine Font-Matching-Logik nötig

---

## 4. Fixtures + Golden Tests

### Fixtures

Ordner:

```
fixtures/
  simple_paragraph.rtf
  bold_italic.rtf
  unicode.rtf
  alignment.rtf
  ...
```

Mindestens 10 realistische Beispiele.

Quelle:

* anonymisierte echte RTFs
* selbst erzeugte Edgecases

---

### Golden Tests Strategie

Pipeline:

1. Convert RTF → DOCX
2. `.docx` unzippen
3. `word/document.xml` parsen
4. Assertions:

* Text vorhanden
* erwartete Tags existieren

Beispiele:

* Bold → `w:b`
* Italic → `w:i`
* Alignment center → `w:jc w:val="center"`

---

## 5. CI & Releases

Übernahme aus Template-Repo:

### CI

* Linux
* macOS
* Windows

Checks:

* fmt
* clippy
* tests
* build

---

### Releases

Trigger:

* Git Tag `vX.Y.Z`

Artefakte:

* statische Binaries (3 OS)
* optional Docker Image

Optional später:

* Homebrew Tap
* scoop/choco

---

# Non-Goals v0.1

Bewusst ausgeschlossen:

* Pixel-perfekte Word-Fidelity
* OLE / Embeddings
* Header/Footer
* Footnotes
* Track Changes
* Vollständige RTF Spec

Das verhindert Scope-Creep.

---

# Repository / Architektur

Angelehnt an dein Template-Repo.

## Workspace Layout

```
crates/
  rtfkit-core/
  rtfkit-cli/
```

Optional später:

```
rtfkit-render-typst/
```

---

## rtfkit-core

Verantwortung:

* IR Types
* Parser Integration
* Interpreter
* Report/Warnungsmodell
* IR Normalization

Keine:

* CLI
* File IO

---

## rtfkit-cli

Verantwortung:

* clap CLI
* IO
* Report Rendering
* Exit Codes
* Strict Mode Enforcement

---

## Architekturprinzipien

* Deterministische JSON-Ausgabe (stable ordering)
* Keine Panics nach außen
* Typed Errors + Result-Flows
* Reproduzierbare Builds

Passt gut zu deinem generellen OSS-Template-Ansatz.

---

# Phasenplan (mit klaren Deliverables)

## Phase 0 — Bootstrap & “Hello Convert”

### Ziel

Projekt steht, Pipeline existiert, auch wenn noch kein echter Parser.

---

### Deliverables

* Repo aus Template erstellt (`rtfkit`)
* Workspace-Struktur steht
* CLI Skeleton:

```bash
rtfkit convert input.rtf -o out.docx
```

* Dummy Verhalten:

  * liest Datei
  * erzeugt klare Fehlermeldung ODER placeholder DOCX

---

### Akzeptanz

* CI grün auf 3 OS
* `--help` funktioniert
* `--version` liefert Version

---

## Phase 1 — RTF → IR

### Hauptziel

Parser + Interpreter stabil.

DOCX noch optional.

---

### Aufgaben

#### 1. Parser Spike

Evaluieren und entscheiden:

* vorhandene RTF Parser Crates
* License
* Unicode Verhalten
* Maintenance

Entscheidung dokumentieren (ADR).

---

#### 2. Interpreter

* Group Stack Implementierung

* Style State Tracking

* Control Words:

  * `\b`
  * `\i`
  * `\ul`
  * `\par`
  * `\line`
  * `\ql/\qc/\qr`

* Text Aggregation

---

#### 3. Reporting

* Warning Types definieren
* Stats sammeln

---

### Zwischenergebnis

```bash
rtfkit convert input.rtf --emit-ir out.json
```

liefert IR JSON.

DOCX darf noch fehlen.

---

### Akzeptanz

* Fixtures → IR Snapshots stabil
* Unicode korrekt im IR
* IR deterministisch serialisiert

---

## Phase 2 — IR → DOCX (MVP Writer)

### Ziel

End-to-End v0.1 Feature komplett.

---

### Aufgaben

#### 1. DOCX Lib Spike

2 Kandidaten evaluieren, Entscheidung dokumentieren.

Kriterien:

* Aktiv gepflegt
* Einfaches Paragraph/Run API
* Keine heavy deps

---

#### 2. Writer Implementierung

* Document Root
* Paragraph Mapping
* Run Mapping
* Alignment Mapping

---

#### 3. Strict Mode

Wenn:

* Warnings vom Typ “DroppedContent”
  → Exit Code 4

---

### Zwischenergebnis

Funktionierender Converter für:

* Plain Text
* Basic Formatting

---

### Akzeptanz

* Alle Fixtures öffnen in:

  * Word
  * LibreOffice
* Golden Tests prüfen XML Tags
* Strict Mode funktioniert deterministisch
