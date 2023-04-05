mod config;
mod spinners;

use std::{
    collections::HashMap,
    io::{stdout, Write},
    panic,
    path::{self, Path},
    sync::{Arc, Mutex},
};

use rand::seq::SliceRandom;
use spinners::{get_spinners, Spinner};
use syntect::highlighting::{Style as HStyle, ThemeSet};
use syntect::parsing::SyntaxSet;
use syntect::util::{as_24_bit_terminal_escaped, LinesWithEndings};
use syntect::{dumps::from_uncompressed_data, easy::HighlightLines};

use crossterm::{
    cursor, execute,
    style::{self, Color, Stylize},
    terminal::{disable_raw_mode, enable_raw_mode, Clear, ClearType},
};

use directories::ProjectDirs;
use regex::Regex;
use rustyline::{config::Configurer, error::ReadlineError, ColorMode, Editor};
use serde::{Deserialize, Serialize};
use tokio::{io::AsyncWriteExt, main};

#[derive(Serialize, Deserialize, Debug, Clone)]
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

#[main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let code_re = Regex::new(r#"```(?P<language>\w+)(?:\r?\n|\r)(?P<code>[\s\S]*?)\r?\n```"#)?;
    let tiny_code_re = Regex::new(r#"`(?P<tinycode>[^`]+)`"#)?;

    let args = std::env::args().collect::<Vec<String>>();

    let ps: SyntaxSet =
        from_uncompressed_data(include_bytes!("../assets/default_newlines.packdump"))?;
    let ts = ThemeSet::load_defaults();

    let client = reqwest::Client::new();
    let mut headers = reqwest::header::HeaderMap::new();
    headers.append("Content-Type", "application/json".parse()?);

    let mut config_dir: path::PathBuf = Path::new(".").to_path_buf();
    let mut data_dir: path::PathBuf = Path::new(".").to_path_buf();

    if let Some(proj_dirs) = ProjectDirs::from("dev", "kalkafox", "gpt-cli") {
        let conf_dir = proj_dirs.config_dir();
        let dat_dir = proj_dirs.data_dir();
        config_dir = conf_dir.to_path_buf();
        data_dir = dat_dir.to_path_buf();
        tokio::fs::create_dir_all(&conf_dir).await?;
        tokio::fs::create_dir_all(&data_dir).await?;
    }

    let mut config = {
        if !config_dir.join("config.toml").exists() {
            let config = config::create_config(config_dir.to_str().unwrap()).await?;
            tokio::fs::write(config_dir.join("config.toml"), toml::to_string(&config)?).await?;
            config
        } else {
            toml::from_str(&tokio::fs::read_to_string(config_dir.join("config.toml")).await?)?
        }
    };

    if !config_dir.join("spinners.json").exists() {
        let spinners = get_spinners().await?;
        tokio::fs::write(
            config_dir.join("spinners.json"),
            serde_json::to_string(&spinners)?,
        )
        .await?;
    }

    let spinners: HashMap<String, Spinner> =
        serde_json::from_str(&tokio::fs::read_to_string(config_dir.join("spinners.json")).await?)?;

    if !config_dir.join("openai.key").exists() {
        loop {
            let openai_key = dialoguer::Password::new()
                .with_prompt("Enter your OpenAI API key")
                .interact()?;

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

            if !config.app.notify_save {
                break;
            }

            let save_confirm = dialoguer::Confirm::new()
                .with_prompt("Save OpenAI API key?")
                .interact()?;

            if !save_confirm {
                let save_confirm = dialoguer::Confirm::new()
                    .with_prompt(format!(
                        "Ask again next time? (you can change this in {}/config.toml)",
                        config_dir.display()
                    ))
                    .interact()?;
                if !save_confirm {
                    break;
                }

                config.app.notify_save = false;

                break;
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

    let data_dir_c = data_dir.clone();

    panic::set_hook(Box::new(move |panic_info| {
        let mut file = std::fs::File::create(data_dir_c.join("panic.log")).unwrap();
        writeln!(file, "{:?}", panic_info).unwrap();

        println!(
            "Panic info has been saved to {}/panic.log",
            data_dir_c.display()
        );
    }));

    messages.push(Message {
        role: "user".to_string(),
        content: config.app.prompt.clone(),
    });

    let spinner_values = spinners.values().collect::<Vec<&Spinner>>();

    if args.len() > 1 {
        let mut args = args;
        args.remove(0);

        if args[0] != "c" {
            let line = args.join(" ");
            rl.add_history_entry(line.as_str())?;
            chat_completion(
                &client,
                &headers,
                &mut messages,
                &spinner_values,
                &config,
                &line,
                &ps,
                &ts,
                &code_re,
                &tiny_code_re,
            )
            .await?;
        }
    }

    println!("To clear the conversation history, type /clear");

    loop {
        let readline = rl.readline(">> ");
        match readline {
            Ok(line) => {
                if line.is_empty() {
                    execute!(stdout(), cursor::MoveUp(1))?;
                    continue;
                }

                if line.starts_with("/") {
                    match line.as_str() {
                        "/clear" => {
                            messages.clear();
                            messages.push(Message {
                                role: "user".to_string(),
                                content: config.app.prompt.clone(),
                            });

                            println!("Conversation history has been cleared");
                        }
                        "/prompt" => {
                            let prompt = dialoguer::Input::new()
                                .with_prompt("Enter new prompt")
                                .interact()?;
                            config.app.prompt = prompt;
                            messages.clear();
                            messages.push(Message {
                                role: "user".to_string(),
                                content: config.app.prompt.clone(),
                            });
                        }
                        "/save" => {
                            let save_confirm = dialoguer::Confirm::new()
                                .with_prompt("Save config?")
                                .interact()?;
                            if save_confirm {
                                config::save_config(
                                    config_dir.display().to_string().as_str(),
                                    &config,
                                )
                                .await?;
                            }
                        }
                        "/exit" => {
                            break;
                        }
                        _ => {
                            println!(
                                "Unknown command: {}",
                                line.strip_prefix("/").unwrap().bold().red()
                            );
                        }
                    }

                    continue;
                }

                rl.add_history_entry(line.as_str())?;

                chat_completion(
                    &client,
                    &headers,
                    &mut messages,
                    &spinner_values,
                    &config,
                    &line,
                    &ps,
                    &ts,
                    &code_re,
                    &tiny_code_re,
                )
                .await?;
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

    config::save_config(&config_dir.display().to_string().as_str(), &config).await?;

    if config.app.notify_save {
        let logs_dir = data_dir.join("logs");
        if !logs_dir.exists() {
            tokio::fs::create_dir_all(&logs_dir).await?;
        }

        let log_file = &logs_dir.join(format!(
            "{}.json",
            chrono::Local::now().format("%Y-%m-%d_%H-%M-%S")
        ));

        let log_file = tokio::fs::File::create(log_file).await?;
        let mut log_file = tokio::io::BufWriter::new(log_file);

        let messages_json = serde_json::to_string(&messages)?;
        log_file.write_all(messages_json.as_bytes()).await?;

        let mut log_file_content = String::new();
        for message in messages {
            let formatted_string = match message.role.as_str() {
                "user" => format!("[{}]\n{}", "User", message.content),
                "assistant" => format!("[{}]\n{}", "GPT", message.content),
                _ => format!("[{}]\n{}", message.role, message.content),
            };

            log_file_content.push_str(&formatted_string);
            log_file_content.push_str("\n\n");
        }

        let log_file = &logs_dir.join(format!(
            "{}.log",
            chrono::Local::now().format("%Y-%m-%d_%H-%M-%S")
        ));

        let log_file = tokio::fs::File::create(log_file).await?;
        let mut log_file = tokio::io::BufWriter::new(log_file);

        log_file.write_all(log_file_content.as_bytes()).await?;
    }

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
        message_mut = message_mut.replace(&cap["language"], &cap["language"].grey().to_string());

        let mut language = &cap["language"];

        let syntax = ps.find_syntax_by_extension(language);
        if syntax.is_none() {
            match &cap["language"] {
                "rust" => language = "rs",
                "javascript" => language = "jsx",
                "python" => language = "py",
                "typescript" => language = "tsx",
                "c++" => language = "cpp",
                "c#" => language = "cs",
                "kotlin" => language = "kt",
                "ruby" => language = "erb",
                "bash" => language = "sh",
                "shell" => language = "sh",
                "sh" => language = "sh",
                "powershell" => language = "ps1",
                "elixir" => language = "ex",
                "erlang" => language = "erl",
                "haskell" => language = "hs",
                "webassembly" => language = "wasm",
                "assembly" => language = "asm",
                "markdown" => language = "md",
                _ => {}
            }
        }

        let syntax = ps.find_syntax_by_extension(language);

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

fn get_time_suffix(elapsed: &std::time::Duration) -> &str {
    let elapsed = elapsed.as_secs_f32();

    if elapsed < 1.0 {
        "ms"
    } else if elapsed < 60.0 {
        "s"
    } else if elapsed < 3600.0 {
        "m"
    } else {
        "h"
    }
}

async fn chat_completion(
    client: &reqwest::Client,
    headers: &reqwest::header::HeaderMap,
    messages: &mut Vec<Message>,
    spinner_values: &Vec<&Spinner>,
    config: &config::Config,
    prompt: &String,
    ps: &syntect::parsing::SyntaxSet,
    ts: &syntect::highlighting::ThemeSet,
    code_re: &Regex,
    tiny_code_re: &Regex,
) -> Result<(), Box<dyn std::error::Error>> {
    let now = std::time::Instant::now();
    execute!(stdout(), cursor::Hide).unwrap();

    println!();

    // Get a random spinner
    let spinner = spinner_values.choose(&mut rand::thread_rng()).unwrap();
    let spinner_frames = spinner.frames.clone();
    let spinner_interval = spinner.interval;

    let spinner_frame = Arc::new(Mutex::new(spinner_frames[0].clone()));
    let spinner_frame_clone = spinner_frame.clone();

    let rainbow_speed = config.app.rainbow_speed;
    let rainbow_delay = config.app.rainbow_delay;

    let rainbow_task = tokio::spawn(async move {
        let mut i = 0;

        let dots = vec![
            ".    ", "..   ", "...  ", ".... ", ".....", " ....", "  ...", "   ..", "    .",
            "     ",
        ];

        loop {
            let r = (i as f32 / rainbow_speed).sin().powi(2);
            let g = (i as f32 / rainbow_speed + 2.0 * std::f32::consts::PI / 3.0)
                .sin()
                .powi(2);
            let b = (i as f32 / rainbow_speed + 4.0 * std::f32::consts::PI / 3.0)
                .sin()
                .powi(2);

            let color_style = Color::Rgb {
                r: (r * 255.0) as u8,
                g: (g * 255.0) as u8,
                b: (b * 255.0) as u8,
            };

            // Colorize the current line
            execute!(stdout(), cursor::MoveToColumn(0)).unwrap();
            execute!(stdout(), style::SetForegroundColor(color_style)).unwrap();
            let elapsed = &now.elapsed();
            let formatted_time = get_time_suffix(elapsed);
            print!(
                " {} {} {}{}{}{}",
                *spinner_frame.lock().unwrap(),
                dots[i % dots.len()].grey(),
                "(".grey(),
                format!("{:.2}", elapsed.as_secs_f32()).bold().dark_green(),
                formatted_time.grey(),
                ")".grey()
            );
            execute!(stdout(), Clear(ClearType::UntilNewLine)).unwrap();

            tokio::time::sleep(std::time::Duration::from_millis(
                if u64::from(spinner_interval) < rainbow_delay {
                    rainbow_delay
                } else {
                    spinner_interval.into()
                },
            ))
            .await;

            i = i + 1;
        }
    });

    let spin_task = tokio::spawn(async move {
        loop {
            for frame in spinner_frames.iter() {
                // Print the frame
                //print!("{} ({:.2}ms)", frame, now.elapsed().as_secs_f32());
                *spinner_frame_clone.lock().unwrap() = frame.clone();
                tokio::time::sleep(std::time::Duration::from_millis(spinner_interval.into())).await;
            }
        }
    });

    // message_history.push(format!("{}: {}",
    // //"",
    // "You",
    // line));
    messages.push(Message {
        role: "user".to_string(),
        content: prompt.clone(),
    });

    let messages_json = serde_json::to_value(&messages)?;
    let body = serde_json::json!({
        "model": config.openai.model,
        "messages": messages_json,
        "max_tokens": config.openai.max_tokens,
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

    assert!(rainbow_task.await.unwrap_err().is_cancelled());
    assert!(spin_task.await.unwrap_err().is_cancelled());

    execute!(stdout(), cursor::MoveToColumn(0)).unwrap();
    execute!(stdout(), crossterm::style::SetForegroundColor(Color::Reset)).unwrap();

    execute!(stdout(), Clear(ClearType::UntilNewLine)).unwrap();

    print!(
        "{} {}{}{}",
        "✓".green().bold(),
        "(finished in ",
        format!("{:.2}", now.elapsed().as_secs_f32())
            .bold()
            .dark_green(),
        format!("{})", get_time_suffix(&now.elapsed()))
    );

    let pretty_string = highlight_message(&message.content, &ps, &ts, &code_re, &tiny_code_re);

    println!("\n");

    println!(
        "{}: {}\n",
        config.app.response_prefix.clone().dark_green().bold(),
        pretty_string
    );

    execute!(stdout(), cursor::Show).unwrap();
    Ok(())
}
