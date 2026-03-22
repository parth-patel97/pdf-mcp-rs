# pdf-mcp-server

A high-performance MCP server for PDF text extraction, written in Rust.

## Tools exposed

| Tool | Required args | Optional args | Description |
|---|---|---|---|
| `extract_pdf_text` | `path` | — | Extract all text, all pages |
| `extract_pdf_pages` | `path` | `from_page`, `to_page` | Extract a specific page range |
| `get_pdf_metadata` | `path` | — | Title, author, pages, file size |

---

## Build

### Prerequisites

Rust 1.85+ is required (due to `time-macros` 0.2.27 needing edition 2024).

```bash
# Install rustup if not already installed
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Update to latest stable
rustup update stable
```

### Compile

```bash
# Debug build (fast compile, slower binary)
cargo build

# Release build (slower compile, fully optimized binary)
cargo build --release
```

Binary will be at:
- Debug:   `./target/debug/pdf-mcp-server`
- Release: `./target/release/pdf-mcp-server`

---

## Wire up to Claude Desktop

Edit `~/Library/Application Support/Claude/claude_desktop_config.json` (macOS)
or `%APPDATA%\Claude\claude_desktop_config.json` (Windows):

```json
{
  "mcpServers": {
    "pdf": {
      "command": "/absolute/path/to/pdf-mcp-server/target/release/pdf-mcp-server",
      "args": []
    }
  }
}
```

Restart Claude Desktop. The 3 PDF tools will appear in the tools panel.

---

## Wire up to your own MCP client

The server communicates over stdio using JSON-RPC 2.0. Each request and response
is a single newline-delimited JSON object.

### Handshake

```json
--> {"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","clientInfo":{"name":"my-client","version":"1.0"}}}
<-- {"jsonrpc":"2.0","id":1,"result":{"protocolVersion":"2024-11-05","serverInfo":{"name":"pdf-mcp-server","version":"0.1.0"},"capabilities":{"tools":{"listChanged":false}}}}

--> {"jsonrpc":"2.0","method":"notifications/initialized"}
(no response)
```

### List tools

```json
--> {"jsonrpc":"2.0","id":2,"method":"tools/list"}
<-- {"jsonrpc":"2.0","id":2,"result":{"tools":[...]}}
```

### Call a tool

```json
--> {"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"extract_pdf_text","arguments":{"path":"/tmp/report.pdf"}}}
<-- {"jsonrpc":"2.0","id":3,"result":{"content":[{"type":"text","text":"PDF: /tmp/report.pdf\nPages: 12..."}],"isError":false}}
```

---

## Logging

All log output goes to **stderr**. Stdout is exclusively for MCP protocol JSON.

```bash
# View logs while running manually
./target/release/pdf-mcp-server 2>pdf-mcp.log
```

---

## Project structure

```
src/
├── main.rs       ← MCP stdio event loop + tool registry
├── protocol.rs   ← JSON-RPC 2.0 + MCP type definitions
├── tools.rs      ← Tool handler dispatch (extract_text, extract_pages, metadata)
└── pdf.rs        ← lopdf extraction engine + text normalization
```

---

## Performance notes

- Uses `lopdf` — pure Rust, no C/C++ PDF library dependency
- `BufWriter` on stdout — batches writes, no syscall per character
- Release profile has `lto = true`, `codegen-units = 1`, `opt-level = 3`
- Synchronous I/O is appropriate here — PDF parsing is CPU-bound, not I/O-bound
- For very large PDFs (500+ pages), consider chunking via `extract_pdf_pages`
