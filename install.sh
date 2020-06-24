#!/usr/bin/env bash

# Exit immediately if a command exits with a non-zero status.
set -e

# DOWNLOAD_URL="https://github.com/Huemul/thresh/releases/download/alpha_v4/thresh-x86-64-linux"
DOWNLOAD_URL="https://github.com/moneditas/api/releases/download/latest/moneditas-x86-64-linux"

default_install_dir() {
  [ -z "${XDG_CONFIG_HOME-}" ] && printf %s "${HOME}/.thresh" || printf %s "${XDG_CONFIG_HOME}/thresh"
}

main() {
  local _dir="$(default_install_dir)"
  local _file="${_dir}/thresh"
  
  # Do not re create the folder if it already exists, the user might have sensitive 
  # configuration on it.
  test -d "$_dir" || mkdir "$_dir"
  curl -L --show-error $DOWNLOAD_URL --output "$_file"  
  chmod +x "$_file"
}

main