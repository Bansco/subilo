# Subilo configuration

Configuration for the deployment of applications is done using a `.subilorc` file
(`toml` format):


```toml
# List of applications to deploy

[[projects]]
# Unique application identifier
name = "foo-app"

# Path where the commands should run. The tilde (~) is properly expanded.
# The path does not have to be a git repo it can be any directory
path = "~/path/to/app/directory"

# List of commands to run to deploy the application
commands = [
  "git pull --rebase",
  "docker-compose down",
  "docker-compose up -d",
]

# Project's home page (optional)
home = "https://foo.com"

# Project's CI page (optional)
ci = "https://app.circleci.com/pipelines/github/bar/foo"

# Project's repository (optional)
repo = "https://github.com/bar/foo"


[[projects]]
name = "sarasa"
path = "~/path/to/sarasa"
commands = [
  "git pull",
  "./restart-serever.sh",
]


[[projects]]
name = "baz"
path = "~/path/to/baz"
commands = [
  "git pull",
  "systemctl restart yet_another"
]
```
