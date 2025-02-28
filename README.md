# autodocs

documentation auto translation using ai with manual/auto checking and reviewing

## installation

```bash
git clone https://github.com/enkerewpo/autodocs
```

## usage

```bash
cargo run -- run {config_file}.yml
```

demo config file:

```yaml
repo: https://github.com/syswonder/hvisor-book # repository URL for auto git clone and translation
branch: main

engine:
  name: openai
  url: https://api.openai.com/v1/engines/
  model: gpt-4o
  api_key_file: key.txt # store your api key here

# engine:
#   name: deepseek
#   url: https://api.siliconflow.cn/v1/
#   model: deepseek-ai/DeepSeek-V3
#   api_key_file: key-silicon.txt

filter:
  target: "*.md *.toml"
  include:
    - docs/
    - src/test.txt
  exclude:
    - docs/test/
```

## roadmap

- [x] multi-engine support
- [ ] multi-language intertranslating, currently only Chinese to English docs auto translation


wheatfox (wheatfox17@icloud.com) 2025