# `heygpt`

A simple common-line interface for [ChatGPT API](https://platform.openai.com/docs/api-reference/chat/create).

- 🌟 Streaming output!
- 💡 One-shot mode to get a quick answer
- 🤖 Interactive mode to have a conversation

<img width="986" alt="screenshot" src="https://github.com/fuyufjh/heygpt/assets/10192522/68ee10a1-942d-47b1-8d13-2d5cef70b649">

**[Demo (YouTube)](https://youtu.be/Edqts2ff1Y0)**

## Quickstart

Install latest release version to `$HOME/.cargo/bin/` via cargo:

```bash
cargo install heygpt
```

You'll need a OpenAI API key (you can get one [here](https://platform.openai.com/account/api-keys)), and you'll need to export your API Key as an environment variable:

You can also set a OpenAI API base environment variable, just like [openai-python](https://github.com/openai/openai-python/blob/main/openai/__init__.py#L37)

```bash
export OPENAI_API_KEY=<your api key>
# export OPENAI_API_BASE="https://api.openai.com/v1"
```

Then you can start an interactive conversation with ChatGPT:

```bash
heygpt
```

OR use the one-shot mode by providing a prompt:

```bash
heygpt how to record screen on mac
```

You can also compose prompt with bash tricks like

```bash
heygpt read the code and tell me what it is doing: $(cat src/main.rs)
```

```bash
heygpt read the code diff and write a commit message: $(git diff)
```

```bash
heygpt "please translate this poem to English:
> 床前明月光，
> 疑是地上霜。
> 举头望明月，
> 低头思故乡。"
```

You may even compose `heygpt` with other CLI tools via pipes:

```
$ echo "It's late. I should go to bed" | heygpt | cowsay
 ______________________________________
/ Goodnight! Sleep well and have sweet \
\ dreams.                              /
 --------------------------------------
        \   ^__^
         \  (oo)\_______
            (__)\       )\/\
                ||----w |
                ||     ||
```

## Advanced

### Commands in interactive mode

Enter `\?` to see available commands:

```
user => \?
Available commands:
  \?, \help: Show this help
  \b, \back: Retract and back to the last user message
  \h, \history: View current conversation history
```

### Configuration file

`heygpt` will load configurations from `$HOME/.heygpt.toml`. You may also set API keys and base URL here. Example:

```toml
model = "gpt-4"
api_base_url = "https://some.openai.mirror/v1"
api_key = "your api key"
```
