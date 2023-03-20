use anyhow::{anyhow, bail, Result};
use clap::Parser;
use console::style;
use futures::stream::StreamExt;
use log::{debug, trace};
use reqwest::header::{HeaderMap, AUTHORIZATION};
use reqwest::{Client, RequestBuilder, StatusCode};
use reqwest_eventsource::{Event, EventSource};
use rustyline::error::ReadlineError;
use rustyline::{DefaultEditor, Editor};
use std::io::Write;

mod model;
mod spinner;

use model::*;
use spinner::Spinner;

/// Command-line options
#[derive(Parser, Debug)]
#[command(about, long_about = None, trailing_var_arg=true)]
struct Options {
    /// Whether to use streaming API
    #[arg(long)]
    pub no_stream: bool,

    /// The model to query
    #[arg(long, default_value_t = String::from("gpt-3.5-turbo"))]
    pub model: String,

    /// Sampling temperature to use, between 0 and 2.
    #[arg(
        long,
        hide_short_help = true,
        long_help = r#"Higher values like 0.8 will make the output more random, while lower values like 0.2 will make it more focused and deterministic.
We generally recommend altering this or top_p but not both."#
    )]
    pub temperature: Option<f64>,

    /// Probability of nucleus sampling
    #[arg(
        long,
        hide_short_help = true,
        long_help = r#"An alternative to sampling with temperature, called nucleus sampling, where the model considers the results of the tokens with top_p probability mass. So 0.1 means only the tokens comprising the top 10% probability mass are considered.
We generally recommend altering this or temperature but not both."#
    )]
    pub top_p: Option<f64>,

    #[arg(
        long,
        hide_short_help = true,
        default_missing_value = "",
        num_args(0..=1),
        require_equals = true,
        long_help = "System prompt passed to chatgpt."
    )]
    pub system: Option<String>,

    /// The prompt to ask. Leave it empty to activate interactive mode
    pub prompt: Vec<String>,
}

const READLINE_HISTORY: &str = ".heygpt_history";

const OPENAI_API_KEY: &str = "OPENAI_API_KEY";
const OPENAI_API_BASE: &str = "OPENAI_API_BASE";

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<()> {
    env_logger::init();

    let options = Options::parse();

    // get OPENAI_API_KEY from environment variable
    let api_key =
        std::env::var(OPENAI_API_KEY).map_err(|_| anyhow!("{} not set", OPENAI_API_KEY))?;

    let api_base = std::env::var(OPENAI_API_BASE).unwrap_or("https://api.openai.com/v1".into());

    let is_stdout = atty::is(atty::Stream::Stdout);

    let mut session = Session::new(options, api_key, api_base, is_stdout);
    if !session.is_interactive() {
        session.run_one_shot().await?;
    } else {
        session.run_interactive().await?;
    }

    Ok(())
}

struct Session {
    /// Command-line options
    options: Options,

    /// OpenAI API key
    api_key: String,

    /// OpenAI API base URL
    api_base: String,

    /// Messages history
    messages: Vec<Message>,

    /// Whether stdout is a TTY
    is_stdout: bool,

    /// Spinner holder
    spinner: Option<Spinner>,
}

impl Session {
    pub fn new(options: Options, api_key: String, api_base: String, is_stdout: bool) -> Self {
        Self {
            options,
            api_key,
            api_base,
            is_stdout,
            messages: Vec::new(),
            spinner: None,
        }
    }

    pub fn is_interactive(&self) -> bool {
        // Enter interactive mode if prompt is empty
        self.options.prompt.is_empty()
    }

    pub async fn run_one_shot(&mut self) -> Result<()> {
        let prompt = self.options.prompt.join(" ");

        if let Some(system_prompt) = &self.options.system {
            self.messages.push(Message {
                role: "system".to_string(),
                content: system_prompt.clone(),
            });
        }

        self.messages.push(Message {
            role: "user".to_string(),
            content: prompt,
        });

        let _ = self.complete_and_print().await?;
        Ok(())
    }

    pub async fn run_interactive(&mut self) -> Result<()> {
        let mut rl = DefaultEditor::new()?;

        // Persist input history in `$HOME/.heygpt_history`
        let history_file = {
            let mut p = dirs::home_dir().unwrap();
            p.push(READLINE_HISTORY);
            p.to_str().unwrap().to_owned()
        };
        let _ = rl.load_history(&history_file);

        // If `--system` or `--system="..."` is specified
        if let Some(s) = &self.options.system {
            let system_prompt = if !s.is_empty() {
                // If `--system="..."` is specified, use it as system prompt
                s.clone()
            } else {
                // Otherwise, read system prompt interactively
                if let Some(p) = self.read_prompt(&mut rl, "system").await? {
                    p
                } else {
                    return Ok(());
                }
            };
            self.messages.push(Message {
                role: "system".to_string(),
                content: system_prompt,
            });
        };

        loop {
            let prompt = if let Some(p) = self.read_prompt(&mut rl, "user").await? {
                p
            } else {
                break;
            };

            self.messages.push(Message {
                role: "user".to_string(),
                content: prompt,
            });

            match self.complete_and_print().await {
                Ok(response) => self.messages.push(response),
                Err(err) => {
                    let last_msg = self.messages.pop(); // remove the last message
                    assert!(last_msg.is_some());
                    println!("{}: {err}", style("ERROR").bold().red());
                }
            }
        }

        rl.append_history(&history_file)?;
        Ok(())
    }

