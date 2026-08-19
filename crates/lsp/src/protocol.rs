use std::collections::BTreeMap;
use std::sync::RwLock;

use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::{
    CompletionItem, CompletionItemKind, CompletionOptions, CompletionResponse, CompletionTextEdit,
    Diagnostic, DiagnosticSeverity, DidChangeTextDocumentParams, DidCloseTextDocumentParams,
    DidOpenTextDocumentParams, DocumentFormattingParams, DocumentRangeFormattingParams,
    DocumentSymbolResponse, FoldingRange, FoldingRangeProviderCapability, Hover, HoverContents,
    HoverParams, InitializeParams, InitializeResult, InitializedParams, InsertTextFormat,
    MarkupContent, MarkupKind, MessageType, OneOf, Position, SemanticToken, SemanticTokenType,
    SemanticTokens, SemanticTokensFullOptions, SemanticTokensLegend, SemanticTokensParams,
    SemanticTokensResult, SemanticTokensServerCapabilities, ServerCapabilities, SymbolInformation,
    SymbolKind, TextDocumentSyncCapability, TextDocumentSyncKind, TextEdit, Url,
};
use tower_lsp::{Client, LanguageServer as LanguageServerProtocol, LspService, Server};

use crate::position::{folding_range_to_lsp_range, position_to_offset, range_to_lsp_range};
use crate::{
    DocumentSnapshot, LspCompletionKind, LspDiagnostic as HostDiagnostic, LspDiagnosticSeverity,
    LspDocumentSymbolKind, LspHover, LspSemanticTokenKind, completions, semantic_tokens,
};

pub async fn run_stdio() {
    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();
    let (service, socket) = LspService::new(Backend::new);

    Server::new(stdin, stdout, socket).serve(service).await;
}

struct Backend {
    client: Client,
    documents: RwLock<BTreeMap<String, OpenDocument>>,
}

#[derive(Debug, Clone)]
struct OpenDocument {
    version: i32,
    source: String,
}

impl Backend {
    fn new(client: Client) -> Self {
        Self {
            client,
            documents: RwLock::new(BTreeMap::new()),
        }
    }

    fn apply_open(&self, params: DidOpenTextDocumentParams) {
        let mut documents = self
            .documents
            .write()
            .expect("document lock should not panic");
        documents.insert(
            params.text_document.uri.to_string(),
            OpenDocument {
                version: params.text_document.version,
                source: params.text_document.text,
            },
        );
    }

    fn apply_change(&self, params: DidChangeTextDocumentParams) {
        let DidChangeTextDocumentParams {
            text_document,
            content_changes,
        } = params;
        let Some(change) = content_changes.last().map(|change| change.text.clone()) else {
            return;
        };

        let mut documents = self
            .documents
            .write()
            .expect("document lock should not panic");
        documents.insert(
            text_document.uri.to_string(),
            OpenDocument {
                version: text_document.version,
                source: change,
            },
        );
    }

    fn apply_close(&self, params: DidCloseTextDocumentParams) {
        let mut documents = self
            .documents
            .write()
            .expect("document lock should not panic");
        documents.remove(params.text_document.uri.as_str());
    }

    fn diagnostics_for_uri(&self, uri: &Url) -> Vec<Diagnostic> {
        let Some(snapshot) = self.snapshot_for_uri(uri) else {
            return Vec::new();
        };

        crate::diagnostics(&snapshot)
            .into_iter()
            .map(|diagnostic| host_diagnostic_to_protocol(&snapshot.source, diagnostic))
            .collect()
    }

    fn hover_for_uri(&self, uri: &Url, position: Position) -> Option<Hover> {
        let snapshot = self.snapshot_for_uri(uri)?;
        let offset = position_to_offset(&snapshot.source, position);
        let hover = crate::hover(&snapshot, offset)?;

        Some(Hover {
            contents: HoverContents::Markup(MarkupContent {
                kind: MarkupKind::Markdown,
                value: hover_markdown(&hover),
            }),
            range: Some(range_to_lsp_range(&snapshot.source, hover.range)),
        })
    }

