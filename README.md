# Subilo

🛳 Tiny deployment agent

[![Rust](https://github.com/huemul/subilo/workflows/Rust/badge.svg)](https://github.com/Huemul/subilo/actions?query=workflow%3ARust)


Subilo is a tool to setup continuous deployments for applications running on
machines with no external integrations like IoT devices and VPSs.

#### How it works: 
Subilo is a small server that lives on your app's machine and listens for
authenticated HTTP webhooks. These webhooks have information about what app
should be deployed matching the Subilo configuration file (`.subilorc`).
This configuration file also defines what steps should be taken to successfully
deploy an application, for example: `git pull`, `./restart-server` and `./notify`.

#### Basic example: 

Configuration (`.subilorc`):

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

This webhook is usually sent from a CI after the tests passed.

```bash
curl -X POST 'https://subilo.yourdomain.com/webhook' \
  -H 'Authorization: Bearer ********' \
  -d '{ "name": "foo-project" }'
```

Status and logs of these project deployments can be checked in the [Dashboard](https://subilo.io/jobs)
using the URL and authentication token provided by the Subilo agent.


## Install

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

Create a `.subilorc` file with the required configuration.
A `.subilorc` example can be found [here](https://github.com/huemul/subilo/blob/master/configuration.md).

### Start

To start the Subilo agent the `serve` command should be used specifying the
authentication secret, the port (optional), config file and logs directory (optional).

Example:

```bash
subilo --secret super-secret serve --port 8089 --config /path/to/.subilorc
```

### Authentication

To get access to Subilo agent endpoints, create an authentication token using the
`token` command in the CLI.

#### Token with write permissions:
This token is used to access the POST `/webhook` endpoint and deploy the
an application using the predefined commands in the `.subilorc` file.

Example:

```bash
subilo --secret super-secret token
```

#### Token with only read permissions:
This token is used to access the logs and project configuration endpoints, these
endpoints are used by the https://subilo.io website.

Example:

```bash
subilo --secret super-secret token --permissions job:read
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

To read logs and check status from systemctl, the following commands can be used:

```bash
$ systemctl status subilo
$ journalctl -u subilo -b
```

### Setup deployment webhooks 

Once Subilo is running and exposed to the internet, deployment jobs can be
triggered by POSTing to the `/webhook` endpoint wiht the project name.

The project name is matched against the `.subilorc` configuration file and the
specified commands are run to deploy the project.

#### CI

Usually, this webhook is used by a CI, so after the application's tests passed,
the application can be deployed safely.
Store the token as a secret in the CI configuration and add a curl command to
POST to the `/webhook` endpoint to trigger a deploy.

Example:

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
