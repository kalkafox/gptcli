# gptcli

gptcli is a command line interface built with Rust that currently offers syntax highlighting, configurable settings, color support, and message history. It offers a user-friendly way to interact with [OpenAI's GPT-3 API](https://openai.com) (and hopefully, GPT-4 aswell!)

## Installation

To install gptcli, you'll need to have Rust installed on your system. Once you have Rust set up, you can install gptcli by running the following command:

```sh
cargo install gptcli
```

This should automatically download and install gptcli on your system.

## Usage

To use gptcli, simply run the following command:

```sh
gptcli
```

This will start the client in the default configuration.

To use gptcli with a prompt, run the following command:

```sh
gptcli 'How far away is Andromeda?'
```

This will initialize gptcli with the specified prompt.

## Configuration

To configure gptcli, you can create a `config.toml` file in the same directory as the `gptcli` binary. The `config.toml` file should contain settings in the following format:

```toml
[openai]
model = "gpt-3.5-turbo"
temperature = 1.0
top_p = 1.0
n = 1
max_tokens = "inf"
frequency_penalty = 0.0
presence_penalty = 0.0

[app]
prompt = """
Please wrap code in triple backticks, with the language specified. For example, ```python
print('Hello world')
```"""
rainbow_speed = 15.0
notify_save = true
response_prefix = "GPT-3"
rainbow_delay = 100
save_conversation = true
```

Here's a brief description of each setting:

- OpenAI

  - `model`: The model to use for the API. Defaults to `gpt-3.5-turbo`.
  - `temperature`: The temperature to use for the API. Defaults to `1.0`.
  - `top_p`: The top_p to use for the API. Defaults to `1.0`.
  - `n`: The n to use for the API. Defaults to `1`.
  - `max_tokens`: The max_tokens to use for the API. Defaults to `inf`.
  - `frequency_penalty`: The frequency_penalty to use for the API. Defaults to `0.0`.
  - `presence_penalty`: The presence_penalty to use for the API. Defaults to `0.0`.

- App
  - `prompt`: The prompt to use when starting the client. Defaults to `Please wrap code in triple backticks, with the language specified. For example, ```python print('Hello world') ````
  - `rainbow_speed`: The speed at which the rainbow effect should run. Defaults to `15.0`.
  - `notify_save`: Whether or not to notify the user when a conversation is saved. Defaults to `true`.
  - `response_prefix`: The prefix to use for the response. Defaults to `GPT-3`.
  - `rainbow_delay`: The delay between each rainbow effect. Defaults to `100`.
  - `save_conversation`: Whether or not to save the conversation. Defaults to `true`.

## Contributing

If you have any issues or feature requests, please open an issue on the [GitHub repository](https://github.com/kalkafox/gptcli). Pull requests are welcome!
