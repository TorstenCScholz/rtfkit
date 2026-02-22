use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};

use serde_json::Value;
use tempfile::TempDir;
use zip::ZipArchive;

#[derive(Debug, Clone)]
struct CorpusMeta {
    fixture: String,
    description: String,
    expected_non_strict_exit: i32,
    expected_strict_exit: i32,
    required_warning_types: Vec<String>,
    required_dropped_reasons: Vec<String>,
    min_docx_bytes: u64,
    min_html_bytes: u64,
}

fn project_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf()
}

fn realworld_dir() -> PathBuf {
    project_root().join("fixtures").join("realworld")
}

fn load_meta(path: &Path) -> CorpusMeta {
    let raw = fs::read_to_string(path)
        .unwrap_or_else(|e| panic!("failed to read metadata file {}: {e}", path.display()));
    let value: Value = serde_json::from_str(&raw)
        .unwrap_or_else(|e| panic!("failed to parse metadata file {}: {e}", path.display()));

    fn req_string(value: &Value, key: &str) -> String {
        value
            .get(key)
            .and_then(Value::as_str)
            .unwrap_or_else(|| panic!("missing or invalid string field '{key}'"))
            .to_string()
    }

    fn req_i32(value: &Value, key: &str) -> i32 {
        value
            .get(key)
            .and_then(Value::as_i64)
            .unwrap_or_else(|| panic!("missing or invalid integer field '{key}'")) as i32
    }

    fn req_u64(value: &Value, key: &str) -> u64 {
        value
            .get(key)
            .and_then(Value::as_u64)
            .unwrap_or_else(|| panic!("missing or invalid u64 field '{key}'"))
    }

    fn req_string_array(value: &Value, key: &str) -> Vec<String> {
        let arr = value
            .get(key)
            .and_then(Value::as_array)
            .unwrap_or_else(|| panic!("missing or invalid array field '{key}'"));

        arr.iter()
            .map(|v| {
                v.as_str()
                    .unwrap_or_else(|| panic!("array '{key}' must contain only strings"))
                    .to_string()
            })
            .collect()
    }

    CorpusMeta {
        fixture: req_string(&value, "fixture"),
        description: req_string(&value, "description"),
        expected_non_strict_exit: req_i32(&value, "expected_non_strict_exit"),
        expected_strict_exit: req_i32(&value, "expected_strict_exit"),
        required_warning_types: req_string_array(&value, "required_warning_types"),
        required_dropped_reasons: req_string_array(&value, "required_dropped_reasons"),
        min_docx_bytes: req_u64(&value, "min_docx_bytes"),
        min_html_bytes: req_u64(&value, "min_html_bytes"),
    }
}

fn load_corpus() -> Vec<CorpusMeta> {
    let mut entries: Vec<PathBuf> = fs::read_dir(realworld_dir())
        .expect("failed to read realworld fixture directory")
        .filter_map(|entry| entry.ok().map(|e| e.path()))
        .filter(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.ends_with(".meta.json"))
        })
        .collect();

    entries.sort();
    assert!(
        !entries.is_empty(),
        "realworld corpus must contain at least one .meta.json file"
    );

    entries.into_iter().map(|path| load_meta(&path)).collect()
}

fn run_cli(args: &[String]) -> std::process::Output {
    let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("rtfkit");
    cmd.args(args);
    cmd.output().expect("failed to execute rtfkit")
}

