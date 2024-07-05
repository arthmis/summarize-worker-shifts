mod employee_shift;
use employee_shift::summarize_shifts::summarize_shifts_from_json_file;
use std::{env, io::Write, path::PathBuf, str::FromStr};

use anyhow::Error;

fn main() -> Result<(), Error> {
    let args: Vec<String> = env::args().collect();
    let file_path = args[1].as_str();

    let path = PathBuf::from_str(file_path)?;
    let summaries = summarize_shifts_from_json_file(&path)?;

    let mut file = std::fs::File::create("./employee_summaries.json")?;
    file.write_all(serde_json::to_string_pretty(&summaries)?.as_bytes())?;

    Ok(())
}
