mod pdf;
mod protocol;
mod tools;

use anyhow::Result;
use protocol::*;
use serde_json::{json, Value};
use std::io::{self, BufRead, Write};

macro_rules! log {
    ($level:expr, $($arg:tt)*) => {
        eprintln!("[{}] {}", $level, format!($($arg)*));
    };
}

fn main() -> Result<()> {
    log!("INFO", "pdf-mcp-server starting on stdio");

    let stdin = io::stdin();
    let stdout = io::stdout();
    let mut out = io::BufWriter::new(stdout.lock());

    for line in stdin.lock().lines() {
        let line = match line {
            Ok(l) if l.trim().is_empty() => continue,
            Ok(l) => l,
            Err(e) => {
                log!("ERROR", "stdin read error: {e}");
                break;
            }
        };

        let response = handle_line(&line);

        if let Some(resp) = response {
            let serialized = serde_json::to_string(&resp)?;
            writeln!(out, "{serialized}")?;
            out.flush()?;
        }
    }

    log!("INFO", "pdf-mcp-server shutting down");
    Ok(())
}

fn handle_line(line: &str) -> Option<RpcResponse> {
    let req: RpcRequest = match serde_json::from_str(line) {
        Ok(r) => r,
        Err(e) => {
            log!("ERROR", "JSON parse error: {e}");
            return Some(RpcResponse::err(None, -32700, "Parse error"));
        }
    };

    let id = req.id.clone();

    match req.method.as_str() {
        "initialize" => {
            log!("INFO", "client initializing");
            Some(RpcResponse::ok(id, json!(InitializeResult {
                protocol_version: "2024-11-05",
                server_info: ServerInfo { name: "pdf-mcp-server", version: "0.1.0" },
                capabilities: Capabilities { tools: ToolsCapability { list_changed: false } },
            })))
        }
        "notifications/initialized" => {
            log!("INFO", "client initialized");
            None
        }
        "ping" => Some(RpcResponse::ok(id, json!({}))),
        "tools/list" => {
            Some(RpcResponse::ok(id, json!(ToolsListResult { tools: tool_definitions() })))
        }
        "tools/call" => {
            let params: ToolCallParams = match parse_params(req.params) {
                Ok(p) => p,
                Err(e) => return Some(RpcResponse::err(id, -32602, e)),
            };
            log!("INFO", "tool call: {}", params.name);
            let result = tools::dispatch(&params.name, params.arguments.as_ref());
            Some(RpcResponse::ok(id, serde_json::to_value(result).unwrap()))
        }
        other => {
            log!("WARN", "unknown method: {other}");
            Some(RpcResponse::err(id, -32601, format!("Method not found: {other}")))
        }
    }
}

fn tool_definitions() -> Vec<Tool> {
    vec![
        Tool {
            name: "extract_pdf_text",
            description: "Extract all text from a PDF file, organized by page.",
            input_schema: json!({
                "type": "object",
                "properties": {
                    "path": { "type": "string", "description": "Absolute or relative path to the PDF file" }
                },
                "required": ["path"]
            }),
        },
        Tool {
            name: "extract_pdf_pages",
            description: "Extract text from a specific page range in a PDF (1-indexed, inclusive).",
            input_schema: json!({
                "type": "object",
                "properties": {
                    "path": { "type": "string", "description": "Path to the PDF file" },
                    "from_page": { "type": "integer", "description": "First page to extract (default: 1)", "minimum": 1 },
                    "to_page":   { "type": "integer", "description": "Last page to extract (default: last)", "minimum": 1 }
                },
                "required": ["path"]
            }),
        },
        Tool {
            name: "get_pdf_metadata",
            description: "Get PDF metadata: title, author, subject, page count, file size.",
            input_schema: json!({
                "type": "object",
                "properties": {
                    "path": { "type": "string", "description": "Path to the PDF file" }
                },
                "required": ["path"]
            }),
        },
    ]
}

fn parse_params<T: serde::de::DeserializeOwned>(v: Option<Value>) -> Result<T, String> {
    match v {
        Some(val) => serde_json::from_value(val).map_err(|e| e.to_string()),
        None => Err("Missing params".to_string()),
    }
}
