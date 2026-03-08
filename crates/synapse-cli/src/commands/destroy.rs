use std::fs;

pub fn run(purge: bool) -> anyhow::Result<()> {
    // Read state
    if let Ok(state) = fs::read_to_string(".synapse/state.json") {
        if let Ok(json) = serde_json::from_str::<serde_json::Value>(&state) {
            if let Some(pid) = json["pid"].as_u64() {
                // Try to stop the process
                #[cfg(unix)]
                {
                    use std::process::Command;
                    let _ = Command::new("kill").arg(pid.to_string()).output();
                    println!("  ✓ Stopped runtime (PID: {pid})");
                }
            }
        }
    }

    if purge {
        // Delete data directory
        if fs::metadata("data").is_ok() {
            fs::remove_dir_all("data")?;
            println!("  ✓ Deleted data directory");
        }
    } else {
        println!("  ✓ Preserved data in ./data/");
    }

    // Clean up state
    let _ = fs::remove_file(".synapse/state.json");

    println!("Runtime destroyed.");
    Ok(())
}