    fn completion_for_uri(&self, uri: &Url, position: Position) -> Option<CompletionResponse> {
        let snapshot = self.snapshot_for_uri(uri)?;
        let offset = position_to_offset(&snapshot.source, position);
        let items = completions(&snapshot.source, offset)
            .into_iter()
            .map(|item| CompletionItem {
                label: item.label,
                kind: Some(match item.kind {
                    LspCompletionKind::Directive => CompletionItemKind::KEYWORD,
                    LspCompletionKind::Keyword => CompletionItemKind::KEYWORD,
                    LspCompletionKind::Guard => CompletionItemKind::FUNCTION,
                    LspCompletionKind::Metadata => CompletionItemKind::PROPERTY,
                    LspCompletionKind::Task => CompletionItemKind::FUNCTION,
                    LspCompletionKind::Group => CompletionItemKind::MODULE,
                }),
                detail: Some(item.detail),
                insert_text: Some(item.insert_text.clone()),
                insert_text_format: Some(InsertTextFormat::SNIPPET),
                text_edit: Some(CompletionTextEdit::Edit(TextEdit {
                    range: range_to_lsp_range(&snapshot.source, item.replace_range),
                    new_text: item.insert_text,
                })),
                ..CompletionItem::default()
            })
            .collect();
        Some(CompletionResponse::Array(items))
    }

    fn symbols_for_uri(&self, uri: &Url) -> Vec<SymbolInformation> {
        let Some(snapshot) = self.snapshot_for_uri(uri) else {
            return Vec::new();
        };

        crate::symbols(&snapshot)
            .into_iter()
            .map(|symbol| symbol_to_information(uri, &snapshot.source, symbol))
            .collect()
    }

    fn folding_ranges_for_uri(&self, uri: &Url) -> Vec<FoldingRange> {
        let Some(snapshot) = self.snapshot_for_uri(uri) else {
            return Vec::new();
        };

        crate::folding_ranges(&snapshot)
            .into_iter()
            .map(|range| {
                let protocol_range = folding_range_to_lsp_range(&snapshot.source, range.range);
                FoldingRange {
                    start_line: protocol_range.start.line,
                    start_character: Some(protocol_range.start.character),
                    end_line: protocol_range.end.line,
                    end_character: Some(protocol_range.end.character),
                    kind: None,
                    collapsed_text: None,
                }
            })
            .collect()
    }

    fn snapshot_for_uri(&self, uri: &Url) -> Option<DocumentSnapshot> {
        let documents = self
            .documents
            .read()
            .expect("document lock should not panic");
        let document = documents.get(uri.as_str())?;
        Some(DocumentSnapshot::new(
            uri.as_str(),
            document.version,
            &document.source,
        ))
    }

    async fn publish_diagnostics(&self, uri: Url, version: Option<i32>) {
        let diagnostics = self.diagnostics_for_uri(&uri);
        self.client
            .publish_diagnostics(uri, diagnostics, version)
            .await;
    }
}

#[tower_lsp::async_trait]
impl LanguageServerProtocol for Backend {
    async fn initialize(&self, _: InitializeParams) -> Result<InitializeResult> {
        Ok(InitializeResult {
            server_info: None,
            capabilities: ServerCapabilities {
                text_document_sync: Some(TextDocumentSyncCapability::Kind(
                    TextDocumentSyncKind::FULL,
                )),
                completion_provider: Some(CompletionOptions {
                    trigger_characters: Some(vec![
                        "!".to_string(),
                        "@".to_string(),
                        "[".to_string(),
                        "&".to_string(),
                        ".".to_string(),
                    ]),
                    ..CompletionOptions::default()
                }),
                hover_provider: Some(tower_lsp::lsp_types::HoverProviderCapability::Simple(true)),
                document_symbol_provider: Some(OneOf::Left(true)),
                folding_range_provider: Some(FoldingRangeProviderCapability::Simple(true)),
                document_formatting_provider: Some(OneOf::Left(true)),
                document_range_formatting_provider: Some(OneOf::Left(true)),
                semantic_tokens_provider: Some(
                    SemanticTokensServerCapabilities::SemanticTokensOptions(
                        tower_lsp::lsp_types::SemanticTokensOptions {
                            work_done_progress_options: Default::default(),
                            legend: SemanticTokensLegend {
                                token_types: vec![
                                    SemanticTokenType::new("directive"),
                                    SemanticTokenType::new("namespace"),
                                    SemanticTokenType::new("task"),
                                    SemanticTokenType::new("parameter"),
                                    SemanticTokenType::new("guard"),
                                    SemanticTokenType::new("dependency"),
                                    SemanticTokenType::new("shell"),
                                    SemanticTokenType::new("variable"),
                                    SemanticTokenType::new("metadata"),
                                    SemanticTokenType::new("blockMarker"),
                                    SemanticTokenType::new("delimiter"),
                                ],
                                token_modifiers: Vec::new(),
                            },
                            range: None,
                            full: Some(SemanticTokensFullOptions::Bool(true)),
                        },
                    ),
                ),
                ..ServerCapabilities::default()
            },
        })
    }