fn assert_exit_code(output: &std::process::Output, expected: i32, context: &str) {
    let actual = output
        .status
        .code()
        .unwrap_or_else(|| panic!("process terminated by signal for {context}"));

    assert_eq!(
        actual,
        expected,
        "unexpected exit code for {context}\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

fn extract_docx_xml(docx_path: &Path, xml_name: &str) -> String {
    let file = fs::File::open(docx_path)
        .unwrap_or_else(|e| panic!("failed to open DOCX {}: {e}", docx_path.display()));
    let mut archive = ZipArchive::new(file)
        .unwrap_or_else(|e| panic!("failed to read DOCX zip {}: {e}", docx_path.display()));

    let mut xml = String::new();
    archive
        .by_name(xml_name)
        .unwrap_or_else(|e| panic!("missing {xml_name} in {}: {e}", docx_path.display()))
        .read_to_string(&mut xml)
        .unwrap_or_else(|e| {
            panic!(
                "failed to read {xml_name} from {}: {e}",
                docx_path.display()
            )
        });

    xml
}

fn assert_realworld_fixture_richness(fixture_path: &Path, meta: &CorpusMeta) {
    let raw = fs::read_to_string(fixture_path)
        .unwrap_or_else(|e| panic!("failed to read fixture {}: {e}", fixture_path.display()));

    let required_tokens = [
        ("\\paperw", "page geometry"),
        ("\\margl", "page margin"),
        ("{\\header", "header destination"),
        ("{\\footer", "footer destination"),
        ("\\trowd", "table rows"),
        ("{\\pict", "embedded image"),
        ("HYPERLINK", "hyperlink fields"),
    ];

    for (token, feature_name) in required_tokens {
        assert!(
            raw.contains(token),
            "{} must include {} token '{}' ({})",
            meta.fixture,
            feature_name,
            token,
            meta.description
        );
    }

    let page_count = raw.matches("\\page").count() + 1;
    assert!(
        page_count >= 10,
        "{} must contain at least 10 pages worth of content, found {} ({})",
        meta.fixture,
        page_count,
        meta.description
    );

    let link_count = raw.matches("HYPERLINK").count();
    assert!(
        link_count >= 5,
        "{} should contain multiple link fields, found {} ({})",
        meta.fixture,
        link_count,
        meta.description
    );

    let table_row_count = raw.matches("\\trowd").count();
    assert!(
        table_row_count >= 10,
        "{} should contain substantial table content, found {} table rows ({})",
        meta.fixture,
        table_row_count,
        meta.description
    );
}

#[test]
fn realworld_corpus_contracts() {
    let corpus = load_corpus();
    let base = realworld_dir();

    for meta in corpus {
        let fixture_path = base.join(&meta.fixture);
        assert!(
            fixture_path.exists(),
            "fixture listed in metadata does not exist: {}",
            fixture_path.display()
        );
        assert_realworld_fixture_richness(&fixture_path, &meta);

        let fixture = fixture_path.to_str().unwrap().to_string();
        let stem = Path::new(&meta.fixture)
            .file_stem()
            .unwrap()
            .to_str()
            .unwrap()
            .to_string();

        let temp = TempDir::new().unwrap();
        let docx_path = temp.path().join(format!("{stem}.docx"));
        let html_path = temp.path().join(format!("{stem}.html"));

        let docx_args = vec![
            "convert".to_string(),
            fixture.clone(),
            "-o".to_string(),
            docx_path.to_str().unwrap().to_string(),
            "--force".to_string(),
        ];
        let docx_output = run_cli(&docx_args);
        assert_exit_code(
            &docx_output,
            meta.expected_non_strict_exit,
            &format!("{} DOCX", meta.fixture),
        );

        let html_args = vec![
            "convert".to_string(),
            fixture.clone(),
            "--to".to_string(),
            "html".to_string(),
            "-o".to_string(),
            html_path.to_str().unwrap().to_string(),
            "--force".to_string(),
        ];
        let html_output = run_cli(&html_args);
        assert_exit_code(
            &html_output,
            meta.expected_non_strict_exit,
            &format!("{} HTML", meta.fixture),
        );

        if meta.expected_non_strict_exit == 0 {
            let docx_meta = fs::metadata(&docx_path)
                .unwrap_or_else(|e| panic!("missing DOCX output {}: {e}", docx_path.display()));
            let html_meta = fs::metadata(&html_path)
                .unwrap_or_else(|e| panic!("missing HTML output {}: {e}", html_path.display()));

            assert!(
                docx_meta.len() >= meta.min_docx_bytes,
                "DOCX output too small for {}: {} < {}",
                meta.fixture,
                docx_meta.len(),
                meta.min_docx_bytes
            );
            assert!(
                html_meta.len() >= meta.min_html_bytes,
                "HTML output too small for {}: {} < {}",
                meta.fixture,
                html_meta.len(),
                meta.min_html_bytes
            );
        }

        let report_args = vec![
            "convert".to_string(),
            fixture.clone(),
            "--format".to_string(),
            "json".to_string(),
        ];
        let report_output = run_cli(&report_args);
        assert_exit_code(
            &report_output,
            meta.expected_non_strict_exit,
            &format!("{} report-json", meta.fixture),
        );

        let report: Value = serde_json::from_slice(&report_output.stdout).unwrap_or_else(|e| {
            panic!(
                "invalid report JSON for {}: {e}\nstdout:\n{}",
                meta.fixture,
                String::from_utf8_lossy(&report_output.stdout)
            )
        });

        let warnings = report
            .get("warnings")
            .and_then(Value::as_array)
            .unwrap_or_else(|| panic!("report JSON missing warnings array for {}", meta.fixture));

        let warning_types: Vec<String> = warnings
            .iter()
            .filter_map(|w| {
                w.get("type")
                    .and_then(Value::as_str)
                    .map(ToString::to_string)
            })
            .collect();

        let dropped_reasons: Vec<String> = warnings
            .iter()
            .filter_map(|w| {
                if w.get("type").and_then(Value::as_str) == Some("dropped_content") {
                    w.get("reason")
                        .and_then(Value::as_str)
                        .map(ToString::to_string)
                } else {
                    None
                }
            })
            .collect();

        for required_type in &meta.required_warning_types {
            assert!(
                warning_types.iter().any(|t| t == required_type),
                "{} is missing required warning type '{}' ({})",
                meta.fixture,
                required_type,
                meta.description
            );
        }

        for required_reason in &meta.required_dropped_reasons {
            assert!(
                dropped_reasons.iter().any(|r| r == required_reason),
                "{} is missing required dropped-content reason '{}' ({})",
                meta.fixture,
                required_reason,
                meta.description
            );
        }

        let strict_args = vec![
            "convert".to_string(),
            fixture,
            "--strict".to_string(),
            "--format".to_string(),
            "json".to_string(),
        ];
        let strict_output = run_cli(&strict_args);
        assert_exit_code(
            &strict_output,
            meta.expected_strict_exit,
            &format!("{} strict", meta.fixture),
        );

        if meta.expected_strict_exit == 4 {
            assert!(
                String::from_utf8_lossy(&strict_output.stderr).contains("Strict mode violated"),
                "strict mode expected failure message for {}",
                meta.fixture
            );
        }
    }
}

#[test]
fn realworld_corpus_determinism() {
    let corpus = load_corpus();
    let base = realworld_dir();

    for meta in corpus {
        let fixture_path = base.join(&meta.fixture);
        let fixture = fixture_path.to_str().unwrap().to_string();
        let stem = Path::new(&meta.fixture)
            .file_stem()
            .unwrap()
            .to_str()
            .unwrap()
            .to_string();

        let temp = TempDir::new().unwrap();

        let mut html_runs: Vec<Vec<u8>> = Vec::new();
        let mut docx_xml_runs: Vec<String> = Vec::new();

        for i in 0..3 {
            let html_path = temp.path().join(format!("{stem}_{i}.html"));
            let docx_path = temp.path().join(format!("{stem}_{i}.docx"));

            let html_args = vec![
                "convert".to_string(),
                fixture.clone(),
                "--to".to_string(),
                "html".to_string(),
                "-o".to_string(),
                html_path.to_str().unwrap().to_string(),
                "--force".to_string(),
            ];
            let html_output = run_cli(&html_args);
            assert_exit_code(
                &html_output,
                meta.expected_non_strict_exit,
                &format!("{} html run {i}", meta.fixture),
            );

            let docx_args = vec![
                "convert".to_string(),
                fixture.clone(),
                "-o".to_string(),
                docx_path.to_str().unwrap().to_string(),
                "--force".to_string(),
            ];
            let docx_output = run_cli(&docx_args);
            assert_exit_code(
                &docx_output,
                meta.expected_non_strict_exit,
                &format!("{} docx run {i}", meta.fixture),
            );

            html_runs.push(
                fs::read(&html_path)
                    .unwrap_or_else(|e| panic!("failed to read {}: {e}", html_path.display())),
            );
            docx_xml_runs.push(extract_docx_xml(&docx_path, "word/document.xml"));
        }

        for (idx, html) in html_runs.iter().enumerate().skip(1) {
            assert_eq!(
                html_runs[0], *html,
                "HTML output is non-deterministic for {} (run 0 vs run {idx})",
                meta.fixture
            );
        }

        for (idx, xml) in docx_xml_runs.iter().enumerate().skip(1) {
            assert_eq!(
                docx_xml_runs[0], *xml,
                "DOCX document.xml is non-deterministic for {} (run 0 vs run {idx})",
                meta.fixture
            );
        }
    }
}
