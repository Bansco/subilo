# Tresh

🛳 Tiny GitHub webhooks based CI/CD server for your VPS

## Install & setup

Download the latest binary from the release and give exec permission:

```
$ wget -O thresh "https://github.com/Huemul/thresh/releases/download/v0.0.1/thresh_x86_64-ubuntu"
$ chmod +x thresh
```

**NOTE**: you probably want to change the version (`v0.0.1`) to the [latest available release](https://github.com/Huemul/thresh/releases).

Now that Thresh is available:

```
$ ./thresh --help
thresh 0.0.1
gillchristian <gillchristiang@gmail.com>:ndelvalle <nicolas.delvalle@gmail.com>
Tiny GitHub webhooks based CI/CD server for your VPS

USAGE:
    thresh [OPTIONS]

FLAGS:
    -h, --help       Prints help information
    -V, --version    Prints version information

OPTIONS:
    -c, --config <config>        Path to the Threshfile [default: ./.threshfile]
    -l, --logs-dir <logs-dir>    Sets a custom logs directory
    -p, --port <port>            Sets a custom server port
```

Next create a `.threshfile` with the configuration to run for any project you want. For a example threshfile see [sample.threshfile](https://github.com/Huemul/thresh/blob/master/sample.threshfile).

Create a systemd file (`/etc/systemd/system/thresh.service`) with the following contents.

```
[Unit]
Description=Thresh

[Service]
ExecStart=/path/to/thresh -c /path/to/.threshfile -l /path/to/thresh-logs -p 8080
```

**NOTE**: Make sure to update it with the right options and path to the Thresh binary.

Now enable and start thresh service:

```bash
# might require sudo
$ systemctl enable /etc/systemd/system/thresh.service
$ systemctl start thresh
```

To see logs and status the following commands are useful:

```bash
$ systemctl status thresh
$ journalctl -u thresh -b
```

Once Thresh is running and exposed to the internet on your VPS is time to [add the GitHub webhook to a repo](https://developer.github.com/webhooks/creating/).

Create a webhook the sends `push` events to the webhook URL (`<domain-running-thresh>/webhook`).

Thresh responds wih a "job id" in the case a webhook triggered a job. Which can be used to see the log file online:

```
GET <domain-running-thresh>/logs
GET <domain-running-thresh>/logs/:{job_id}
```

## Development

#### Testing 

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
curl -d {
  "ref": "refs/heads/master",
  "repository": { "full_name": "username/repository" },
  "sender": { "login": "username" }
}' -H "Content-Type: application/json" -X POST http://localhost:8080/webhook
```

:point_up: That can also be used to trigger the webhook from other sources (eg. any CI/CD server). In that case, make sure the URL is passed by secrets.
