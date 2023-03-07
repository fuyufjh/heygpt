# `heygpt`

A simple common-line interface for [ChatGPT API](https://platform.openai.com/docs/api-reference/chat/create).

- 🌟 Streaming output! 
- 🤖 Both interactive & one-shot mode

<img width="1022" alt="Screen Shot 2023-03-07 at 09 30 53" src="https://user-images.githubusercontent.com/10192522/223295925-00eed881-cdfc-4f46-9510-1e0bd1c99e60.png">

**[Demo (YouTube)](https://youtu.be/Edqts2ff1Y0)**

## Quickstart

Install to `$HOME/.cargo/bin/` via cargo:

```bash
cargo install --path .
```

You'll need a OpenAI API key (you can get one [here](https://platform.openai.com/account/api-keys)), and you'll need to export your API Key as an environment variable:


```bash
export OPENAI_API_KEY=<your api key>
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

You may need write prompt in multiple lines:

```bash
heygpt "please translate this poem to English:
> 床前明月光，
> 疑是地上霜。
> 举头望明月，
> 低头思故乡。"
```
