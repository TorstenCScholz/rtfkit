#![allow(dead_code)]

use std::fs::File;
use std::io::Read;
use std::path::Path;

use quick_xml::Reader;
use quick_xml::events::Event;
use zip::ZipArchive;

/// Extract word/document.xml from a DOCX file.
/// Returns the XML content as a string.
pub fn extract_document_xml(docx_path: &Path) -> String {
    let file = File::open(docx_path).expect("Failed to open DOCX file");
    let mut archive = ZipArchive::new(file).expect("Failed to read DOCX as ZIP");

    let mut document_xml = String::new();
    archive
        .by_name("word/document.xml")
        .expect("word/document.xml not found in DOCX")
        .read_to_string(&mut document_xml)
        .expect("Failed to read document.xml");

    document_xml
}

/// Extract any XML file from a DOCX archive.
/// Returns the XML content as a string.
pub fn extract_xml_from_docx(docx_path: &Path, xml_path: &str) -> String {
    let file = File::open(docx_path).expect("Failed to open DOCX file");
    let mut archive = ZipArchive::new(file).expect("Failed to read DOCX as ZIP");

    let mut xml_content = String::new();
    archive
        .by_name(xml_path)
        .unwrap_or_else(|_| panic!("{xml_path} not found in DOCX"))
        .read_to_string(&mut xml_content)
        .expect("Failed to read XML content");

    xml_content
}

/// Extract an XML file from a DOCX archive (determinism variant).
/// Returns the XML content as a string.
pub fn extract_docx_xml(docx_path: &Path, xml_name: &str) -> String {
    let file = File::open(docx_path).expect("Failed to open DOCX file");
    let mut archive = ZipArchive::new(file).expect("Failed to read DOCX as ZIP");

    let mut xml_content = String::new();
    archive
        .by_name(xml_name)
        .unwrap_or_else(|_| panic!("{xml_name} not found in DOCX"))
        .read_to_string(&mut xml_content)
        .expect("Failed to read XML content");

    xml_content
}

/// Extract word/numbering.xml from a DOCX file.
/// Returns None if the file doesn't exist.
pub fn extract_numbering_xml(docx_path: &Path) -> Option<String> {
    let file = File::open(docx_path).expect("Failed to open DOCX file");
    let mut archive = ZipArchive::new(file).expect("Failed to read DOCX as ZIP");

    let mut numbering_xml = String::new();
    match archive.by_name("word/numbering.xml") {
        Ok(mut file) => {
            file.read_to_string(&mut numbering_xml)
                .expect("Failed to read numbering.xml");
            Some(numbering_xml)
        }
        Err(_) => None,
    }
}

/// Extract word/_rels/document.xml.rels from a DOCX file.
pub fn extract_rels_xml(docx_path: &Path) -> Option<String> {
    let file = File::open(docx_path).expect("Failed to open DOCX file");
    let mut archive = ZipArchive::new(file).expect("Failed to read DOCX as ZIP");

    let mut rels_xml = String::new();
    match archive.by_name("word/_rels/document.xml.rels") {
        Ok(mut file) => {
            file.read_to_string(&mut rels_xml)
                .expect("Failed to read document.xml.rels");
            Some(rels_xml)
        }
        Err(_) => None,
    }
}

/// Check if a file exists inside a DOCX archive.
pub fn file_exists_in_docx(docx_path: &Path, file_path: &str) -> bool {
    let file = File::open(docx_path).expect("Failed to open DOCX file");
    let mut archive = ZipArchive::new(file).expect("Failed to read DOCX as ZIP");
    archive.by_name(file_path).is_ok()
}

/// Get the list of files in a DOCX archive directory.
pub fn list_docx_files(docx_path: &Path, prefix: &str) -> Vec<String> {
    let file = File::open(docx_path).expect("Failed to open DOCX file");
    let archive = ZipArchive::new(file).expect("Failed to read DOCX as ZIP");

    archive
        .file_names()
        .filter(|name| name.starts_with(prefix))
        .map(|s| s.to_string())
        .collect()
}

/// Count occurrences of a specific XML element in the document.
pub fn count_elements(xml: &str, element_name: &str) -> usize {
    let mut reader = Reader::from_str(xml);
    let mut count = 0;

    loop {
        match reader.read_event() {
            Ok(Event::Start(e)) | Ok(Event::Empty(e)) => {
                if e.name().as_ref() == element_name.as_bytes() {
                    count += 1;
                }
            }
            Ok(Event::Eof) => break,
            Err(_) => break,
            _ => {}
        }
    }

    count
}

