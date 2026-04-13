use std::collections::BTreeMap;
use std::sync::Mutex;

use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::{
    Diagnostic, DiagnosticSeverity, DidChangeTextDocumentParams, DidCloseTextDocumentParams,
    DidOpenTextDocumentParams, DocumentSymbolResponse, FoldingRange,
    FoldingRangeProviderCapability, Hover, HoverContents, HoverParams, InitializeParams,
    InitializeResult, InitializedParams, MarkupContent, MarkupKind, MessageType, OneOf, Position,
    ServerCapabilities, SymbolInformation, SymbolKind, TextDocumentSyncCapability,
    TextDocumentSyncKind, Url,
};
use tower_lsp::{Client, LanguageServer as LanguageServerProtocol, LspService, Server};

use crate::position::{position_to_offset, range_to_lsp_range};
use crate::{
    DocumentSnapshot, LspDiagnostic as HostDiagnostic, LspDiagnosticSeverity,
    LspDocumentSymbolKind, LspHover,
};

pub async fn run_stdio() {
    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();
    let (service, socket) = LspService::new(Backend::new);

    Server::new(stdin, stdout, socket).serve(service).await;
}

struct Backend {
    client: Client,
    documents: Mutex<BTreeMap<String, OpenDocument>>,
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
            documents: Mutex::new(BTreeMap::new()),
        }
    }

    fn apply_open(&self, params: DidOpenTextDocumentParams) {
        let mut documents = self
            .documents
            .lock()
            .expect("document mutex should not panic");
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
            .lock()
            .expect("document mutex should not panic");
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
            .lock()
            .expect("document mutex should not panic");
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
                let protocol_range = range_to_lsp_range(&snapshot.source, range.range);
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
            .lock()
            .expect("document mutex should not panic");
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
                hover_provider: Some(tower_lsp::lsp_types::HoverProviderCapability::Simple(true)),
                document_symbol_provider: Some(OneOf::Left(true)),
                folding_range_provider: Some(FoldingRangeProviderCapability::Simple(true)),
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
    if matches!(hover.kind, crate::LspHoverKind::DocComment) {
        return hover.docs.clone().unwrap_or_default();
    }

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
