use super::status::get_client_from_state;

pub async fn run(backend: Option<&str>, memory: Option<&str>, compact: bool) -> anyhow::Result<()> {
    let client = get_client_from_state()?;

    match client.inspect().await {
        Ok(data) => {
            let obj = data.as_object().cloned().unwrap_or_default();

            let backends: Vec<(&str, &[&str])> = vec![
                ("sqlite", &["sqlite"]),
                ("qdrant", &["qdrant", "vector"]),
                ("neo4j", &["neo4j", "graph"]),
            ];

            for (key, aliases) in &backends {
                if let Some(filter) = backend {
                    let filter_lower = filter.to_lowercase();
                    if !aliases.iter().any(|a| *a == filter_lower) {
                        continue;
                    }
                }

                let section = match obj.get(*key) {
                    Some(v) => v,
                    None => continue,
                };

                if section.is_string() {
                    println!("{}: {}", key.to_uppercase(), section.as_str().unwrap());
                    println!();
                    continue;
                }

                let tables = match section.as_object() {
                    Some(t) => t,
                    None => continue,
                };

                println!("╔══════════════════════════════════════");
                println!("║  {} ", key.to_uppercase());
                println!("╚══════════════════════════════════════");

                for (table_name, table_data) in tables {
                    if let Some(mem_filter) = memory {
                        if !table_name.eq_ignore_ascii_case(mem_filter) {
                            continue;
                        }
                    }

                    let count = table_data
                        .get("count")
                        .and_then(|v| v.as_u64())
                        .unwrap_or(0);

                    if let Some(err) = table_data.get("error") {
                        println!("  ├─ {} (error: {})", table_name, err);
                        continue;
                    }

                    println!("  ├─ {} ({} records)", table_name, count);

                    if compact || count == 0 {
                        continue;
                    }

                    if let Some(records) = table_data.get("records").and_then(|v| v.as_array()) {
                        for (i, rec) in records.iter().enumerate() {
                            let prefix = if i + 1 == records.len() {
                                "  │  └─"
                            } else {
                                "  │  ├─"
                            };
                            if let Some(obj) = rec.as_object() {
                                let id = obj.get("id").and_then(|v| v.as_str()).unwrap_or("?");
                                let fields: Vec<String> = obj
                                    .iter()
                                    .filter(|(k, _)| *k != "id" && *k != "__type")
                                    .map(|(k, v)| {
                                        let val_str = match v {
                                            serde_json::Value::String(s) => {
                                                if s.len() > 60 {
                                                    format!("\"{}...\"", &s[..57])
                                                } else {
                                                    format!("\"{}\"", s)
                                                }
                                            }
                                            other => other.to_string(),
                                        };
                                        format!("{}={}", k, val_str)
                                    })
                                    .collect();
                                println!("{} [{}] {}", prefix, id, fields.join(", "));
                            } else {
                                println!("{} {}", prefix, rec);
                            }
                        }
                    }
                }
                println!();
            }
        }
        Err(e) => {
            eprintln!("Failed to inspect runtime: {e}");
            eprintln!("Is the runtime running? Start it with `synapse apply`");
        }
    }

    Ok(())
}
