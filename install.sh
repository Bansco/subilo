#!/usr/bin/env bash

# Exit immediately if a command exits with a non-zero status.
set -e

reset="\033[0m"
red="\033[31m"
green="\033[32m"
cyan="\033[36m"

DOWNLOAD_URL="https://github.com/Huemul/thresh/releases/download/alpha_v4/thresh_x86-64-linux"

# the following function was brought from https://yarnpkg.com/install.sh
# https://github.com/yarnpkg/yarn/blob/master/LICENSE
detect_profile() {
  if [ -n "${PROFILE}" ] && [ -f "${PROFILE}" ]; then
    echo "${PROFILE}"
    return
  fi

  local DETECTED_PROFILE
  DETECTED_PROFILE=''
  local SHELLTYPE
  SHELLTYPE="$(basename "/$SHELL")"

  if [ "$SHELLTYPE" = "bash" ]; then
    if [ -f "$HOME/.bashrc" ]; then
      DETECTED_PROFILE="$HOME/.bashrc"
    elif [ -f "$HOME/.bash_profile" ]; then
      DETECTED_PROFILE="$HOME/.bash_profile"
    fi
  elif [ "$SHELLTYPE" = "zsh" ]; then
    DETECTED_PROFILE="$HOME/.zshrc"
  elif [ "$SHELLTYPE" = "fish" ]; then
    DETECTED_PROFILE="$HOME/.config/fish/config.fish"
  fi

  if [ -z "$DETECTED_PROFILE" ]; then
    if [ -f "$HOME/.profile" ]; then
      DETECTED_PROFILE="$HOME/.profile"
    elif [ -f "$HOME/.bashrc" ]; then
      DETECTED_PROFILE="$HOME/.bashrc"
    elif [ -f "$HOME/.bash_profile" ]; then
      DETECTED_PROFILE="$HOME/.bash_profile"
    elif [ -f "$HOME/.zshrc" ]; then
      DETECTED_PROFILE="$HOME/.zshrc"
    elif [ -f "$HOME/.config/fish/config.fish" ]; then
      DETECTED_PROFILE="$HOME/.config/fish/config.fish"
    fi
  fi

  if [ ! -z "$DETECTED_PROFILE" ]; then
    echo "$DETECTED_PROFILE"
  fi
}

install_thresh() {
  local INSTALL_DIR="${HOME}/.thresh/bin"
  local INSTALL_FILE="${INSTALL_DIR}/thresh"
  
  # Do not re create the folder if it already exists, the user might have sensitive 
  # configuration on it.
  test -d "$INSTALL_DIR" || mkdir -p "$INSTALL_DIR"
  curl --location --show-error --progress-bar $DOWNLOAD_URL --output "$INSTALL_FILE"  
  chmod +x "$INSTALL_FILE"

  # Add Thresh bin to PATH
  # the following code block was brought from https://yarnpkg.com/install.sh
  # https://github.com/yarnpkg/yarn/blob/master/LICENSE
  THRESH_PROFILE="$(detect_profile)"
  SOURCE_STR="\nexport PATH=\"\$HOME/.thresh/bin:\$PATH\"\n"

  if [ -z "${THRESH_PROFILE-}" ] ; then
    printf "$red> Profile not found. Tried ${THRESH_PROFILE} (as defined in \$PROFILE), ~/.bashrc, ~/.bash_profile, ~/.zshrc, and ~/.profile.\n"
    echo "> Create one of them and run this script again"
    echo "> Create it (touch ${THRESH_PROFILE}) and run this script again"
    echo "   OR"
    printf "> Append the following lines to the correct file yourself:$reset\n"
    command printf "${SOURCE_STR}"
  else
    if ! grep -q 'thresh/bin' "$THRESH_PROFILE"; then
      if [[ $THRESH_PROFILE == *"fish"* ]]; then
        command fish -c 'set -U fish_user_paths $fish_user_paths ~/.thresh/bin'
        printf "$cyan> We've added ~/.thresh/bin to your fish_user_paths universal variable\n"
      else
        command printf "$SOURCE_STR" >> "$THRESH_PROFILE"
        printf "$cyan> We've added the following to your $THRESH_PROFILE\n"
      fi
      
      echo "> If this isn't the profile of your current shell then please add the following to your correct profile:"
      printf "   $SOURCE_STR$reset\n"
    fi

    version=`$HOME/.thresh/bin/thresh --version` || (
      printf "$red> Thresh was installed, but doesn't seem to be working :(.$reset\n"
      exit 1;
    )

    printf "$green> Successfully installed Thresh $version! Please source the profile or open another terminal where the \`thresh\` command will now be available.$reset\n"
  fi
}

install_thresh
