# `heygpt`

A simple common-line interface for [ChatGPT API](https://platform.openai.com/docs/api-reference/chat/create).

- 🌟 Streaming output! 
- 🤖 Both interactive & one-shot mode

<img width="1022" alt="Screen Shot 2023-03-07 at 09 30 53" src="https://user-images.githubusercontent.com/10192522/223295925-00eed881-cdfc-4f46-9510-1e0bd1c99e60.png">

**[Demo (YouTube)](https://youtu.be/Edqts2ff1Y0)**

## Quickstart

Install to `$HOME/.cargo/bin/` via cargo:

```
cargo install --path .
```

You'll need a OpenAI API key (you can get one [here](https://platform.openai.com/account/api-keys)), and you'll need to export your API Key as an environment variable:


```
export OPENAI_API_KEY=<your api key>
```

Then you can start a conversation with ChatGPT:

```
heygpt how to record screen on mac
```

OR use the interactive mode by providing no prompt:

```
heygpt 
```
