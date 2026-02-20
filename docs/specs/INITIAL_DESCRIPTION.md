## Task: OSS RTF Tooling (CLI-first) – `rtfkit` (RTF → DOCX zuerst, später HTML/PDF)

### Kontext

Wir wollen ein Open-Source Tool bauen, das **RTF ohne LibreOffice/Word** konvertiert. Fokus ist **“90% Usecases gut genug”** und **gute DX** (ein Binary, reproduzierbare Outputs, fixtures/golden tests, Releases für alle OS).

### High-Level Ziel

* **v0.1:** `RTF → DOCX` (MVP subset) als CLI, stabil & testbar
* **v0.2+:** Ausbau (Listen, Tabellen, Bilder), optional `RTF → HTML`
* **v0.3+:** `RTF → PDF` über ein Rendering-Backend (z.B. Typst) – ohne LibreOffice

---

## Scope v0.1 (MVP)

### Muss

1. **CLI**

* `rtfkit convert --to docx <input.rtf> -o <output.docx>`
* Flags:

  * `--format text|json` (für Report)
  * `--emit-ir <path>.json` (Debug / Dev-Support)
  * `--strict` (unsupported features → exit code != 0)
* Exit Codes:

  * `0` ok
  * `2` invalid input (parse)
  * `3` conversion failure (writer / io)
  * `4` strict failed (unsupported features encountered)

2. **Parser + IR**

* RTF wird in ein eigenes **IR (Intermediate Representation)** gemappt:

  * `Document { blocks: Vec<Block> }`
  * `Block::Paragraph(Paragraph)`
  * `Paragraph { alignment, runs: Vec<Run> }`
  * `Run { text, bold, italic, underline, font_size?, color? }`
* Parser-Strategie: “Style State Machine” mit **Group-Stack** (`{}`) und Current Style State
* Minimal unterstützen:

  * Text, Absätze (`\par`, `\line`)
  * Bold/Italic/Underline
  * Paragraph alignment (left/center/right; justify optional)
  * Unicode / Escapes so, dass Sonderzeichen nicht kaputt gehen

3. **DOCX Writer**

* IR → DOCX mit bestehender Rust-Lib (Evaluation + Entscheidung Teil der Task)
* Output muss in Word/LibreOffice öffnen und lesbar sein.

4. **Fixtures + Golden Tests**

* `fixtures/` mit mind. 10 RTFs (realistisch, anonymisiert)
* Golden Tests:

  * Convert → `.docx`
  * unzip `.docx` → parse `word/document.xml`
  * Assertions: Text vorhanden + expected tags (`w:b`, `w:i`, `w:u`, `w:jc`)

5. **CI & Releases**

* Template-Repo Konventionen übernehmen:

  * fmt/clippy/test auf Linux/macOS/Windows
  * Release pipeline via Tag `vX.Y.Z` (Binaries + optional docker)

---

## Non-Goals v0.1

* Pixel-perfekte Word-Fidelity
* OLE Objects, Embedded Files
* Header/Footer, Footnotes, Track Changes
* Vollständige RTF Spec Abdeckung

---

## Repository / Architektur (an Template angelehnt)

**Workspace**

* `crates/rtfkit-core/`

  * parser adapter → IR builder
  * IR types + normalizer
  * report/warnings model
* `crates/rtfkit-cli/`

  * clap CLI, file IO, output formatting, exit codes
* (optional später) `crates/rtfkit-render-typst/` (PDF backend)

**Konventionen**

* Deterministische JSON Ausgabe (stable ordering)
* Kein Panic nach außen (alles in typed errors + report)

---

## Phasenplan (mit konkreten Zwischenergebnissen)

### Phase 0 — Bootstrap & “Hello Convert”

**Ergebnis**

* Repo aus Template kopiert + umbenannt (`rtfkit`)
* CLI skeleton steht: `rtfkit convert …`
* Dummy-Pipeline: liest RTF als bytes, schreibt minimale DOCX “placeholder” oder gibt klare Fehlermeldung aus

**Akzeptanz**

* CI grün auf 3 OS
* `rtfkit --help` / `--version` ok

---

### Phase 1 — RTF → IR (Parser/Interpreter)

**Aufgaben**

* Parser-Lib auswählen/integrieren (Spike, Entscheidung dokumentieren):

  * Kriterien: Unicode/escapes robust, License ok, aktive Maintenance
