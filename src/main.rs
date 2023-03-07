use anyhow::bail;
use clap::Parser;
use console::style;
use futures::stream::StreamExt;
use reqwest::header::{HeaderMap, AUTHORIZATION};
use reqwest::Client;
use reqwest_eventsource::{Event, EventSource};
use rustyline::error::ReadlineError;
use rustyline::DefaultEditor;
use serde::{Deserialize, Serialize};
use std::io::Write;

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
struct Message {
    role: String,
    content: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct DeltaMessage {
    role: Option<String>,
    content: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
struct OpenAIRequest {
    model: String,
    messages: Vec<Message>,
    stream: bool,

    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f64>,

    #[serde(skip_serializing_if = "Option::is_none")]
    top_p: Option<f64>,
}

#[derive(Debug, Deserialize, Serialize)]
struct ResponseChoice {
    pub message: Message,
    pub index: usize,
    pub finish_reason: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
struct ResponseDeltaChoice {
    pub delta: DeltaMessage,
    pub index: usize,
    pub finish_reason: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
struct ResponseStreamMessage {
    pub id: String,
    pub object: String,
    pub created: u64,
    pub model: String,
    pub choices: Vec<ResponseDeltaChoice>,
}

#[derive(Debug, Deserialize, Serialize)]
struct ResponseMessage {
    pub choices: Vec<ResponseChoice>,
    pub created: u64,
    pub id: String,
    pub model: String,
    pub object: String,
    pub usage: ResponseUsage,
}

#[derive(Debug, Deserialize, Serialize)]
struct ResponseUsage {
    pub completion_tokens: isize,
    pub prompt_tokens: isize,
    pub total_tokens: isize,
}

/// Simple program to greet a person
#[derive(Parser, Debug)]
#[command(about, long_about = None, trailing_var_arg=true)]
struct Options {
    /// OpenAI API Key
    #[arg(long, required = true, env = "OPENAI_API_KEY", hide_env_values = true)]
    pub api_key: String,

    /// Whether to use streaming API
    #[arg(long)]
    pub no_stream: bool,

    /// The model to query
    #[arg(long, default_value_t = String::from("gpt-3.5-turbo"))]
    pub model: String,

    /// Send a 'system' message at the beginning (interactive mode only)
    #[arg(short, long)]
    pub system: bool,

    /// Sampling temperature to use, between 0 and 2.
    #[arg(
        long,
        hide_short_help = true,
        long_help = r#"Higher values like 0.8 will make the output more random, while lower values like 0.2 will make it more focused and deterministic.
We generally recommend altering this or top_p but not both."#
    )]
    pub temperature: Option<f64>,

    #[arg(
        long,
        hide_short_help = true,
        long_help = r#"An alternative to sampling with temperature, called nucleus sampling, where the model considers the results of the tokens with top_p probability mass. So 0.1 means only the tokens comprising the top 10% probability mass are considered.
We generally recommend altering this or temperature but not both."#
    )]
    pub top_p: Option<f64>,

    /// The prompt to ask. Leave it empty to activate interactive mode
    pub prompt: Vec<String>,
}

const READLINE_HISTORY: &str = ".heygpt_history";

