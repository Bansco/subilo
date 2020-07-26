# Subilo

[![Rust](https://github.com/huemul/subilo/workflows/Rust/badge.svg)](https://github.com/Huemul/subilo/actions?query=workflow%3ARust)

> 🛳 Deployment automation agent

Subilo is a tool to setup continuous deployments for applications running on
machines with no external integrations, like IoT devices and VPSs.

## How it works

The Subilo agent is a small server that lives on your application's machine and
listens for secure HTTP webhooks. These webhooks have information about what
application to deploy matching the Subilo configuration file (`.subilorc`).
The file also defines what steps should be taken to successfully deploy an
application, for example: `git pull` or pull the latest Docker image, restart
the application and send a notification.

## Basic example

**Configuration** (`.subilorc`):

```toml
[[projects]]
name = "foo-app"
path = "~/apps/foo-app"
commands = [
  "git pull",
  "./restart-serever.sh",
  "echo 'Pulled changes and restarted server successfully'",
]
```

**Webhook**:

This webhook is usually sent from a CI run after the tests passed.

```bash
curl -X POST 'https://subilo.yourdomain.com/webhook' \
  -H 'Authorization: Bearer ********' \
  -H 'Content-Type: application/json' \
  -d '{ "name": "foo-app" }'
```

Status and logs of these deployments can then be seen in the
[Dashboard](https://subilo.io/jobs) using the URL and the authentication token
provided by the Subilo agent.

## Install

### Install script

```bash
curl -s -L https://raw.githubusercontent.com/huemul/subilo/master/install.sh | bash
```

This command runs the
[install script](https://github.com/huemul/subilo/blob/master/install.sh).
The script downloads the latest Subilo release and attempts to add the Subilo bin
path to the `$PATH` in the correct profile file (`~/.profile`, `~/.bashrc`,
`~/.bash_profile`, `~/.zshrc` or `~/.config/fish/config.fish`)

### Build source with Cargo

```
cargo install subilo
```

### Manually

Download the latest [released binary](https://github.com/huemul/subilo/releases)
and add executable permissions:

```
wget -O subilo "https://github.com/huemul/subilo/releases/download/v0.1.2/subilo-x86-64-linux"
chmod +x subilo
```

## Use

### Command line interface

Now that Subilo is available, the `help` subcommand can be run to display the
CLI information:

```
subilo --help
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

Create a `.subilorc` file with the required configuration. An example can be
found [here](/configuration.md).

### Start

To start the agent the `serve` command should be used specifying the
authentication secret, the port (optional), config file and logs directory
(optional).

Example:

```bash
subilo --secret super-secret serve --port 8089 --config /path/to/.subilorc
```

NOTE: at the moment, the API to display the deployment jobs status and logs is
based on these logs files.

### Authentication

To get access to the agent endpoints, create an authentication token using the
`token` command in the CLI.

#### Token with write permissions:

This token is used to access the POST `/webhook` endpoint and deploy
an application using the predefined commands in `.subilorc`.

Example:

```bash
subilo --secret "super-secret" token --permissions "job:write"
```

#### Token with only read permissions:

By default, the token only has read parmissions. In other words, only access the
logs and information endpoints. These endpoints can be used to see the status
and logs of the deployment jobs. They are what powers the
[subilo.io](https://subilo.io) website.

Example:

```bash
subilo --secret "super-secret" token
```

### Systemd configuration (Optional)

We recommend running Subilo with
[systemd](https://en.wikipedia.org/wiki/Systemd) to easily manage it. But that's
completely optional, you may run it however suits you.

Create a systemd service file (`/etc/systemd/system/subilo.service`) with the
following attributes:

```
[Unit]
Description=Subilo

[Service]
ExecStart=/path/to/subilo -s super-secret-secret serve -l /path/to/subilo-logs -p 8080 -c /path/to/.subilorc

[Install]
WantedBy=multi-user.target
```

Then enable and start Subilo service:

```bash
# Might require sudo
systemctl enable /etc/systemd/system/subilo.service
systemctl start subilo
```

To read logs and check status from systemctl, the following commands can be used:

```bash
systemctl status subilo
journalctl -u subilo -b
```

### Setup deployment webhooks

Once Subilo is running and exposed to the internet, deployment jobs can be
triggered by a POST request to the `/webhook` endpoint wiht the application's
name on the payload.

```bash
curl -X POST 'https://subilo.yourdomain.com/webhook' \
  -H 'Authorization: Bearer ********' \
  -H 'Content-Type: application/json' \
  -d '{ "name": "foo-app" }'
```

The name is matched against the `.subilorc` configuration file and the
specified commands are run to deploy the app.

#### CI

Usually, this webhook is trigger from a CI run, so after the application's tests
passed, it can be safely deployed. Store the token as a secret in the CI
configuration and add a curl command to POST to the `/webhook` endpoint to
trigger a deploy.

## Development

### Run

```bash
cargo run

# Watch mode
cargo watch -x run

# Setting CLI options
cargo run -- --port 9090 --logs-dir ./logs
```

### Test

```bash
cargo test

# Watch mode
cargo watch -x test
```

## LICENSE

MIT
[MIT License](/LICENSE) ©
[Christian Gill](https://gillchristian.xyz) and
[Nicoals Del Valle](https://github.com/ndelvalle)
