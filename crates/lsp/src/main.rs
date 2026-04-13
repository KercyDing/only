#[tokio::main]
async fn main() {
    only_lsp::run_stdio().await;
}
