pub async fn run(event: &str, payload_json: &str) -> anyhow::Result<()> {
    let client = crate::commands::status::get_client_from_state()?;

    let payload: serde_json::Value =
        serde_json::from_str(payload_json).unwrap_or(serde_json::json!({}));

    match client.emit(event, payload).await {
        Ok(result) => {
            println!("  ✓ Event '{event}' emitted");
            println!("{}", serde_json::to_string_pretty(&result)?);
        }
        Err(e) => {
            eprintln!("Emit failed: {e}");
        }
    }

    Ok(())
}
