# Subilo projects configuration

Projects configuration can be setup using a `.subilorc` file using 
`toml` format:


```toml
# List of projects / applications to deploy

[[projects]]
# Unique project identifier
name = "foo-project"

# Path where the commands should run. The tilde (~) is properly expanded.
# The path does not have to be a git repo it can be any directory
path = "~/path/to/project/directory"

# List of commands to run to deploy the project
commands = [
  "git pull --rebase",
  "docker-compose down",
  "docker-compose up -d",
]

# Project's home page (optional)
home = "https://foo-project.com"

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