    async fn read_prompt<H, I>(
        &mut self,
        rl: &mut Editor<H, I>,
        role: &str,
    ) -> Result<Option<String>>
    where
        H: rustyline::Helper,
        I: rustyline::history::History,
    {
        loop {
            let readline = rl.readline(&format!("{} => ", style(role).bold().cyan()));
            match readline {
                Ok(line) => {
                    if line.is_empty() {
                        continue; // ignore empty input
                    }
                    rl.add_history_entry(line.as_str())?;

                    if line.starts_with("\\") {
                        let cmd = &line[1..];
                        self.run_command(cmd);
                        continue;
                    } else {
                        return Ok(Some(line));
                    }
                }
                Err(ReadlineError::Interrupted) => {
                    println!("CTRL-C");
                    return Ok(None);
                }
                Err(ReadlineError::Eof) => {
                    println!("CTRL-D");
                    return Ok(None);
                }
                Err(err) => {
                    bail!("Readline error: {:?}", err);
                }
            };
        }
    }

    /// Complete the message sequence and returns the next message.
    /// Meanwhile, output the response to stdout.
    async fn complete_and_print(&mut self) -> Result<Message> {
        // Build the request
        let data = Request {
            model: self.options.model.clone(),
            stream: !self.options.no_stream,
            messages: self.messages.to_vec(),
            temperature: self.options.temperature,
            top_p: self.options.top_p,
        };

        let mut headers = HeaderMap::new();
        headers.insert(
            AUTHORIZATION,
            format!("Bearer {}", self.api_key).parse().unwrap(),
        );

        let client = Client::new();
        let req = client
            .post(format!("{}/chat/completions", &self.api_base))
            .headers(headers)
            .json(&data);

        debug!("Request body: {:?}", &data);

        // Show spinner if stdout is not redirected
        if self.is_stdout {
            self.spinner = Some(Spinner::new());
        }

        if !self.options.no_stream {
            self.do_stream_request(req).await
        } else {
            self.do_non_stream_request(req).await
        }
    }

    async fn do_stream_request(&mut self, req: RequestBuilder) -> Result<Message> {
        let mut full_message = Message::default();

        let mut es = EventSource::new(req)?;
        while let Some(event) = es.next().await {
            self.spinner = None;
            match event {
                Ok(Event::Open) => {
                    debug!("response stream opened");
                }
                Ok(Event::Message(message)) if message.data == "[DONE]" => {
                    debug!("response stream ended with [DONE]");
                    println!();
                    break;
                }
                Ok(Event::Message(message)) => {
                    trace!("response stream message: {:?}", &message);
                    let message: ResponseStreamMessage = serde_json::from_str(&message.data)?;
                    let delta = message.choices.into_iter().next().unwrap().delta;
                    if let Some(role) = delta.role {
                        full_message.role.push_str(&role);

                        if self.is_interactive() {
                            print!("{} => ", style(role).bold().green());
                            std::io::stdout().flush().unwrap();
                        }
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
                    debug!("EventSource stream error: {}", err);
                    return Err(err.into());
                }
            }
        }

        debug!("response stream full message: {:?}", &full_message);

        Ok(full_message)
    }

    async fn do_non_stream_request(&mut self, req: RequestBuilder) -> Result<Message> {
        let response = req.send().await?;

        self.spinner = None;

        if response.status() != StatusCode::OK {
            let r: WrappedApiError = response.json().await?;
            return Err(anyhow!("{}: {}", r.error.r#type, r.error.message));
        }

        let response: ResponseMessage = response.json().await?;
        debug!("response message: {:?}", &response);

        let mut message = response.choices[0].message.clone();

        // Trick: Sometimes the response starts with a newline. Strip it here.
        if message.content.starts_with("\n") {
            message.content = message.content.trim_start().to_owned();
        }

        if self.is_interactive() {
            print!("{} => ", style(&message.role).bold().green());
        }
        println!("{}", &message.content);
        std::io::stdout().flush()?;

        Ok(message)
    }

    fn run_command(&mut self, cmd: &str) {
        match cmd {
            "?" | "help" => {
                println!("{}", style("Available commands:").bold());
                println!("  \\?, \\help     Show this help");
                println!("  \\b, \\back     Retract and back to the last user message");
                println!("  \\h, \\history  View current conversation history");
            }
            "b" | "back" => match self.retract() {
                Ok(()) => println!("Retracted last message"),
                Err(err) => println!("{}: {err}", style("ERROR").bold().red()),
            },
            "h" | "history" => {
                println!("{}", style("History:").bold());
                for (i, message) in self.messages.iter().enumerate() {
                    println!("[{}] {} => {}", i, message.role, message.content);
                }
            }
            _ => {
                println!("Unknown command: \\{cmd}. Enter '\\?' for help.");
            }
        }
    }

    /// Retract the last message sent by user, as well as the subsequent messages
    fn retract(&mut self) -> Result<()> {
        let mut count = 0usize;
        for message in self.messages.iter().rev() {
            count += 1;
            if message.role == "user" {
                break;
            }
        }
        if count == 0 {
            bail!("No message to retract");
        } else {
            self.messages.truncate(self.messages.len() - count);
            Ok(())
        }
    }
}
