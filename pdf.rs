use anyhow::{Context, Result};
use lopdf::{Document, Object};
use std::collections::BTreeMap;
use std::path::Path;

/// Extracted content from a PDF
pub struct PdfContent {
    pub pages: Vec<PageContent>,
    pub metadata: PdfMetadata,
}

pub struct PageContent {
    pub page_number: u32,
    pub text: String,
}

#[derive(Default)]
pub struct PdfMetadata {
    pub title: Option<String>,
    pub author: Option<String>,
    pub subject: Option<String>,
    pub creator: Option<String>,
    pub producer: Option<String>,
    pub page_count: u32,
    pub file_size_bytes: u64,
}

// ── Public API ────────────────────────────────────────────────────────────────

/// Extract all text from a PDF file. Returns structured page content.
pub fn extract_text(path: &str) -> Result<PdfContent> {
    let path = Path::new(path);
    let file_size = std::fs::metadata(path)
        .with_context(|| format!("Cannot access file: {path:?}"))?
        .len();

    let doc = Document::load(path)
        .with_context(|| format!("Failed to parse PDF: {path:?}"))?;

    let metadata = extract_metadata(&doc, file_size);
    let pages = extract_all_pages(&doc)?;

    Ok(PdfContent { pages, metadata })
}

/// Extract text from a specific page range (1-indexed, inclusive).
pub fn extract_pages(path: &str, from: u32, to: u32) -> Result<PdfContent> {
    let path = Path::new(path);
    let file_size = std::fs::metadata(path)
        .with_context(|| format!("Cannot access file: {path:?}"))?
        .len();

    let doc = Document::load(path)
        .with_context(|| format!("Failed to parse PDF: {path:?}"))?;

    let metadata = extract_metadata(&doc, file_size);

    let page_ids = doc.get_pages(); // BTreeMap<page_number, ObjectId>
    let total = page_ids.len() as u32;

    let from = from.max(1);
    let to = to.min(total);

    if from > to {
        anyhow::bail!("Invalid page range: {from}–{to} (document has {total} pages)");
    }

    let pages = (from..=to)
        .filter_map(|n| {
            let page_id = *page_ids.get(&n)?;
            let text = extract_page_text(&doc, page_id, n).unwrap_or_default();
            Some(PageContent { page_number: n, text })
        })
        .collect();

    Ok(PdfContent { pages, metadata })
}

// ── Internal helpers ──────────────────────────────────────────────────────────

fn extract_all_pages(doc: &Document) -> Result<Vec<PageContent>> {
    let page_ids: BTreeMap<u32, lopdf::ObjectId> = doc.get_pages();

    let pages = page_ids
        .into_iter()
        .map(|(number, id)| {
            let text = extract_page_text(doc, id, number).unwrap_or_default();
            PageContent { page_number: number, text }
        })
        .collect();

    Ok(pages)
}

fn extract_page_text(doc: &Document, page_id: lopdf::ObjectId, page_number: u32) -> Result<String> {
    let _content_streams = doc
        .get_page_content(page_id)
        .with_context(|| "Failed to read page content stream")?;

    // extract_text takes 1-indexed page numbers, not object IDs
    let raw = doc
        .extract_text(&[page_number])
        .unwrap_or_default();

    // Normalize whitespace: collapse runs, trim lines
    let normalized = normalize_text(&raw);
    Ok(normalized)
}

fn normalize_text(raw: &str) -> String {
    // Split into lines, trim each, drop blank runs, re-join
    let mut result = String::with_capacity(raw.len());
    let mut last_blank = false;

    for line in raw.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            if !last_blank {
                result.push('\n');
            }
            last_blank = true;
        } else {
            result.push_str(trimmed);
            result.push('\n');
            last_blank = false;
        }
    }

    result.trim().to_string()
}

fn extract_metadata(doc: &Document, file_size: u64) -> PdfMetadata {
    let page_count = doc.get_pages().len() as u32;

    let mut meta = PdfMetadata {
        page_count,
        file_size_bytes: file_size,
        ..Default::default()
    };

    // PDF info dictionary lives at the trailer's /Info reference
    if let Ok(info_id) = doc.trailer.get(b"Info") {
        if let Ok(Object::Dictionary(dict)) = doc.get_object(info_id.as_reference().unwrap_or((0, 0))) {
            meta.title  = get_string_entry(dict, b"Title");
            meta.author = get_string_entry(dict, b"Author");
            meta.subject = get_string_entry(dict, b"Subject");
            meta.creator = get_string_entry(dict, b"Creator");
            meta.producer = get_string_entry(dict, b"Producer");
        }
    }

    meta
}

fn get_string_entry(dict: &lopdf::Dictionary, key: &[u8]) -> Option<String> {
    match dict.get(key) {
        Ok(Object::String(bytes, _)) => {
            Some(String::from_utf8_lossy(bytes).into_owned())
        }
        _ => None,
    }
}
