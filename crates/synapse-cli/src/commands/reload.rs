use super::status::get_client_from_state;

pub async fn run() -> anyhow::Result<()> {
    let client = get_client_from_state()?;

    println!("Sending reload signal...");
    match client.reload().await {
        Ok(resp) => {
            let msg = resp
                .get("message")
                .and_then(|v| v.as_str())
                .unwrap_or("reloaded");
            println!("  ✓ {msg}");
        }
        Err(e) => {
            println!("  ✗ Reload failed: {e}");
            println!("  Is the runtime running? Start it with `synapse apply`");
        }
    }

    Ok(())
}
