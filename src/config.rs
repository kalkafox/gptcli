use serde::{Deserialize, Serialize};
use serde_with::serde_as;

#[derive(Debug, Deserialize, Serialize)]
pub struct Config {
    pub openai: OpenAIConfig,
    pub app: AppConfig,
}

#[serde_as]
#[derive(Debug, Deserialize, Serialize)]
pub struct OpenAIConfig {
    pub model: String,
    pub temperature: f32,
    pub top_p: f32,
    pub n: u32,
    pub stop: Option<String>,
    #[serde_as(as = "serde_with::DisplayFromStr")]
    pub max_tokens: f32,
    pub frequency_penalty: f32,
    pub presence_penalty: f32,
    pub logit_bias: Option<String>,
    pub user: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct AppConfig {
    pub prompt: String,
    pub rainbow_speed: f32,
    pub notify_save: bool,
    pub response_prefix: String,
    pub rainbow_delay: u64,
    pub save_conversation: bool,
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
            model: "gpt-3.5-turbo".to_string(),
            temperature: 1.0,
            top_p: 1.0,
            n: 1,
            stop: None,
            max_tokens: std::f32::INFINITY,
            frequency_penalty: 0.0,
            presence_penalty: 0.0,
            logit_bias: None,
            user: None,
        },
        app: AppConfig {
            prompt: "Please wrap code in triple backticks, with the language specified. For example, ```python\nprint('Hello world')\n```".to_string(),
            rainbow_speed: 15.0,
            notify_save: true,
            response_prefix: "GPT-3".to_string(),
            rainbow_delay: 100,
            save_conversation: false,
        },
    };
    save_config(config_dir, &config).await?;
    Ok(config)
}
