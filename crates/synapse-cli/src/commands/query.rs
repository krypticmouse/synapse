pub async fn run(query_name: &str, params_json: &str) -> anyhow::Result<()> {
    let client = crate::commands::status::get_client_from_state()?;

    let params: serde_json::Value = serde_json::from_str(params_json)
        .unwrap_or(serde_json::json!({}));

    match client.query(query_name, params).await {
        Ok(results) => {
            println!("{}", serde_json::to_string_pretty(&results)?);
        }
        Err(e) => {
            eprintln!("Query failed: {e}");
        }
    }

    Ok(())
}
