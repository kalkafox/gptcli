use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
pub struct Config {
    pub openai: OpenAIConfig,
    pub app: AppConfig,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct OpenAIConfig {
    pub max_tokens: u32,
    pub temperature: f32,
    pub top_p: f32,
    pub frequency_penalty: f32,
    pub presence_penalty: f32,
    pub stop: Vec<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct AppConfig {
    pub prompt: String,
    pub rainbow_speed: f32,
    pub notify_save: bool,
    pub response_prefix: String,
    pub rainbow_delay: u64,
}

pub async fn save_config(
    config_dir: &str,
    config: &Config,
) -> Result<(), Box<dyn std::error::Error>> {
    let config_path = format!("{}/config.toml", config_dir);
    let config_file = toml::to_string(&config)?;
    tokio::fs::write(config_path, config_file).await?;
    Ok(())
}

pub async fn create_config(config_dir: &str) -> Result<Config, Box<dyn std::error::Error>> {
    let config = Config {
        openai: OpenAIConfig {
            max_tokens: 64,
            temperature: 0.9,
            top_p: 1.0,
            frequency_penalty: 0.0,
            presence_penalty: 0.0,
            stop: vec!["\r".to_string(), " \r".to_string()],
        },
        app: AppConfig {
            prompt: "Please wrap any generated code in a Markdown code block.".to_string(),
            rainbow_speed: 15.0,
            notify_save: true,
            response_prefix: "GPT-3".to_string(),
            rainbow_delay: 10,
        },
    };
    save_config(config_dir, &config).await?;
    Ok(config)
}
