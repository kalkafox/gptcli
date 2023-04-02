use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Serialize, Deserialize)]
pub struct Spinner {
    pub interval: u32,
    pub frames: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Spinners {
    // The key is not known at compile time, the format is {"dots": {...}, "dots2": {...}, ...}
    #[serde(flatten)]
    pub spinners: HashMap<String, Spinner>,
}

pub async fn get_spinners() -> Result<HashMap<String, Spinner>, Box<dyn std::error::Error>> {
    let spinner_res = reqwest::get(
        "https://raw.githubusercontent.com/sindresorhus/cli-spinners/master/spinners.json",
    )
    .await?;
    let spinners: Spinners = spinner_res.json().await?;

    Ok(spinners.spinners)
}
