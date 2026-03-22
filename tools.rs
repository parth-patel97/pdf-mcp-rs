use crate::pdf;
use crate::protocol::{ContentBlock, ToolCallResult};
use serde_json::Value;

pub fn dispatch(name: &str, args: Option<&Value>) -> ToolCallResult {
    match name {
        "extract_pdf_text"  => handle_extract_text(args),
        "extract_pdf_pages" => handle_extract_pages(args),
        "get_pdf_metadata"  => handle_metadata(args),
        _ => error_result(format!("Unknown tool: {name}")),
    }
}

fn handle_extract_text(args: Option<&Value>) -> ToolCallResult {
    let path = match get_str(args, "path") {
        Ok(p) => p,
        Err(e) => return error_result(e),
    };

    match pdf::extract_text(&path) {
        Ok(content) => {
            let mut out = String::new();
            out.push_str(&format!(
                "PDF: {path}\nPages: {} | File size: {} bytes\n\n",
                content.metadata.page_count,
                content.metadata.file_size_bytes,
            ));
            for page in &content.pages {
                out.push_str(&format!("--- Page {} ---\n", page.page_number));
                if page.text.is_empty() {
                    out.push_str("[no extractable text]\n");
                } else {
                    out.push_str(&page.text);
                }
                out.push_str("\n\n");
            }
            ok_result(out.trim().to_string())
        }
        Err(e) => error_result(format!("PDF extraction failed: {e:#}")),
    }
}

fn handle_extract_pages(args: Option<&Value>) -> ToolCallResult {
    let path  = match get_str(args, "path") { Ok(p) => p, Err(e) => return error_result(e) };
    let from  = get_u32(args, "from_page").unwrap_or(1);
    let to    = get_u32(args, "to_page").unwrap_or(u32::MAX);

    match pdf::extract_pages(&path, from, to) {
        Ok(content) => {
            let mut out = String::new();
            out.push_str(&format!(
                "PDF: {path}\nExtracting pages {from}–{}\n\n",
                to.min(content.metadata.page_count)
            ));
            for page in &content.pages {
                out.push_str(&format!("--- Page {} ---\n", page.page_number));
                if page.text.is_empty() {
                    out.push_str("[no extractable text]\n");
                } else {
                    out.push_str(&page.text);
                }
                out.push_str("\n\n");
            }
            ok_result(out.trim().to_string())
        }
        Err(e) => error_result(format!("PDF extraction failed: {e:#}")),
    }
}

fn handle_metadata(args: Option<&Value>) -> ToolCallResult {
    let path = match get_str(args, "path") { Ok(p) => p, Err(e) => return error_result(e) };

    match pdf::extract_text(&path) {
        Ok(content) => {
            let m = &content.metadata;
            let mut out = String::new();
            out.push_str(&format!("File:      {path}\n"));
            out.push_str(&format!("Pages:     {}\n", m.page_count));
            out.push_str(&format!("File size: {} bytes ({:.1} KB)\n",
                m.file_size_bytes, m.file_size_bytes as f64 / 1024.0));
            if let Some(v) = &m.title    { out.push_str(&format!("Title:     {v}\n")); }
            if let Some(v) = &m.author   { out.push_str(&format!("Author:    {v}\n")); }
            if let Some(v) = &m.subject  { out.push_str(&format!("Subject:   {v}\n")); }
            if let Some(v) = &m.creator  { out.push_str(&format!("Creator:   {v}\n")); }
            if let Some(v) = &m.producer { out.push_str(&format!("Producer:  {v}\n")); }
            ok_result(out)
        }
        Err(e) => error_result(format!("Metadata read failed: {e:#}")),
    }
}

fn get_str(args: Option<&Value>, key: &str) -> Result<String, String> {
    args.and_then(|v| v.get(key))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| format!("Missing required argument: \"{key}\""))
}

fn get_u32(args: Option<&Value>, key: &str) -> Option<u32> {
    args?.get(key)?.as_u64().map(|n| n as u32)
}

fn ok_result(text: impl Into<String>) -> ToolCallResult {
    ToolCallResult { content: vec![ContentBlock::text(text.into())], is_error: false }
}

fn error_result(msg: impl Into<String>) -> ToolCallResult {
    ToolCallResult { content: vec![ContentBlock::text(msg.into())], is_error: true }
}
