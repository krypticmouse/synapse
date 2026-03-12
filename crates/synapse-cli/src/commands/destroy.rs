use std::fs;

pub fn run(purge: bool) -> anyhow::Result<()> {
    if let Ok(state) = fs::read_to_string(".synapse/state.json") {
        if let Ok(json) = serde_json::from_str::<serde_json::Value>(&state) {
            if let Some(pid) = json["pid"].as_u64() {
                #[cfg(unix)]
                {
                    use std::process::Command;
                    let _ = Command::new("kill").arg(pid.to_string()).output();
                    println!("  ✓ Stopped runtime (PID: {pid})");
                }
            }
        }
    } else {
        println!("  No running runtime found.");
    }

    if purge {
        let dirs_to_purge = [".synapse", "data"];
        let globs_to_purge = ["*.db", "*.db-wal", "*.db-shm"];

        for dir in &dirs_to_purge {
            if fs::metadata(dir).is_ok() {
                fs::remove_dir_all(dir)?;
                println!("  ✓ Deleted {dir}/");
            }
        }

        for pattern in &globs_to_purge {
            if let Ok(entries) = fs::read_dir(".") {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                        let suffix = pattern.trim_start_matches('*');
                        if name.ends_with(suffix) {
                            fs::remove_file(&path)?;
                            println!("  ✓ Deleted {}", path.display());
                        }
                    }
                }
            }
        }
        println!("  ✓ All data purged.");
    } else {
        let _ = fs::remove_file(".synapse/state.json");
        println!("  ✓ Preserved data files.");
    }

    println!("Runtime destroyed.");
    Ok(())
}