/// Check if the XML contains a specific text string within a <w:t> element.
pub fn contains_text(xml: &str, text: &str) -> bool {
    let mut reader = Reader::from_str(xml);
    let mut in_text_element = false;
    let mut buf = Vec::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e)) => {
                if e.name().as_ref() == b"w:t" {
                    in_text_element = true;
                }
            }
            Ok(Event::Empty(e)) => {
                if e.name().as_ref() == b"w:t" {
                    in_text_element = false;
                }
            }
            Ok(Event::Text(e)) => {
                if in_text_element {
                    if let Ok(t) = e.unescape() {
                        if t.contains(text) {
                            return true;
                        }
                    }
                }
            }
            Ok(Event::End(e)) => {
                if e.name().as_ref() == b"w:t" {
                    in_text_element = false;
                }
            }
            Ok(Event::Eof) => break,
            Err(_) => break,
            _ => {}
        }
        buf.clear();
    }

    false
}

/// Get all text content from <w:t> elements.
pub fn get_all_text_content(xml: &str) -> Vec<String> {
    let mut reader = Reader::from_str(xml);
    let mut in_text_element = false;
    let mut texts = Vec::new();
    let mut buf = Vec::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e)) => {
                if e.name().as_ref() == b"w:t" {
                    in_text_element = true;
                }
            }
            Ok(Event::Text(e)) => {
                if in_text_element {
                    if let Ok(t) = e.unescape() {
                        texts.push(t.to_string());
                    }
                }
            }
            Ok(Event::End(e)) => {
                if e.name().as_ref() == b"w:t" {
                    in_text_element = false;
                }
            }
            Ok(Event::Eof) => break,
            Err(_) => break,
            _ => {}
        }
        buf.clear();
    }

    texts
}

/// Check if a formatting element (like <w:b>, <w:i>, <w:u>) exists in the document.
pub fn has_formatting_element(xml: &str, element_name: &str) -> bool {
    let mut reader = Reader::from_str(xml);

    loop {
        match reader.read_event() {
            Ok(Event::Start(e)) | Ok(Event::Empty(e)) => {
                if e.name().as_ref() == element_name.as_bytes() {
                    return true;
                }
            }
            Ok(Event::Eof) => break,
            Err(_) => break,
            _ => {}
        }
    }

    false
}

/// Check if a <w:jc w:val="..."> element exists with the specified alignment value.
pub fn has_alignment(xml: &str, alignment_value: &str) -> bool {
    let mut reader = Reader::from_str(xml);
    let mut buf = Vec::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e)) | Ok(Event::Empty(e)) => {
                if e.name().as_ref() == b"w:jc" {
                    for attr in e.attributes().flatten() {
                        if attr.key.as_ref() == b"w:val" {
                            if let Ok(value) = std::str::from_utf8(&attr.value) {
                                if value == alignment_value {
                                    return true;
                                }
                            }
                        }
                    }
                }
            }
            Ok(Event::Eof) => break,
            Err(_) => break,
            _ => {}
        }
        buf.clear();
    }

    false
}

/// Check if a run (<w:r>) with specific formatting exists containing the given text.
pub fn has_run_with_formatting_and_text(xml: &str, formatting_element: &str, text: &str) -> bool {
    let mut reader = Reader::from_str(xml);
    let mut in_run = false;
    let mut has_formatting = false;
    let mut in_text = false;
    let mut current_text = String::new();
    let mut buf = Vec::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e)) => {
                match e.name().as_ref() {
                    b"w:r" => {
                        in_run = true;
                        has_formatting = false;
                        current_text.clear();
                    }
                    b"w:rPr" => {
                        // Run properties - formatting will be inside
                    }
                    b"w:t" => {
                        in_text = true;
                    }
                    tag if tag == formatting_element.as_bytes() && in_run => {
                        has_formatting = true;
                    }
                    _ => {}
                }
            }
            Ok(Event::Empty(e)) => match e.name().as_ref() {
                tag if tag == formatting_element.as_bytes() && in_run => {
                    has_formatting = true;
                }
                _ => {}
            },
            Ok(Event::Text(e)) => {
                if in_text {
                    if let Ok(t) = e.unescape() {
                        current_text.push_str(&t);
                    }
                }
            }
            Ok(Event::End(e)) => match e.name().as_ref() {
                b"w:r" => {
                    if has_formatting && current_text.contains(text) {
                        return true;
                    }
                    in_run = false;
                }
                b"w:t" => {
                    in_text = false;
                }
                _ => {}
            },
            Ok(Event::Eof) => break,
            Err(_) => break,
            _ => {}
        }
        buf.clear();
    }

    false
}

