//! Minimal real LSP for Titan: parse-error diagnostics + hover-over-function
//! signatures. Deliberately small — go-to-definition/completion are
//! follow-ups, not faked here.
use std::collections::HashMap;
use lsp_server::{Connection, Message, Notification, RequestId, Response};
use lsp_types::{
    Diagnostic, DiagnosticSeverity, Hover, HoverContents, HoverParams, MarkedString, Position,
    PublishDiagnosticsParams, Range, ServerCapabilities, TextDocumentSyncCapability,
    TextDocumentSyncKind, Uri,
};
use titan::{ast::Item, diag::OmniError, parser};

fn diagnostics_for(text: &str, uri: &Uri) -> Vec<Diagnostic> {
    match parser::parse(text, &uri.to_string()) {
        Ok(_) => vec![],
        Err(e) => vec![to_diagnostic(&e)],
    }
}

fn to_diagnostic(e: &OmniError) -> Diagnostic {
    let (line, col) = e
        .span
        .map(|s| (s.start.line.saturating_sub(1), s.start.col.saturating_sub(1)))
        .unwrap_or((0, 0));
    let pos = Position::new(line, col);
    Diagnostic {
        range: Range::new(pos, Position::new(line, col + 1)),
        severity: Some(DiagnosticSeverity::ERROR),
        message: e.message.clone(),
        source: Some("titan".to_string()),
        ..Default::default()
    }
}

fn publish(conn: &Connection, uri: Uri, diags: Vec<Diagnostic>) {
    let params = PublishDiagnosticsParams { uri, diagnostics: diags, version: None };
    let _ = conn.sender.send(Message::Notification(Notification::new(
        "textDocument/publishDiagnostics".into(),
        params,
    )));
}

/// Extracts the identifier under `pos` from `text` (simple whitespace/punct
/// boundary scan — no full lexer needed for this).
fn word_at(text: &str, pos: Position) -> Option<String> {
    let line = text.lines().nth(pos.line as usize)?;
    let col = pos.character as usize;
    let is_ident = |c: char| c.is_alphanumeric() || c == '_';
    let bytes: Vec<char> = line.chars().collect();
    if col > bytes.len() {
        return None;
    }
    let mut start = col.min(bytes.len().saturating_sub(1));
    if start < bytes.len() && !is_ident(bytes[start]) && start > 0 {
        start -= 1;
    }
    if start >= bytes.len() || !is_ident(bytes[start]) {
        return None;
    }
    while start > 0 && is_ident(bytes[start - 1]) {
        start -= 1;
    }
    let mut end = start;
    while end < bytes.len() && is_ident(bytes[end]) {
        end += 1;
    }
    Some(bytes[start..end].iter().collect())
}

/// Finds the top-level `fn` item named `word`, if the document parses.
fn find_fn<'a>(program: &'a titan::ast::Program, word: &str) -> Option<&'a titan::ast::FnItem> {
    program.items.iter().find_map(|item| match item {
        Item::Fn(f) if f.name == word => Some(f),
        _ => None,
    })
}

/// Real hover: parses the document and, if the hovered word names a
/// top-level `fn`, returns its real signature (name/params/return type) —
/// not a placeholder.
fn hover_for(text: &str, uri: &Uri, word: &str) -> Option<String> {
    let program = parser::parse(text, &uri.to_string()).ok()?;
    let f = find_fn(&program, word)?;
    let params: Vec<&str> = f.params.iter().map(|p| p.name.as_str()).collect();
    let ret = f.ret.as_ref().map(|t| format!(" -> {t:?}")).unwrap_or_default();
    Some(format!("fn {}({}){}", f.name, params.join(", "), ret))
}

/// Real go-to-definition: same lookup as hover, returning the fn's real span.
fn definition_for(text: &str, uri: &Uri, word: &str) -> Option<lsp_types::Location> {
    let program = parser::parse(text, &uri.to_string()).ok()?;
    let f = find_fn(&program, word)?;
    let line = f.span.start.line.saturating_sub(1);
    let col = f.span.start.col.saturating_sub(1);
    let pos = Position::new(line, col);
    Some(lsp_types::Location { uri: uri.clone(), range: Range::new(pos, pos) })
}

