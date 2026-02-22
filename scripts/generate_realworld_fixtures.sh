#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
OUT_DIR="${REPO_ROOT}/fixtures/realworld"

mkdir -p "${OUT_DIR}"

section_title_for() {
  local idx="$1"
  case "$(((idx - 1) % 8))" in
    0) echo "Executive Summary" ;;
    1) echo "Operational Performance" ;;
    2) echo "Financial Signals" ;;
    3) echo "Risk Register" ;;
    4) echo "Program Delivery" ;;
    5) echo "Workforce and Hiring" ;;
    6) echo "Security and Compliance" ;;
    7) echo "Roadmap and Dependencies" ;;
  esac
}

write_fixture() {
  local file_name="$1"
  local title="$2"
  local slug="$3"
  local pages="$4"
  local subtitle="$5"
  local out_path="${OUT_DIR}/${file_name}"

  cat >"${out_path}" <<EOF
{\rtf1\ansi\ansicpg1252\deff0
{\fonttbl{\f0 Calibri;}{\f1 Cambria;}{\f2 Consolas;}}
{\colortbl;\red0\green0\blue0;\red0\green89\blue179;\red230\green230\blue230;\red180\green0\blue0;}
\paperw12240\paperh15840\margl1440\margr1440\margt1200\margb1200
{\header \pard\qr\fs18 Confidential Internal Draft \tab ${title}\par}
{\footer \pard\qc\fs18 Page {\field{\*\fldinst PAGE}{\fldrslt 1}} of {\field{\*\fldinst NUMPAGES}{\fldrslt 1}}\par}
{\*\generator RealworldCorpusBuilder v2;}
{\*\unsupporteddestination This destination must be dropped in strict mode.}
{\listtable
{\list\listtemplateid1\listid1
{\listlevel\levelnfc23\levelnfcn23\leveljc0\levelfollow0\levelstartat1\levelspace0\levelindent0{\leveltext\'01\u8226?;}{\levelnumbers;}\fi-360\li720}
{\listlevel\levelnfc23\levelnfcn23\leveljc0\levelfollow0\levelstartat1\levelspace0\levelindent0{\leveltext\'01\u9702?;}{\levelnumbers;}\fi-360\li1440}
{\listname ;}\listid1}
{\list\listtemplateid2\listid2
{\listlevel\levelnfc0\levelnfcn0\leveljc0\levelfollow0\levelstartat1\levelspace0\levelindent0{\leveltext\'02\'00.;}{\levelnumbers\'01;}\fi-360\li720}
{\listname ;}\listid2}}
{\listoverridetable{\listoverride\listid1\listoverridecount0\ls1}{\listoverride\listid2\listoverridecount0\ls2}}
\viewkind4\uc1
\pard\sa180\fs34\b ${title}\b0\par
\pard\fs20 ${subtitle}\par
\pard\sb120\sa120\fs20 This realworld fixture intentionally blends supported and unsupported constructs to simulate converted enterprise reports.\par
EOF

  local i
  for i in $(seq 1 "${pages}"); do
    local sec_title
    sec_title="$(section_title_for "${i}")"
    local baseline=$((100 + i))
    local current=$((121 + i))
    local defects=$((18 + i))

    cat >>"${out_path}" <<EOF
\pard\sb220\sa120\fs26\b ${i}. ${sec_title}\b0\par
\pard\fs20 This section contains narrative content, mixed inline formatting, and review notes with Unicode markers: \u8212? \u8364? \u169?.\par
\pard\fs20 Program update ${i}: release cadence improved by ${i}% while cross-team coordination remains a top risk.\par
{\field{\*\fldinst HYPERLINK "https://example.com/${slug}/section-${i}"}{\fldrslt Open source appendix ${i}}}\par
{\field{\*\fldinst HYPERLINK "mailto:program-office@example.com"}{\fldrslt Contact program office}}\par
{\pict\pngblip\picwgoal2160\pichgoal960 89504E470D0A1A0A0000000D49484452000000010000000108060000001F15C4890000000A49444154789C6360000000020001E221BC330000000049454E44AE426082}\par
\pard\ls1\ilvl0 Primary finding ${i}: scope pressure is concentrated in two delivery streams.\par
\pard\ls1\ilvl1 Supporting evidence A.${i}: delivery lead-time variance widened after release cut-off.\par
\pard\ls1\ilvl1 Supporting evidence B.${i}: staffing churn increased in one business unit.\par
\pard\ls2\ilvl0 Action item ${i}: re-baseline quarterly milestones and update dependencies.\par
\trowd\trgaph108\trleft0\cellx2200\cellx4700\cellx7300\cellx10000
\intbl KPI\cell Baseline\cell Current\cell Commentary\cell\row
\trowd\trgaph108\trleft0\cellx2200\cellx4700\cellx7300\cellx10000
\intbl Throughput\cell ${baseline}\cell ${current}\cell Stable but sensitive to resource contention\cell\row
\trowd\trgaph108\trleft0\cellx2200\cellx4700\cellx7300\cellx10000
\intbl Defect density\cell 24\cell ${defects}\cell Trending down with patch-cycle enforcement\cell\row
\pard\fs20 Closing note ${i}: decisions are pending on governance and long-tail remediation ownership.\par
EOF

    if (( i % 3 == 0 )); then
      cat >>"${out_path}" <<EOF
{\footnote \pard\plain\fs18 Footnote ${i}: legal review requested for archived exceptions and risk acceptance boundaries.}\par
EOF
    fi

    if (( i % 4 == 0 )); then
      cat >>"${out_path}" <<'EOF'
\sect\sectd\lndscpsxn\pgwsxn15840\pghsxn12240\marglsxn900\margrsxn900
\pard\sb120\sa80\fs22\b Landscape matrix view\b0\par
\trowd\trgaph108\trleft0\cellx2200\cellx4400\cellx6600\cellx8800\cellx11000\cellx13600
\intbl Region\cell Owner\cell Severity\cell ETA\cell Status\cell Notes\cell\row
\trowd\trgaph108\trleft0\cellx2200\cellx4400\cellx6600\cellx8800\cellx11000\cellx13600
\intbl North America\cell Ops Lead\cell High\cell 2026-03-20\cell In Progress\cell Needs vendor mitigation\cell\row
\trowd\trgaph108\trleft0\cellx2200\cellx4400\cellx6600\cellx8800\cellx11000\cellx13600
\intbl EMEA\cell Security Lead\cell Medium\cell 2026-04-01\cell Planned\cell Awaiting procurement sign-off\cell\row
\sect\sectd\lndscpsxn0\pgwsxn12240\pghsxn15840\marglsxn1440\margrsxn1440
EOF
    fi

    if (( i < pages )); then
      printf '%s\n' '\page' >>"${out_path}"
    fi
  done

  printf '%s\n' '}' >>"${out_path}"
}

write_meta() {
  local file_name="$1"
  local description="$2"
  local min_docx="$3"
  local min_html="$4"
  local meta_path="${OUT_DIR}/${file_name%.rtf}.meta.json"

  cat >"${meta_path}" <<EOF
{
  "fixture": "${file_name}",
  "description": "${description}",
  "expected_non_strict_exit": 0,
  "expected_strict_exit": 4,
  "required_warning_types": ["dropped_content"],
  "required_dropped_reasons": ["Dropped unsupported RTF destination content"],
  "min_docx_bytes": ${min_docx},
  "min_html_bytes": ${min_html}
}
EOF
}

write_fixture \
  "annual_report_10p.rtf" \
  "Annual Operating Report (FY2025)" \
  "annual" \
  10 \
  "Corporate operating report with executive narrative, embedded links, matrix tables, and mixed formatting."

write_meta \
  "annual_report_10p.rtf" \
  "Annual-style report with page formatting, headers/footers, hyperlinks, images, lists, and tables." \
  18000 \
  7000

write_fixture \
  "technical_spec_12p.rtf" \
  "Platform Technical Specification" \
  "technical" \
  12 \
  "Technical specification with implementation notes, wide compliance matrices, and structured decision logs."

write_meta \
  "technical_spec_12p.rtf" \
  "Technical spec with repeated sections, deep tables, links, and embedded images." \
  20000 \
  8000

write_fixture \
  "policy_doc_15p.rtf" \
  "Global Policy and Controls Handbook" \
  "policy" \
  15 \
  "Policy handbook with governance controls, compliance evidence references, and long-form procedural content."

write_meta \
  "policy_doc_15p.rtf" \
  "Policy handbook style fixture with heavy narrative, page breaks, tables, links, and footnotes." \
  22000 \
  10000

echo "Generated fixtures:"
ls -1 "${OUT_DIR}"/*.rtf "${OUT_DIR}"/*.meta.json | sed "s#${REPO_ROOT}/##"
