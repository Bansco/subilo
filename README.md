# Thresh

🛳 Tiny deployment agent

![Rust](https://github.com/huemul/thresh/workflows/Rust/badge.svg)

## Install and setup

### Cargo

```
$ cargo install thresh
```

### Manually

Download the latest [released binary](https://github.com/Huemul/thresh/releases)
and add executable permissions:

```
$ wget -O thresh "https://github.com/Huemul/thresh/releases/download/v0.0.1/thresh-x86-64-linux"
$ chmod +x thresh
```

Now that Thresh is available, the `help` subcommand can be run to display the
CLI information:

```
$ ./thresh --help
thresh 0.0.1
Tiny deployment agent

USAGE:
    thresh [OPTIONS] <SUBCOMMAND>

FLAGS:
    -h, --help       Prints help information
    -V, --version    Prints version information

OPTIONS:
    -c, --config <config>    Path to Threshfile [default: .threshfile]
    -s, --secret <secret>    Secret to generate and authenticate the token. Can also be provided in the Threshfile

SUBCOMMANDS:
    help     Prints this message or the help of the given subcommand(s)
    serve    Start thresh agent server
    token    Create a token based on the secret to authorize agent connections
```

Next create a `.threshfile` with the required configuration to deploy projects.
Optionally this file can contain global Thresh configuration. A `threshfile`
example can be found
[here](https://github.com/Huemul/thresh/blob/master/sample.threshfile).

### Systemd configuration (Optional)

Create a systemd service file (`/etc/systemd/system/thresh.service`) with the
following attributes:

```
[Unit]
Description=Thresh

[Service]
ExecStart=/path/to/thresh -c /path/to/.threshfile -s super-secret-secret serve -l /path/to/thresh-logs -p 8080
```

Then enable and start Thresh service:

```bash
# Might require sudo
$ systemctl enable /etc/systemd/system/thresh.service
$ systemctl start thresh
```

To read logs and check status the following commands can be used:

```bash
$ systemctl status thresh
$ journalctl -u thresh -b
```

### Trigger jobs

Once Thresh is running and exposed to the internet, deployment jobs can be
triggered by POSTing to the `/webhook` endpoint wiht the project name.

```bash
curl -X POST 'https://thresh.yourdomain.com/webhook' \
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
