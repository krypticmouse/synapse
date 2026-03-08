pub async fn run() -> anyhow::Result<()> {
    let client = get_client_from_state()?;

    match client.status().await {
        Ok(status) => {
            println!("Synapse Runtime Status");
            println!("======================");
            println!("Status:     {}", status.status);
            println!("Uptime:     {}s", status.uptime_secs);
            println!();
            println!("Handlers:   {:?}", status.handlers);
            println!("Queries:    {:?}", status.queries);
            println!("Memories:   {:?}", status.memories);
        }
        Err(e) => {
            println!("Runtime not reachable: {e}");
            println!("Is the runtime running? Start it with `synapse apply`");
        }
    }

    Ok(())
}

pub fn get_client_from_state() -> anyhow::Result<synapse_sdk::Client> {
    // Try to read state file for the address
    if let Ok(state) = std::fs::read_to_string(".synapse/state.json") {
        if let Ok(json) = serde_json::from_str::<serde_json::Value>(&state) {
            if let Some(addr) = json["addr"].as_str() {
                return Ok(synapse_sdk::Client::new(&format!("http://{addr}")));
            }
        }
    }
    Ok(synapse_sdk::Client::new("http://localhost:8080"))
}