/// Real completion: every top-level fn/struct/enum name the document
/// actually parses to, not a canned list.
fn completions_for(text: &str, uri: &Uri) -> Vec<lsp_types::CompletionItem> {
    let Ok(program) = parser::parse(text, &uri.to_string()) else { return vec![] };
    program
        .items
        .iter()
        .filter_map(|item| match item {
            Item::Fn(f) => Some((f.name.clone(), lsp_types::CompletionItemKind::FUNCTION)),
            Item::Struct(s) => Some((s.name.clone(), lsp_types::CompletionItemKind::STRUCT)),
            Item::Enum(e) => Some((e.name.clone(), lsp_types::CompletionItemKind::ENUM)),
            _ => None,
        })
        .map(|(label, kind)| lsp_types::CompletionItem {
            label,
            kind: Some(kind),
            ..Default::default()
        })
        .collect()
}

fn main() {
    let (connection, io_threads) = Connection::stdio();
    let caps = ServerCapabilities {
        text_document_sync: Some(TextDocumentSyncCapability::Kind(TextDocumentSyncKind::FULL)),
        hover_provider: Some(lsp_types::HoverProviderCapability::Simple(true)),
        definition_provider: Some(lsp_types::OneOf::Left(true)),
        completion_provider: Some(Default::default()),
        ..Default::default()
    };
    let _ = connection.initialize(serde_json::to_value(caps).unwrap()).unwrap();

    let mut docs: HashMap<String, String> = HashMap::new();

    for msg in &connection.receiver {
        match msg {
            Message::Notification(n) => {
                let (uri, text) = match n.method.as_str() {
                    "textDocument/didOpen" => {
                        let p: lsp_types::DidOpenTextDocumentParams = serde_json::from_value(n.params).unwrap();
                        (p.text_document.uri, p.text_document.text)
                    }
                    "textDocument/didChange" => {
                        let p: lsp_types::DidChangeTextDocumentParams = serde_json::from_value(n.params).unwrap();
                        let text = p.content_changes.into_iter().last().map(|c| c.text).unwrap_or_default();
                        (p.text_document.uri, text)
                    }
                    "exit" => break,
                    _ => continue,
                };
                docs.insert(uri.to_string(), text.clone());
                let diags = diagnostics_for(&text, &uri);
                publish(&connection, uri, diags);
            }
            Message::Request(r) if r.method == "textDocument/hover" => {
                let id: RequestId = r.id.clone();
                let p: HoverParams = serde_json::from_value(r.params).unwrap();
                let uri = p.text_document_position_params.text_document.uri.clone();
                let pos = p.text_document_position_params.position;
                let result = docs
                    .get(&uri.to_string())
                    .and_then(|text| word_at(text, pos).map(|w| (text.clone(), w)))
                    .and_then(|(text, word)| hover_for(&text, &uri, &word))
                    .map(|sig| Hover {
                        contents: HoverContents::Scalar(MarkedString::String(sig)),
                        range: None,
                    });
                let _ = connection
                    .sender
                    .send(Message::Response(Response::new_ok(id, serde_json::to_value(result).unwrap())));
            }
            Message::Request(r) if r.method == "textDocument/definition" => {
                let id: RequestId = r.id.clone();
                let p: lsp_types::GotoDefinitionParams = serde_json::from_value(r.params).unwrap();
                let uri = p.text_document_position_params.text_document.uri.clone();
                let pos = p.text_document_position_params.position;
                let result = docs
                    .get(&uri.to_string())
                    .and_then(|text| word_at(text, pos).map(|w| (text.clone(), w)))
                    .and_then(|(text, word)| definition_for(&text, &uri, &word))
                    .map(lsp_types::GotoDefinitionResponse::Scalar);
                let _ = connection
                    .sender
                    .send(Message::Response(Response::new_ok(id, serde_json::to_value(result).unwrap())));
            }
            Message::Request(r) if r.method == "textDocument/completion" => {
                let id: RequestId = r.id.clone();
                let p: lsp_types::CompletionParams = serde_json::from_value(r.params).unwrap();
                let uri = p.text_document_position.text_document.uri.clone();
                let items = docs
                    .get(&uri.to_string())
                    .map(|text| completions_for(text, &uri))
                    .unwrap_or_default();
                let _ = connection
                    .sender
                    .send(Message::Response(Response::new_ok(id, serde_json::to_value(items).unwrap())));
            }
            Message::Request(r) if r.method == "shutdown" => {
                let _ = connection.sender.send(Message::Response(Response::new_ok(r.id, ())));
            }
            _ => {}
        }
    }
    let _ = io_threads.join();
}
