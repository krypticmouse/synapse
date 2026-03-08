pub fn run(follow: bool, level: Option<&str>) -> anyhow::Result<()> {
    println!("Log viewing requires a running runtime with log persistence.");
    println!(
        "Use `synapse apply` with RUST_LOG={} for runtime logging.",
        level.unwrap_or("info")
    );
    if follow {
        println!("(--follow mode is not yet implemented)");
    }
    Ok(())
}
