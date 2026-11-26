//! `vmem mcp`: a Model Context Protocol server over stdio.
//!
//! One memory, two doors. Voice agents read the packs by URL; desktop agents
//! (Claude Code, Claude Desktop, any MCP client) call these tools and get the
//! same memory, with the same validation and rebuilds as the CLI. Messages are
//! JSON-RPC 2.0, one per line; logs never go to stdout.

use crate::app::{self, Failure, Remember};
use serde_json::{json, Value};
use std::io::{self, BufRead, Write};
use std::path::{Path, PathBuf};

const PROTOCOL_VERSIONS: [&str; 4] = ["2025-11-25", "2025-06-18", "2025-03-26", "2024-11-05"];
const KIND_NAMES: [&str; 11] = [
    "profile", "preference", "rule", "goal", "project", "person", "decision", "task", "question", "fact", "term",
];

pub fn serve(root: PathBuf) -> Result<(), String> {
    let stdin = io::stdin();
    let mut out = io::stdout().lock();
    for line in stdin.lock().lines() {
        let line = line.map_err(|e| e.to_string())?;
        if line.trim().is_empty() {
            continue;
        }
        if let Some(reply) = handle_line(&root, &line) {
            writeln!(out, "{reply}").map_err(|e| e.to_string())?;
            out.flush().map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}

/// Handles one line of input; `None` for notifications.
pub fn handle_line(root: &Path, line: &str) -> Option<String> {
    let msg: Value = match serde_json::from_str(line) {
        Ok(v) => v,
        Err(e) => return Some(error(Value::Null, -32700, &format!("parse error: {e}")).to_string()),
    };
    if let Value::Array(batch) = &msg {
        let replies: Vec<Value> = batch.iter().filter_map(|m| handle(root, m)).collect();
        return (!replies.is_empty()).then(|| Value::Array(replies).to_string());
    }
    handle(root, &msg).map(|v| v.to_string())
}

fn error(id: Value, code: i64, message: &str) -> Value {
    json!({ "jsonrpc": "2.0", "id": id, "error": { "code": code, "message": message } })
}

fn handle(root: &Path, msg: &Value) -> Option<Value> {
    let id = msg.get("id")?.clone();
    let method = msg.get("method").and_then(Value::as_str).unwrap_or("");
    let params = msg.get("params").cloned().unwrap_or(Value::Null);
    let result = match method {
        "initialize" => Ok(initialize(&params)),
        "ping" => Ok(json!({})),
        "tools/list" => Ok(json!({ "tools": tools() })),
        "tools/call" => call(root, &params),
        _ => Err((-32601, format!("method not found: {method}"))),
    };
    Some(match result {
        Ok(r) => json!({ "jsonrpc": "2.0", "id": id, "result": r }),
        Err((code, m)) => error(id, code, &m),
    })
}

fn initialize(params: &Value) -> Value {
    let asked = params.get("protocolVersion").and_then(Value::as_str).unwrap_or("");
    let version = if PROTOCOL_VERSIONS.contains(&asked) { asked } else { PROTOCOL_VERSIONS[0] };
    json!({
        "protocolVersion": version,
        "capabilities": { "tools": { "listChanged": false } },
        "serverInfo": { "name": "vmem", "version": env!("CARGO_PKG_VERSION") },
        "instructions": "Memory shared with a stateless voice agent. Call memory_resume first to see where the last session ended. Save new knowledge with memory_remember or memory_ingest_handoff; every write rebuilds the packs the voice agent reads. Commit and push the repo afterwards so the voice agent sees the change."
    })
}

fn tool(name: &str, description: &str, schema: Value) -> Value {
    json!({ "name": name, "description": description, "inputSchema": schema })
}

fn tools() -> Vec<Value> {
    vec![
        tool(
            "memory_resume",
            "Where the last session ended: summary, next steps, open questions, items waiting for a screen. Call this first.",
            json!({ "type": "object", "properties": {
                "spoken": { "type": "boolean", "description": "Return a short recap ready to be said aloud." } } }),
        ),
        tool(
            "memory_search",
            "Search memory by keywords (BM25 plus recency). Tolerates misheard words.",
            json!({ "type": "object", "properties": {
                "query": { "type": "string" },
                "limit": { "type": "integer", "minimum": 1, "maximum": 50 },
                "kind": { "type": "string", "enum": KIND_NAMES },
                "include_closed": { "type": "boolean", "description": "Also return done, retired and replaced items." } },
                "required": ["query"] }),
        ),
        tool(
            "memory_topic",
            "Everything about one project, person or topic. Accepts misheard names.",
            json!({ "type": "object", "properties": { "name": { "type": "string" } }, "required": ["name"] }),
        ),
        tool(
            "memory_context",
            "A context pack for one task, under a token budget, with a note of what was left out.",
            json!({ "type": "object", "properties": {
                "goal": { "type": "string" },
                "budget": { "type": "integer", "minimum": 300, "maximum": 20000 } },
                "required": ["goal"] }),
        ),
        tool(
            "memory_remember",
            "Add one item to memory and rebuild the packs. The repo is public: never store private data.",
            json!({ "type": "object", "properties": {
                "kind": { "type": "string", "enum": KIND_NAMES },
                "text": { "type": "string" },
                "title": { "type": "string", "description": "Name, for projects, people and terms." },
                "why": { "type": "string", "description": "Reason, for decisions." },
                "topics": { "type": "array", "items": { "type": "string" } },
                "aliases": { "type": "array", "items": { "type": "string" }, "description": "How the name gets misheard." },
                "pin": { "type": "boolean" },
                "parked": { "type": "boolean", "description": "Task that needs a screen." } },
                "required": ["kind", "text"] }),
        ),
        tool(
            "memory_complete",
            "Mark a task done or a question answered.",
            json!({ "type": "object", "properties": { "id": { "type": "string" }, "note": { "type": "string" } }, "required": ["id"] }),
        ),
        tool(
            "memory_retire",
            "Retire an item that is no longer true. It stays in the history.",
            json!({ "type": "object", "properties": { "id": { "type": "string" }, "reason": { "type": "string" } }, "required": ["id"] }),
        ),
        tool(
            "memory_ingest_handoff",
            "Save an end-of-session handoff (HANDOFF ... END, English or Spanish keys) into memory.",
            json!({ "type": "object", "properties": {
                "text": { "type": "string" },
                "dry_run": { "type": "boolean", "description": "Show what would change without writing." },
                "source": { "type": "string" } },
                "required": ["text"] }),
        ),
        tool(
            "memory_doctor",
            "Check privacy, instruction injection, staleness, duplicates, budgets, links and drift.",
            json!({ "type": "object", "properties": {} }),
        ),
    ]
}

fn strings(args: &Value, key: &str) -> Vec<String> {
    args.get(key)
        .and_then(Value::as_array)
        .map(|a| a.iter().filter_map(Value::as_str).map(str::to_string).collect())
        .unwrap_or_default()
}

fn call(root: &Path, params: &Value) -> Result<Value, (i64, String)> {
    let name = params
        .get("name")
        .and_then(Value::as_str)
        .ok_or((-32602, "missing tool name".to_string()))?;
    let args = params.get("arguments").cloned().unwrap_or_else(|| json!({}));
    let s = |k: &str| args.get(k).and_then(Value::as_str).map(str::to_string);
    let b = |k: &str| args.get(k).and_then(Value::as_bool).unwrap_or(false);
    let n = |k: &str| args.get(k).and_then(Value::as_u64).map(|v| v as usize);
    let need = |k: &str| s(k).ok_or((-32602, format!("missing argument: {k}")));
    let out: app::Out = match name {
        "memory_resume" => app::resume(root, b("spoken")),
        "memory_search" => app::search_cmd(
            root,
            &need("query")?,
            n("limit").unwrap_or(8).clamp(1, 50),
            s("kind").as_deref(),
            b("include_closed"),
            None,
            false,
        ),
        "memory_topic" => app::topic_cmd(root, &need("name")?),
        "memory_context" => app::context_cmd(root, &need("goal")?, n("budget").unwrap_or(1500).clamp(300, 20_000), None, false),
        "memory_remember" => app::remember(
            root,
            Remember {
                kind: need("kind")?,
                text: need("text")?,
                title: s("title"),
                why: s("why"),
                topics: strings(&args, "topics"),
                aliases: strings(&args, "aliases"),
                pin: b("pin"),
                parked: b("parked"),
                source: Some("mcp".into()),
                ..Remember::default()
            },
            false,
        ),
        "memory_complete" => app::close(root, "done", &need("id")?, s("note"), None, false),
        "memory_retire" => app::close(root, "retire", &need("id")?, s("reason"), None, false),
        "memory_ingest_handoff" => app::ingest_cmd(
            root,
            &need("text")?,
            &s("source").unwrap_or_else(|| "mcp".into()),
            b("dry_run"),
            None,
            false,
            false,
        ),
        "memory_doctor" => app::doctor_cmd(root, false),
        other => return Err((-32602, format!("unknown tool: {other}"))),
    };
    let (text, is_error) = match out {
        Ok(t) => (t, false),
        Err(Failure::Check(t)) | Err(Failure::Error(t)) => (t, true),
    };
    Ok(json!({ "content": [{ "type": "text", "text": text }], "isError": is_error }))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ask(line: &str) -> Value {
        let root = std::env::temp_dir();
        serde_json::from_str(&handle_line(&root, line).expect("a reply")).unwrap()
    }

    #[test]
    fn negotiates_the_protocol_version() {
        let r = ask(r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-06-18"}}"#);
        assert_eq!(r["result"]["protocolVersion"], "2025-06-18");
        assert_eq!(r["result"]["serverInfo"]["name"], "vmem");
        let r = ask(r#"{"jsonrpc":"2.0","id":2,"method":"initialize","params":{"protocolVersion":"1999-01-01"}}"#);
        assert_eq!(r["result"]["protocolVersion"], PROTOCOL_VERSIONS[0]);
    }

    #[test]
    fn lists_tools_and_rejects_unknown_methods() {
        let r = ask(r#"{"jsonrpc":"2.0","id":"a","method":"tools/list"}"#);
        assert_eq!(r["result"]["tools"].as_array().unwrap().len(), 9);
        let r = ask(r#"{"jsonrpc":"2.0","id":3,"method":"nope"}"#);
        assert_eq!(r["error"]["code"], -32601);
        let r = ask("not json");
        assert_eq!(r["error"]["code"], -32700);
    }

    #[test]
    fn notifications_get_no_reply() {
        let root = std::env::temp_dir();
        assert!(handle_line(&root, r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#).is_none());
    }
}