#[tokio::main(flavor = "current_thread")]
async fn main() -> anyhow::Result<()> {
    let options = Options::parse();

    dbg!(&options);
    // Enter interactive mode if prompt is empty
    let interactive = options.prompt.is_empty();

    if !interactive {
        if options.system {
            anyhow::bail!("system message is only supported in interactive mode");
        }

        let prompt = options.prompt.join(" ");

        let mut messages = vec![];
        messages.push(Message {
            role: "user".to_string(),
            content: prompt.clone(),
        });

        let _ = complete_and_print(&options, &messages).await?;
    } else {
        let mut messages = vec![];
        let mut rl = DefaultEditor::new()?;

        let history_file = {
            let mut p = dirs::home_dir().unwrap();
            p.push(READLINE_HISTORY);
            p.to_str().unwrap().to_owned()
        };
        let _ = rl.load_history(&history_file);

        if options.system {
            let readline = rl.readline(&format!("{} => ", style("system").bold().white()));
            let prompt = match readline {
                Ok(line) => {
                    rl.add_history_entry(line.as_str())?;
                    line
                }
                Err(ReadlineError::Interrupted) => {
                    println!("CTRL-C");
                    return Ok(());
                }
                Err(ReadlineError::Eof) => {
                    println!("CTRL-D");
                    return Ok(());
                }
                Err(err) => {
                    bail!("Readline error: {:?}", err);
                }
            };
            messages.push(Message {
                role: "system".to_string(),
                content: prompt,
            });
        }

        loop {
            let readline = rl.readline(&format!("{} => ", style("user").bold().cyan()));
            let prompt = match readline {
                Ok(line) => {
                    rl.add_history_entry(line.as_str())?;
                    line
                }
                Err(ReadlineError::Interrupted) => {
                    println!("CTRL-C");
                    break;
                }
                Err(ReadlineError::Eof) => {
                    println!("CTRL-D");
                    break;
                }
                Err(err) => {
                    bail!("Readline error: {:?}", err);
                }
            };

            messages.push(Message {
                role: "user".to_string(),
                content: prompt,
            });

            print!("{} => ", style("assistant").bold().green());
            std::io::stdout().flush()?;

            let response = complete_and_print(&options, &messages).await?;

            messages.push(response);
        }

        rl.append_history(&history_file)?;
    }

    Ok(())
}

/// Complete the message sequence and output the response in time
async fn complete_and_print(options: &Options, messages: &[Message]) -> anyhow::Result<Message> {
    let data = OpenAIRequest {
        model: options.model.clone(),
        stream: !options.no_stream,
        messages: messages.to_vec(),
        temperature: options.temperature,
        top_p: options.top_p,
    };

    let mut headers = HeaderMap::new();
    headers.insert(
        AUTHORIZATION,
        format!("Bearer {}", options.api_key).parse().unwrap(),
    );

    let client = Client::new();
    let req_builder = client
        .post("https://api.openai.com/v1/chat/completions".to_string())
        .headers(headers)
        .json(&data);

    if !options.no_stream {
        let mut full_message = Message::default();

        let mut es = EventSource::new(req_builder)?;
        while let Some(event) = es.next().await {
            match event {
                Ok(Event::Open) => {
                    //println!("Connection Open!")
                }
                Ok(Event::Message(message)) if message.data == "[DONE]" => {
                    println!();
                    //println!("Done!");
                    break;
                }
                Ok(Event::Message(message)) => {
                    let message: ResponseStreamMessage = serde_json::from_str(&message.data)?;
                    let delta = message.choices.into_iter().next().unwrap().delta;
                    if let Some(role) = delta.role {
                        //print!("{}: ", role);
                        full_message.role.push_str(&role);
                    }
                    if let Some(mut content) = delta.content {
                        // Trick: Sometimes the response starts with a newline. Strip it here.
                        if content.starts_with("\n") && full_message.content.is_empty() {
                            content = content.trim_start().to_owned();
                        }
                        print!("{}", content);
                        full_message.content.push_str(&content);
                    }
                    std::io::stdout().flush().unwrap();
                }
                Err(err) => {
                    es.close();
                    anyhow::bail!("EventSource stream error: {}", err);
                }
            }
        }

        Ok(full_message)
    } else {
        let response: ResponseMessage = req_builder.send().await?.json().await?;

        let mut message = response.choices[0].message.clone();

        // Trick: Sometimes the response starts with a newline. Strip it here.
        if message.content.starts_with("\n") {
            message.content = message.content.trim_start().to_owned();
        }

        println!("{}", &message.content);

        Ok(message)
    }
}
