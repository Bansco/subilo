# Thresh

🛳 Tiny continuous deployment server for VPS

![Rust](https://github.com/huemul/thresh/workflows/Rust/badge.svg)

## Install and setup

Download the latest [released binary](https://github.com/Huemul/thresh/releases) and mark the file as executable with the chmod command:

```
$ wget -O thresh "https://github.com/Huemul/thresh/releases/download/v0.0.1/thresh-x86-64-linux"
$ chmod +x thresh
```

Now that Thresh is available, the `help` subcommand can be run to display the CLI interface:

```
$ ./thresh --help
thresh 0.0.1
gillchristian <gillchristiang@gmail.com>, ndelvalle <nicolas.delvalle@gmail.com>
Tiny continuous deployment server for VPS

USAGE:
    thresh <SUBCOMMAND>

FLAGS:
    -h, --help       Prints help information
    -V, --version    Prints version information

SUBCOMMANDS:
    help     Prints this message or the help of the given subcommand(s)
    start    Start thresh agent
    token    Create a token based on the specified secret to authorize agent connections
```

Next create a `.threshfile` with the required configuration to deploy projects. Optionally this file can contain global Thresh configuration.
A `threshfile` example can be found [here](https://github.com/Huemul/thresh/blob/master/sample.threshfile).


## Systemd configuration (Optional)

Create a systemd service file (`/etc/systemd/system/thresh.service`) with the following attributes:

```
[Unit]
Description=Thresh

[Service]
ExecStart=/path/to/thresh -c /path/to/.threshfile -l /path/to/thresh-logs -p 8080
```

Then enable and start Thresh service:

```bash
# These commands might require sudo
$ systemctl enable /etc/systemd/system/thresh.service
$ systemctl start thresh
```

To read logs and check status the following commands can be used:

```bash
$ systemctl status thresh
$ journalctl -u thresh -b
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

### Testing the webhook locally

```bash
curl -X POST 'http://localhost:8080/webhook' \
--header 'Authorization: Bearer ********' \
--header 'Content-Type: application/json' \
--data-raw '{
    "name": "foo-project"
}'
```