* Interpreter implementieren:

  * Group stack push/pop
  * Control words: `\b`, `\i`, `\ul`, `\par`, `\line`, `\qc/\ql/\qr` etc.
  * Text accumulation
* Report system:

  * `warnings[]`: unsupported control words, dropped destinations, etc.
  * `stats`: runs/paragraphs count, time, bytes

**Zwischenergebnis**

* `rtfkit convert --emit-ir out.json` liefert IR JSON (auch wenn DOCX noch nicht fertig)

**Akzeptanz**

* Fixtures → IR golden tests (JSON snapshots) stabil
* Sonderzeichen in Fixtures kommen korrekt im IR an

---

### Phase 2 — IR → DOCX (MVP Writer)

**Aufgaben**

* DOCX Lib auswählen (Spike: 2 Kandidaten, dann committen)
* Implementierung:

  * Document root, paragraphs, runs
  * Mapping bold/italic/underline
  * Paragraph alignment mapping
* Minimal Styling: default font ok (keine perfekte Font-Fidelity nötig)

**Zwischenergebnis**

* v0.1 kann “Text + basic formatting” nach DOCX konvertieren

**Akzeptanz**

* Alle v0.1 fixtures öffnen in Word/LibreOffice
* Golden tests prüfen `document.xml` auf expected formatting tags
* `--strict` failt, wenn warnings vom Typ “DroppedContent” auftreten

---

### Phase 3 — Listen (v0.2)

**Aufgaben**

* RTF list patterns erkennen (häufig: `\pn…`, `\listtable`, `\ls…`)
* Erstes Ziel: “good-enough” bullets/numbering

  * Option A: echte DOCX numbering definitions
  * Option B: fallback: bullet char + indentation (mit warning)
* Fixtures + golden erweitern

**Akzeptanz**

* Bullets/Numbering in 5 neuen Fixtures “visuell plausibel”

---

### Phase 4 — Tabellen (v0.3)

**Aufgaben**

* RTF table control words (`\trowd`, `\cell`, `\row`, etc.) subset
* DOCX table mapping (rows/cells)
* Fallback: wenn unklar → render as paragraphs mit separators + warning

**Akzeptanz**

* Tabellenfixture bleibt strukturell Tabelle (nicht kaputter Fließtext)

---

### Phase 5 — Bilder / `\pict` (v0.4)

**Aufgaben**

* `\pict` Subset (png/jpg wenn vorhanden)
* Wenn Format nicht unterstützt: drop + warning (optional placeholder text)

**Akzeptanz**

* Mind. 1 Fixture mit embedded image funktioniert, rest fällt sauber zurück

---

### Phase 6 — Optional: HTML Output (v0.4/v0.5)

**Aufgaben**

* Zweiter Writer: IR → HTML (Paragraphs/Runs/Lists/Tables)
* Nutzen: leichter diffbar, hilfreich für PDF-Pipeline via headless browser (falls später)

**Akzeptanz**

* `rtfkit convert --to html` liefert valide HTML5

---

### Phase 7 — PDF Backend (v0.6+) ohne LibreOffice

**Aufgaben**

* Renderer Backend wählen (Start: Typst)
* Pipeline: RTF → IR → Typst markup → `typst compile` → PDF
* Flag `--keep-intermediate` speichert `.typ` für Debug
* Dokumentieren: Fonts/Rendering hängt ggf. von Systemfonts ab (später bundling möglich)

**Akzeptanz**

* PDF aus 3 Fixtures erzeugbar, Layout “gut genug”, keine Crashes

---

## Qualitätsanforderungen (für alle Phasen)

* **Robustheit:** Input ist untrusted → Limit für recursion/group depth, Größenlimits, klare errors
* **Determinismus:** gleiche Inputs → gleiche Outputs (so weit möglich)
* **Erklärbarkeit:** Report im JSON: warnings, dropped content, feature coverage
* **Contribution-freundlich:** “Add fixture + expected output” ist der Standard-PR

---

## Konkrete Deliverables (für Deinen Dev)

1. Repo `rtfkit` nach Template-Pattern
2. `rtfkit convert` mit `--to docx`, `--emit-ir`, `--strict`
3. `fixtures/` + `golden/` + Test-Harness (IR golden + DOCX XML asserts)
4. CI + Release pipeline
