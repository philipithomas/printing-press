use std::process::Command;
use tempfile::NamedTempFile;

pub fn render_pdf(html: &str, chromium_path: &str) -> anyhow::Result<Vec<u8>> {
    let input = NamedTempFile::with_suffix(".html")?;
    std::fs::write(input.path(), html)?;

    let output = NamedTempFile::with_suffix(".pdf")?;
    let output_path = output.path().to_string_lossy().to_string();

    let status = Command::new(chromium_path)
        .args([
            "--headless",
            "--disable-gpu",
            "--no-sandbox",
            &format!("--print-to-pdf={}", output_path),
            "--print-to-pdf-no-header",
            "--virtual-time-budget=8000",
            &format!("file://{}", input.path().to_string_lossy()),
        ])
        .output()
        .map_err(|e| anyhow::anyhow!("Failed to run chromium at '{}': {}", chromium_path, e))?;

    if !status.status.success() {
        let stderr = String::from_utf8_lossy(&status.stderr);
        anyhow::bail!("Chromium PDF rendering failed: {}", stderr);
    }

    let pdf_bytes = std::fs::read(&output_path)
        .map_err(|e| anyhow::anyhow!("Failed to read rendered PDF: {}", e))?;

    if pdf_bytes.is_empty() {
        anyhow::bail!("Chromium produced an empty PDF");
    }

    Ok(pdf_bytes)
}

pub fn count_pages(pdf_bytes: &[u8]) -> anyhow::Result<usize> {
    let doc = lopdf::Document::load_mem(pdf_bytes)
        .map_err(|e| anyhow::anyhow!("Failed to parse PDF: {}", e))?;
    Ok(doc.get_pages().len())
}

pub fn find_chromium() -> Option<String> {
    // Check env var first
    if let Ok(path) = std::env::var("CHROMIUM_PATH")
        && !path.is_empty()
        && std::path::Path::new(&path).exists()
    {
        return Some(path);
    }

    // Common paths
    let candidates = [
        "/usr/bin/chromium",
        "/usr/bin/chromium-browser",
        "/usr/bin/google-chrome",
        "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome",
    ];

    candidates
        .iter()
        .find(|p| std::path::Path::new(p).exists())
        .map(|p| p.to_string())
}
