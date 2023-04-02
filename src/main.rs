mod spinners;

use std::{
    collections::HashMap,
    io::stdout,
    ops::Add,
    path::{self, Path},
};

use rand::seq::{IteratorRandom, SliceRandom};
use spinners::{get_spinners, Spinner};
use syntect::easy::HighlightLines;
use syntect::highlighting::{Style as HStyle, ThemeSet};
use syntect::parsing::SyntaxSet;
use syntect::util::{as_24_bit_terminal_escaped, LinesWithEndings};

use crossterm::{
    cursor, execute,
    style::{Color, Stylize},
    terminal::{disable_raw_mode, enable_raw_mode},
};

use directories::ProjectDirs;
use regex::Regex;
use rustyline::{config::Configurer, error::ReadlineError, ColorMode, Editor};
use serde::{Deserialize, Serialize};
use tokio::main;

#[derive(Serialize, Deserialize, Debug)]
struct Message {
    role: String,
    content: String,
}

#[derive(Debug, Deserialize)]
struct ChatCompletion {
    id: String,
    object: String,
    created: u64,
    choices: Vec<Choice>,
    usage: Usage,
}

#[derive(Debug, Deserialize)]
struct Choice {
    index: u32,
    message: Message,
    finish_reason: String,
}

#[derive(Debug, Deserialize)]
struct Usage {
    prompt_tokens: u32,
    completion_tokens: u32,
    total_tokens: u32,
}

const RAINBOW_SPEED: f32 = 15.0;

