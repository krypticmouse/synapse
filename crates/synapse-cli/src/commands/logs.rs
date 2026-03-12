use std::io::{BufRead, BufReader, Seek, SeekFrom};
use std::path::Path;

const LOG_FILE: &str = ".synapse/runtime.log";

pub fn run(follow: bool, level: Option<&str>) -> anyhow::Result<()> {
    let log_path = Path::new(LOG_FILE);

    if !log_path.exists() {
        println!("No log file found at {LOG_FILE}");
        println!("Start the runtime with `synapse apply` to generate logs.");
        return Ok(());
    }

    let level_filter = level.unwrap_or("info");

    let file = std::fs::File::open(log_path)?;
    let reader = BufReader::new(file);

    for line in reader.lines() {
        let line = line?;
        if should_show_line(&line, level_filter) {
            println!("{line}");
        }
    }

    if follow {
        println!("--- following {LOG_FILE} (Ctrl+C to stop) ---");
        let mut file = std::fs::File::open(log_path)?;
        file.seek(SeekFrom::End(0))?;

        loop {
            let mut buf = String::new();
            let mut reader = BufReader::new(&file);
            while reader.read_line(&mut buf)? > 0 {
                let line = buf.trim_end();
                if should_show_line(line, level_filter) {
                    println!("{line}");
                }
                buf.clear();
            }
            std::thread::sleep(std::time::Duration::from_millis(500));
        }
    }

    Ok(())
}

fn should_show_line(line: &str, level_filter: &str) -> bool {
    let levels = ["trace", "debug", "info", "warn", "error"];
    let filter_idx = levels.iter().position(|&l| l == level_filter).unwrap_or(2);

    for (i, &lvl) in levels.iter().enumerate() {
        let upper = lvl.to_uppercase();
        if line.contains(&upper) || line.contains(lvl) {
            return i >= filter_idx;
        }
    }
    true
}
