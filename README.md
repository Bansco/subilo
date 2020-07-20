# Subilo

🛳 Tiny deployment agent

[![Rust](https://github.com/huemul/subilo/workflows/Rust/badge.svg)](https://github.com/Huemul/subilo/actions?query=workflow%3ARust)

Subilo is a deployment agent that allows executing predefined bash commands on
the server where it is running (VPS, raspberry PI, any Linux machine).
It's a small server that listens on a specified port for HTTP requests
(the port should be open to the internet). It exposes a `/webhook` endpoint that
receives a project name that is matched against the Subilo configuration file
(.subilorc) to check what commands should be run.

Useful to deploy projects running on a private server where a normal CI does not
have access to. Just push a webhook after the CI finishes and your project will
be deployed.

### Basic example: 

Configuration:

```toml
[[projects]]
name = "project-foo"
path = "~/projects/project-foo"
commands = [
  "git pull",
  "./restart-serever.sh",
  "echo Pulled changes and restarted server successfully",
]
```

Webhook:

```bash
curl -X POST 'https://subilo.yourdomain.com/webhook' \
  -H 'Authorization: Bearer ********' \
  -d '{ "name": "foo-project" }'
```

## Install and setup

### Install script

```
curl -s -L https://raw.githubusercontent.com/huemul/subilo/master/install.sh | bash
```

This command runs the [install script](https://github.com/huemul/subilo/blob/master/install.sh).
The script downloads the latest Subilo release and attempts to add the Subilo bin
path to the `$PATH` variable in the correct profile file (`~/.profile`, `~/.bashrc`,
`~/.bash_profile`, `~/.zshrc` or `~/.config/fish/config.fish`)

### Cargo

```
$ cargo install subilo
```

### Manually

Download the latest [released binary](https://github.com/huemul/subilo/releases)
and add executable permissions:

```
$ wget -O subilo "https://github.com/huemul/subilo/releases/download/v0.1.2/subilo-x86-64-linux"
$ chmod +x subilo
```

## Use

### Command line interface

Now that Subilo is available, the `help` subcommand can be run to display the
CLI information:

```
$ subilo --help
subilo 0.0.1
Tiny deployment agent

USAGE:
    subilo [FLAGS] [OPTIONS] <SUBCOMMAND>

FLAGS:
    -h, --help       Prints help information
    -V, --version    Prints version information
    -v, --verbose    Makes Subilo verbose. Useful for debugging and seeing what's going on "under the hood"

OPTIONS:
    -s, --secret <secret>    Secret to generate and authenticate the token

SUBCOMMANDS:
    help     Prints this message or the help of the given subcommand(s)
    serve    Start subilo agent
    token    Create a token based on the secret to authorize agent connections
```

### Configuration

Create a `.subilorc` file with the required configuration to deploy projects.
A `.subilorc` example can be found [here](https://github.com/huemul/subilo/blob/master/sample.subilorc).

### Start

To start the Subilo agent the `serve` command should be used specifying the
authentication secret and optionally the port, config file and logs directory.

Example:

```bash
subilo --secret super-secret serve --port 8089 --config /path/to/.subilorc
```

### Authentication

To get access to Subilo agent endpoints, create an authentication token using the
`token` command in the CLI.

#### Token with write permissions:
This token is used to access the POST `/webhook` endpoints that will create a job
and execute the predefined commands for the specified project.

Example:

```bash
subilo --secret super-secret token --permissions job:write
```

#### Token with only read permissions:
This token is used to access the logs and project configuration endpoints, these
endpoints are used by the https://subilo.io website.

Example:

```bash
subilo --secret super-secret token
```

### Systemd configuration (Optional)

Create a systemd service file (`/etc/systemd/system/subilo.service`) with the
following attributes:

```
[Unit]
Description=Subilo

[Service]
ExecStart=/path/to/subilo -s super-secret-secret serve -l /path/to/subilo-logs -p 8080 -c /path/to/.subilorc
```

Then enable and start Subilo service:

```bash
# Might require sudo
$ systemctl enable /etc/systemd/system/subilo.service
$ systemctl start subilo
```

To read logs and check status the following commands can be used:

```bash
$ systemctl status subilo
$ journalctl -u subilo -b
```

### Trigger jobs

Once Subilo is running and exposed to the internet, deployment jobs can be
triggered by POSTing to the `/webhook` endpoint wiht the project name.

```bash
curl -X POST 'https://subilo.yourdomain.com/webhook' \
  -H 'Authorization: Bearer ********' \
  -d '{ "name": "foo-project" }'
```

## Development

#### Run

```bash
cargo run

# Watch mode
cargo watch -x run

# Setting CLI options
cargo run -- --port 9090 --logs-dir ./logs
```

#### Test

```bash
cargo test

# Watch mode
cargo watch -x test
```
