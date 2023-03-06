use clap::Parser;
use console::style;
use futures::stream::StreamExt;
use reqwest::header::{HeaderMap, AUTHORIZATION};
use reqwest::Client;
use reqwest_eventsource::{Event, EventSource};
use serde::{Deserialize, Serialize};
use std::env;
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
    /// Whether to use streaming API
    #[arg(long)]
    pub no_stream: bool,

    /// The model to query
    #[arg(long, default_value_t = String::from("gpt-3.5-turbo"))]
    pub model: String,

    /// The prompt to ask
    pub prompt: Vec<String>,
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> anyhow::Result<()> {
    let options = Options::parse();

    // get OPENAI_API_KEY from environment variable
    let key = "OPENAI_API_KEY";
    let openai_api_key = env::var(key).unwrap_or_else(|_| panic!("{} not set", key));

    let stream = !options.no_stream;

    // Enter interactive mode if prompt is empty
    let interactive = options.prompt.is_empty();

    if !interactive {
        let prompt = options.prompt.join(" ");

        let mut messages = vec![];
        messages.push(Message {
            role: "user".to_string(),
            content: prompt.clone(),
        });

        let _ = complete_and_print(
            openai_api_key.clone(),
            options.model.clone(),
            stream,
            &messages,
        )
        .await?;
    } else {
        let mut messages = vec![];

        loop {
            print!("{} => ", style("user").bold().cyan());
            std::io::stdout().flush()?;

            let mut buffer = String::new();
            std::io::stdin().read_line(&mut buffer)?;
            messages.push(Message {
                role: "user".to_string(),
                content: buffer,
            });

            print!("{} => ", style("assistant").bold().green());
            std::io::stdout().flush()?;

            let response = complete_and_print(
                openai_api_key.clone(),
                options.model.clone(),
                stream,
                &messages,
            )
            .await?;

            messages.push(response);
        }
    }

    Ok(())
}

/// Complete the message sequence and output the response in time
async fn complete_and_print(
    openai_api_key: String,
    model: String,
    stream: bool,
    messages: &[Message],
) -> anyhow::Result<Message> {
    let data = OpenAIRequest {
        model,
        stream,
        messages: messages.to_vec(),
    };

    let mut headers = HeaderMap::new();
    headers.insert(
        AUTHORIZATION,
        format!("Bearer {}", openai_api_key).parse().unwrap(),
    );

    let client = Client::new();
    let req_builder = client
        .post("https://api.openai.com/v1/chat/completions".to_string())
        .headers(headers)
        .json(&data);

    if stream {
        let mut full_message = Message::default();

        let mut es = EventSource::new(req_builder)?;
        while let Some(event) = es.next().await {
            match event {
                Ok(Event::Open) => {
                    //println!("Connection Open!")
                }
                Ok(Event::Message(message)) if message.data == "[DONE]" => {
                    //dbg!(&message);
                    println!();
                    //println!("Done!");
                    break;
                }
                Ok(Event::Message(message)) => {
                    let message: ResponseStreamMessage = serde_json::from_str(&message.data)?;
                    let delta = &message.choices[0].delta;
                    if let Some(role) = &delta.role {
                        //print!("{}: ", role);
                        full_message.role.push_str(role);
                    }
                    if let Some(content) = &delta.content {
                        print!("{}", content);
                        full_message.content.push_str(content);
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

        let message = response.choices[0].message.clone();

        println!("{}", &message.content);

        Ok(message)
    }
}
