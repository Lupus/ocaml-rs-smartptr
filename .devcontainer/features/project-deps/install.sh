#!/usr/bin/env bash

set -euo pipefail

# Load EXISTING_NON_ROOT_USER from the common-utils feature file
source /usr/local/etc/vscode-dev-containers/common

if [ -z "${EXISTING_NON_ROOT_USER:-}" ]; then
  echo "Warning: EXISTING_NON_ROOT_USER is not set, defaulting to root"
  EXISTING_NON_ROOT_USER="root"
fi

if [ "${1:-}" != "--as-user" ]; then
  # Re-run the script as the target user, passing --as-user to avoid recursion
  SCRIPT=$(readlink -f "$0")
  PWD=$(pwd)
  exec su - "$EXISTING_NON_ROOT_USER" -c "bash -c \"cd $PWD && $SCRIPT --as-user\""
fi

export PATH="${ASDF_DATA_DIR:-$HOME/.asdf}/shims:$PATH"

#################################################################################################
##########                                 RUST                                         #########
#################################################################################################

# Download all dependencies including patches defined in Cargo.toml
echo "(*) Fetching cargo deps..."
cargo fetch

###################################################################################################
##########                                  OPAM                                          #########
###################################################################################################

# Install project dependencies
echo "(*) Installing project dependencies from opam..."
opam install  . --deps-only --with-test --assume-depexts --yes

echo "Setup complete!"