    async fn initialized(&self, _: InitializedParams) {
        self.client
            .log_message(MessageType::INFO, "Onlyfile language server initialized.")
            .await;
    }

    async fn shutdown(&self) -> Result<()> {
        Ok(())
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        let uri = params.text_document.uri.clone();
        let version = Some(params.text_document.version);
        self.apply_open(params);
        self.publish_diagnostics(uri, version).await;
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        let uri = params.text_document.uri.clone();
        let version = Some(params.text_document.version);
        self.apply_change(params);
        self.publish_diagnostics(uri, version).await;
    }

    async fn did_close(&self, params: DidCloseTextDocumentParams) {
        let uri = params.text_document.uri.clone();
        self.apply_close(params);
        self.publish_diagnostics(uri, None).await;
    }

    async fn hover(&self, params: HoverParams) -> Result<Option<Hover>> {
        Ok(self.hover_for_uri(
            &params.text_document_position_params.text_document.uri,
            params.text_document_position_params.position,
        ))
    }

    async fn completion(
        &self,
        params: tower_lsp::lsp_types::CompletionParams,
    ) -> Result<Option<CompletionResponse>> {
        Ok(self.completion_for_uri(
            &params.text_document_position.text_document.uri,
            params.text_document_position.position,
        ))
    }

    async fn document_symbol(
        &self,
        params: tower_lsp::lsp_types::DocumentSymbolParams,
    ) -> Result<Option<DocumentSymbolResponse>> {
        Ok(Some(DocumentSymbolResponse::Flat(
            self.symbols_for_uri(&params.text_document.uri),
        )))
    }

    async fn folding_range(
        &self,
        params: tower_lsp::lsp_types::FoldingRangeParams,
    ) -> Result<Option<Vec<FoldingRange>>> {
        Ok(Some(self.folding_ranges_for_uri(&params.text_document.uri)))
    }

