use super::status::get_client_from_state;

pub async fn run() -> anyhow::Result<()> {
    let client = get_client_from_state()?;

    match client.clear().await {
        Ok(data) => {
            println!("All databases cleared.");
            if let Some(obj) = data.get("cleared").and_then(|v| v.as_object()) {
                for (backend, tables) in obj {
                    if let Some(arr) = tables.as_array() {
                        let names: Vec<&str> = arr.iter().filter_map(|v| v.as_str()).collect();
                        println!("  {} — cleared: {}", backend, names.join(", "));
                    }
                }
            }
        }
        Err(e) => {
            eprintln!("Failed to clear: {e}");
            eprintln!("Is the runtime running? Start it with `synapse apply`");
        }
    }

    Ok(())
}