#[main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let spinners = get_spinners().await?;

    let code_re = Regex::new(r#"```(?P<language>\w+)(?:\r?\n|\r)(?P<code>[\s\S]*?)\r?\n```"#)?;
    let tiny_code_re = Regex::new(r#"`(?P<tinycode>[^`]+)`"#)?;

    let ps = SyntaxSet::load_defaults_newlines();
    let ts = ThemeSet::load_defaults();

    let client = reqwest::Client::new();
    let mut headers = reqwest::header::HeaderMap::new();
    headers.append("Content-Type", "application/json".parse()?);

    let mut config_dir: path::PathBuf = Path::new(".").to_path_buf();
    if let Some(proj_dirs) = ProjectDirs::from("dev", "kalkafox", "gpt-cli") {
        let dir = proj_dirs.config_dir();
        config_dir = dir.to_path_buf();
        tokio::fs::create_dir_all(dir).await?;
    }

    if !config_dir.join("openai.key").exists() {
        loop {
            let openai_key = rpassword::prompt_password("Please enter your OpenAI API key: ")?;

            if openai_key.is_empty() {
                // Move cursor up
                execute!(stdout(), cursor::MoveUp(1))?;
                continue;
            }

            headers.append("Authorization", format!("Bearer {}", openai_key).parse()?);

            let res = client
                .get("https://api.openai.com/v1/models")
                .headers(headers.clone())
                .send()
                .await?;

            if res.status() != 200 {
                println!("Invalid OpenAI API key");
                headers.remove("Authorization");
                continue;
            }

            tokio::fs::write(config_dir.join("openai.key"), openai_key.clone()).await?;

            println!(
                "OpenAI key has been stored in {}. Delete it if you wish.",
                config_dir.join("openai.key").display()
            );

            break;
        }
    } else {
        let openai_key = tokio::fs::read_to_string(config_dir.join("openai.key")).await?;

        headers.append("Authorization", format!("Bearer {}", openai_key).parse()?);
    }

    let mut rl = Editor::<(), _>::new()?;
    #[cfg(windows)]
    {
        rl.set_color_mode(ColorMode::Forced);
        enable_raw_mode()?;
    }

    let mut messages: Vec<Message> = vec![];

    // Prompt (will be put into a config later)
    // TODO: toml config
    messages.push(Message {
        role: "user".to_string(),
        content: "Please wrap any generated code in a Markdown code block.".to_string(),
    });

    let spinner_values = spinners.values().collect::<Vec<&Spinner>>();

    loop {
        let readline = rl.readline(">> ");
        match readline {
            Ok(line) => {
                if line.is_empty() {
                    execute!(stdout(), cursor::MoveUp(1))?;
                    continue;
                }

                println!();

                // Get a random spinner
                let spinner = spinner_values.choose(&mut rand::thread_rng()).unwrap();
                let spinner_frames = spinner.frames.clone();
                let spinner_interval = spinner.interval;

                let rainbow_task = tokio::spawn(async move {
                    let mut i = 0;

                    loop {
                        let r = (i as f32 / RAINBOW_SPEED).sin().powi(2);
                        let g = (i as f32 / RAINBOW_SPEED + 2.0 * std::f32::consts::PI / 3.0)
                            .sin()
                            .powi(2);
                        let b = (i as f32 / RAINBOW_SPEED + 4.0 * std::f32::consts::PI / 3.0)
                            .sin()
                            .powi(2);

                        let color_style = Color::Rgb {
                            r: (r * 255.0) as u8,
                            g: (g * 255.0) as u8,
                            b: (b * 255.0) as u8,
                        };

                        // Get current cursor position
                        let (x, y) = cursor::position().unwrap();

                        // Colorize the current line
                        execute!(stdout(), cursor::MoveToColumn(0)).unwrap();
                        execute!(stdout(), crossterm::style::SetForegroundColor(color_style))
                            .unwrap();

                        // Move cursor back to the original position
                        execute!(stdout(), cursor::MoveTo(x, y)).unwrap();

                        tokio::time::sleep(std::time::Duration::from_millis(10)).await;

                        i = i + 1;
                    }
                });

                let spin_task = tokio::spawn(async move {
                    loop {
                        for frame in spinner_frames.iter() {
                            // Disable cursor
                            execute!(stdout(), cursor::Hide).unwrap();

                            // Print the frame
                            print!("{} ", frame);

                            // Move to the very beginning of the line
                            execute!(stdout(), cursor::MoveToColumn(0)).unwrap();
                            tokio::time::sleep(std::time::Duration::from_millis(
                                spinner_interval.into(),
                            ))
                            .await;
                        }
                    }
                });

                // message_history.push(format!("{}: {}",
                // //"",
                // "You",
                // line));
                messages.push(Message {
                    role: "user".to_string(),
                    content: line,
                });

                let messages_json = serde_json::to_value(&messages).unwrap();
                let body = serde_json::json!({
                    "model": "gpt-3.5-turbo",
                    "messages": messages_json,
                });

                let openai_res = client
                    .post("https://api.openai.com/v1/chat/completions")
                    .headers(headers.clone())
                    .json(&body)
                    .send()
                    .await?;

                let chat_completion = openai_res.json::<ChatCompletion>().await?;

                rainbow_task.abort();
                spin_task.abort();

                let choice = &chat_completion.choices[0];
                let message = &choice.message;

                // message_history.push(format!("{}: {}",
                // //"",
                // "GPT-3",
                // message.content));
                messages.push(Message {
                    role: "assistant".to_string(),
                    content: message.content.clone(),
                });

                let pretty_string =
                    highlight_message(&message.content, &ps, &ts, &code_re, &tiny_code_re);

                // Enable cursor
                execute!(stdout(), cursor::Show).unwrap();
                execute!(stdout(), cursor::MoveLeft(2)).unwrap();

                println!(
                    "{}: {}\n",
                    "GPT-3".stylize().dark_green().bold(),
                    pretty_string
                );
            }
            Err(ReadlineError::Interrupted) => {
                println!();
                println!("Buh-bye!");
                break;
            }
            Err(ReadlineError::Eof) => {
                println!("CTRL-D");
                break;
            }
            Err(err) => {
                println!("Error: {:?}", err);
                break;
            }
        }
    }

    #[cfg(windows)]
    disable_raw_mode()?;

    Ok(())
}

fn highlight_message(
    message: &String,
    ps: &syntect::parsing::SyntaxSet,
    ts: &syntect::highlighting::ThemeSet,
    code_re: &Regex,
    tiny_code_re: &Regex,
) -> String {
    let mut message_mut = message.clone();

    for cap in code_re.captures_iter(message.as_str()) {
        //message_mut = message.replace(format!("```{}", &cap["language"]).as_str(), "");

        let mut language = &cap["language"];

        match &cap["language"] {
            "typescript" => language = "ts",
            "javascript" => language = "js",

            _ => {}
        }

        let syntax = ps.find_syntax_by_token(language);
        if syntax.is_none() {
            continue;
        }

        let syntax = syntax.unwrap();

        let mut h = HighlightLines::new(syntax, &ts.themes["base16-mocha.dark"]);

        let mut code: Vec<String> = vec![];
        for line in LinesWithEndings::from(&cap["code"]) {
            let ranges: Vec<(HStyle, &str)> = h.highlight_line(line, &ps).unwrap();
            let mut escaped = as_24_bit_terminal_escaped(&ranges[..], false);
            escaped.push_str("\x1b[0m");
            code.push(escaped);
        }

        let stylized_code = code.join("\x1b[0m");

        message_mut = message_mut.replace(&cap["code"], &stylized_code);
    }

    message_mut = message_mut.replace("```", "");

    // for cap in tiny_code_re.captures_iter(message_mut.clone().as_str()) {
    //     let bold = &cap["tinycode"].stylize().grey().italic().bold().to_string();
    //     message_mut = message_mut.replace(format!("`{}`", &cap["tinycode"]).as_str(), bold);
    // }

    message_mut
}