/// Check if a <w:numPr> element exists in the document (indicates list paragraph).
pub fn has_num_pr(xml: &str) -> bool {
    has_formatting_element(xml, "w:numPr")
}

/// Count the number of <w:numPr> elements in the document.
pub fn count_num_pr(xml: &str) -> usize {
    count_elements(xml, "w:numPr")
}

/// Check if a <w:ilvl> element exists with the specified level value.
pub fn has_ilvl_with_value(xml: &str, level: u8) -> bool {
    let mut reader = Reader::from_str(xml);
    let mut buf = Vec::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e)) | Ok(Event::Empty(e)) => {
                if e.name().as_ref() == b"w:ilvl" {
                    for attr in e.attributes().flatten() {
                        if attr.key.as_ref() == b"w:val" {
                            if let Ok(value) = std::str::from_utf8(&attr.value) {
                                if let Ok(v) = value.parse::<u8>() {
                                    if v == level {
                                        return true;
                                    }
                                }
                            }
                        }
                    }
                }
            }
            Ok(Event::Eof) => break,
            Err(_) => break,
            _ => {}
        }
        buf.clear();
    }

    false
}

/// Check if numbering.xml contains an <w:abstractNum> element.
pub fn has_abstract_num(numbering_xml: &str) -> bool {
    has_formatting_element(numbering_xml, "w:abstractNum")
}

/// Check if numbering.xml contains a <w:num> element.
pub fn has_num(numbering_xml: &str) -> bool {
    has_formatting_element(numbering_xml, "w:num")
}

/// Extract numId values from document.xml to verify stability.
pub fn extract_num_ids(xml: &str) -> Vec<String> {
    let mut reader = Reader::from_str(xml);
    let mut buf = Vec::new();
    let mut num_ids = Vec::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e)) | Ok(Event::Empty(e)) => {
                if e.name().as_ref() == b"w:numId" {
                    for attr in e.attributes().flatten() {
                        if attr.key.as_ref() == b"w:val" {
                            if let Ok(value) = std::str::from_utf8(&attr.value) {
                                num_ids.push(value.to_string());
                            }
                        }
                    }
                }
            }
            Ok(Event::Eof) => break,
            Err(_) => break,
            _ => {}
        }
        buf.clear();
    }

    num_ids
}

/// Extract abstractNumId values from numbering.xml to verify stability.
pub fn extract_abstract_num_ids(numbering_xml: &str) -> Vec<String> {
    let mut reader = Reader::from_str(numbering_xml);
    let mut buf = Vec::new();
    let mut abstract_num_ids = Vec::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e)) | Ok(Event::Empty(e)) => {
                if e.name().as_ref() == b"w:abstractNumId" {
                    for attr in e.attributes().flatten() {
                        if attr.key.as_ref() == b"w:val" {
                            if let Ok(value) = std::str::from_utf8(&attr.value) {
                                abstract_num_ids.push(value.to_string());
                            }
                        }
                    }
                }
            }
            Ok(Event::Eof) => break,
            Err(_) => break,
            _ => {}
        }
        buf.clear();
    }

    abstract_num_ids
}

/// Check if a <w:gridSpan> element exists with the specified span value.
pub fn has_grid_span_with_value(xml: &str, span: u16) -> bool {
    let mut reader = Reader::from_str(xml);
    let mut buf = Vec::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e)) | Ok(Event::Empty(e)) => {
                if e.name().as_ref() == b"w:gridSpan" {
                    for attr in e.attributes().flatten() {
                        if attr.key.as_ref() == b"w:val" {
                            if let Ok(value) = std::str::from_utf8(&attr.value) {
                                if let Ok(v) = value.parse::<u16>() {
                                    if v == span {
                                        return true;
                                    }
                                }
                            }
                        }
                    }
                }
            }
            Ok(Event::Eof) => break,
            Err(_) => break,
            _ => {}
        }
        buf.clear();
    }

    false
}

/// Check if a <w:vMerge> element exists in the document.
pub fn has_vmerge_element(xml: &str) -> bool {
    has_formatting_element(xml, "w:vMerge")
}