    async fn formatting(&self, params: DocumentFormattingParams) -> Result<Option<Vec<TextEdit>>> {
        let Some(snapshot) = self.snapshot_for_uri(&params.text_document.uri) else {
            return Ok(None);
        };
        if snapshot
            .semantic
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.severity == only_diagnostic::DiagnosticSeverity::Error)
        {
            return Ok(None);
        }
        let Ok(formatted) = only_syntax::format_source(&snapshot.source) else {
            return Ok(None);
        };
        if formatted == snapshot.source {
            return Ok(Some(Vec::new()));
        }
        Ok(Some(vec![TextEdit {
            range: range_to_lsp_range(
                &snapshot.source,
                text_size::TextRange::up_to(text_size::TextSize::of(snapshot.source.as_str())),
            ),
            new_text: formatted,
        }]))
    }

    async fn range_formatting(
        &self,
        params: DocumentRangeFormattingParams,
    ) -> Result<Option<Vec<TextEdit>>> {
        let Some(snapshot) = self.snapshot_for_uri(&params.text_document.uri) else {
            return Ok(None);
        };
        if snapshot
            .semantic
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.severity == only_diagnostic::DiagnosticSeverity::Error)
        {
            return Ok(None);
        }

        let range = text_size::TextRange::new(
            position_to_offset(&snapshot.source, params.range.start),
            position_to_offset(&snapshot.source, params.range.end),
        );
        let Ok(Some((formatted_range, formatted))) =
            only_syntax::format_range(&snapshot.source, range)
        else {
            return Ok(None);
        };
        if formatted
            == snapshot.source
                [usize::from(formatted_range.start())..usize::from(formatted_range.end())]
        {
            return Ok(Some(Vec::new()));
        }
        Ok(Some(vec![TextEdit {
            range: range_to_lsp_range(&snapshot.source, formatted_range),
            new_text: formatted,
        }]))
    }

    async fn semantic_tokens_full(
        &self,
        params: SemanticTokensParams,
    ) -> Result<Option<SemanticTokensResult>> {
        let Some(snapshot) = self.snapshot_for_uri(&params.text_document.uri) else {
            return Ok(None);
        };
        let mut previous_line = 0u32;
        let mut previous_character = 0u32;
        let mut data = Vec::new();
        for token in semantic_tokens(&snapshot) {
            let protocol_range = range_to_lsp_range(&snapshot.source, token.range);
            let start = protocol_range.start;
            let token_type = match token.kind {
                LspSemanticTokenKind::Directive => 0,
                LspSemanticTokenKind::Namespace => 1,
                LspSemanticTokenKind::Task => 2,
                LspSemanticTokenKind::Parameter => 3,
                LspSemanticTokenKind::Guard => 4,
                LspSemanticTokenKind::Dependency => 5,
                LspSemanticTokenKind::Shell => 6,
                LspSemanticTokenKind::Variable => 7,
                LspSemanticTokenKind::Metadata => 8,
                LspSemanticTokenKind::BlockMarker => 9,
                LspSemanticTokenKind::Delimiter => 10,
            };
            data.push(SemanticToken {
                delta_line: start.line - previous_line,
                delta_start: if start.line == previous_line {
                    start.character - previous_character
                } else {
                    start.character
                },
                length: protocol_range.end.character - start.character,
                token_type,
                token_modifiers_bitset: 0,
            });
            previous_line = start.line;
            previous_character = start.character;
        }
        Ok(Some(SemanticTokensResult::Tokens(SemanticTokens {
            result_id: None,
            data,
        })))
    }
}

fn host_diagnostic_to_protocol(source: &str, diagnostic: HostDiagnostic) -> Diagnostic {
    Diagnostic {
        range: range_to_lsp_range(source, diagnostic.range),
        severity: Some(match diagnostic.severity {
            LspDiagnosticSeverity::Error => DiagnosticSeverity::ERROR,
            LspDiagnosticSeverity::Warning => DiagnosticSeverity::WARNING,
            LspDiagnosticSeverity::Info => DiagnosticSeverity::INFORMATION,
            LspDiagnosticSeverity::Hint => DiagnosticSeverity::HINT,
        }),
        code: Some(tower_lsp::lsp_types::NumberOrString::String(
            diagnostic.code,
        )),
        code_description: None,
        source: Some("only-lsp".to_string()),
        message: diagnostic.message,
        related_information: None,
        tags: None,
        data: None,
    }
}

fn hover_markdown(hover: &LspHover) -> String {
    let mut sections = vec![format!("```onlyfile\n{}\n```", hover.signature)];

    if let Some(docs) = &hover.docs {
        sections.push(docs.clone());
    }

    if let Some(container) = &hover.container_name {
        sections.push(format!("Container: `{container}`"));
    }

    sections.join("\n\n")
}

#[allow(deprecated)]
fn symbol_to_information(
    uri: &Url,
    source: &str,
    symbol: crate::LspDocumentSymbol,
) -> SymbolInformation {
    SymbolInformation {
        name: symbol.name,
        kind: match symbol.kind {
            LspDocumentSymbolKind::Namespace => SymbolKind::NAMESPACE,
            LspDocumentSymbolKind::Task => SymbolKind::FUNCTION,
        },
        tags: None,
        deprecated: None,
        location: tower_lsp::lsp_types::Location {
            uri: uri.clone(),
            range: range_to_lsp_range(source, symbol.range),
        },
        container_name: symbol.container_name,
    }
}
