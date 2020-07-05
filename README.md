# Subilo

🛳 Tiny deployment agent

[![Rust](https://github.com/huemul/subilo/workflows/Rust/badge.svg)](https://github.com/Huemul/subilo/actions?query=workflow%3ARust)

## Install and setup

### Cargo

```
$ cargo install subilo
```

### Manually

Download the latest [released binary](https://github.com/Huemul/subilo/releases)
and add executable permissions:

```
$ wget -O subilo "https://github.com/Huemul/subilo/releases/download/v0.0.1/subilo-x86-64-linux"
$ chmod +x subilo
```

Now that Subilo is available, the `help` subcommand can be run to display the
CLI information:

```
$ ./subilo --help
subilo 0.0.1
Tiny deployment agent

USAGE:
    subilo [FLAGS] [OPTIONS] <SUBCOMMAND>

FLAGS:
    -h, --help       Prints help information
    -V, --version    Prints version information
    -v, --verbose    Makes Subilo verbose. Useful for debugging and seeing what's going on "under the hood"

OPTIONS:
    -c, --config <config>    Path to Subilofile [default: .subilofile]
    -s, --secret <secret>    Secret to generate and authenticate the token. Can also be provided in the Subilofile

SUBCOMMANDS:
    help     Prints this message or the help of the given subcommand(s)
    serve    Start subilo agent
    token    Create a token based on the secret to authorize agent connections
```

Next create a `.subilofile` with the required configuration to deploy projects.
Optionally this file can contain global Subilo configuration. A `subilofile`
example can be found
[here](https://github.com/Huemul/subilo/blob/master/sample.subilofile).

### Systemd configuration (Optional)

Create a systemd service file (`/etc/systemd/system/subilo.service`) with the
following attributes:

```
[Unit]
Description=Subilo

[Service]
ExecStart=/path/to/subilo -c /path/to/.subilofile -s super-secret-secret serve -l /path/to/subilo-logs -p 8080
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
