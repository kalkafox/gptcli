use std::{io::stdout, path::{self, Path}};

use crossterm::{
    cursor::{self, MoveLeft},
    execute, ExecutableCommand,
};
use directories::ProjectDirs;
use rustyline::{error::ReadlineError, DefaultEditor};
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

#[main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {

    let mut dire: path::PathBuf = Path::new(".").to_path_buf();

    if let Some(proj_dirs) = ProjectDirs::from("dev", "kalkafox",  "gpt-cli") {
        let dir = proj_dirs.config_dir();
        dire = dir.to_path_buf();
        if !dir.exists() {
            tokio::fs::create_dir_all(dir).await?;
        }
    }

    if !dire.join("openai.key").exists() {
        let openai_key = rpassword::prompt_password("Please enter your OpenAI API key: ")?;

        tokio::fs::write(dire.join("openai.key"), openai_key.clone()).await?;

        println!("OpenAI key has been stored in {}. Delete it if you wish.", dire.join("openai.key").display());
    }

    let openai_key = tokio::fs::read_to_string(dire.join("openai.key")).await?;

    let client = reqwest::Client::new();

    let mut headers = reqwest::header::HeaderMap::new();
    headers.append("Authorization", format!("Bearer {}", openai_key).parse()?);
    headers.append("Content-Type", "application/json".parse()?);

    let mut rl = DefaultEditor::new()?;
    #[cfg(feature = "with-file-history")]
    if rl.load_history("history.txt").is_err() {
        println!("No previous history.");
    }

    let mut messages: Vec<Message> = vec![];

    loop {
        let readline = rl.readline("You: ");
        match readline {
            Ok(line) => {
                if line.is_empty() {
                    continue;
                }
                
                println!();

                rl.add_history_entry(line.as_str())?;
                let task = tokio::spawn(async move {
                    let frames = vec![
                        "⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏", "⠋", "⠙", "⠚", "⠒", "⠂",
                        "⠂", "⠒", "⠲", "⠴", "⠦", "⠖", "⠒", "⠐", "⠐", "⠒", "⠓", "⠋",
                    ];

                    loop {
                        for frame in frames.iter() {
                            // Disable cursor
                            execute!(stdout(), cursor::Hide).unwrap();
                            print!("{}", frame);
                            execute!(stdout(), cursor::MoveLeft(1)).unwrap();
                            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
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

                let chat_completion: ChatCompletion = openai_res.json().await?;

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

                task.abort();

                // Enable cursor
                execute!(stdout(), cursor::Show).unwrap();
                execute!(stdout(), cursor::MoveLeft(2)).unwrap();

                println!("{}: {}\n", "GPT-3", message.content);

            }
            Err(ReadlineError::Interrupted) => {
                println!("Bye-bye!");
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
    #[cfg(feature = "with-file-history")]
    rl.save_history("history.txt");

    Ok(())
}
