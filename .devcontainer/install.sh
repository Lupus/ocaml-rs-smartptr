#!/usr/bin/env bash

set -euo pipefail

sudo apt-get update

echo "(*) Installing fzf..."
sudo apt-get install fzf

echo "(*) Installing patdiff..."
opam install patdiff
eval $(opam env)

RUST_VERSION=$(rustc --version | cut -d' ' -f2)
if [ "$(printf '%s\n' "1.70.0" "$RUST_VERSION" | sort -V | head -n1)" != "1.70.0" ]; then
    echo "Requires Rust 1.70+, found $RUST_VERSION" >&2
    exit 1
fi

# Install git-split
echo "(*) Installing git-split..."
(
  git clone https://github.com/tomjaguarpaw/git-split.git /tmp/git-split
  cd /tmp/git-split
  git -c advice.detachedHead=false checkout 2b723a7d859f4b6e568c087576a5dc6978df5047
  sudo cp split.sh /usr/local/bin/git-split
  sudo chmod +x /usr/local/bin/git-split
  rm -rf /tmp/git-split
)

# Install git-subrepo
echo "(*) Installing git-subrepo..."
(
  git clone https://github.com/ingydotnet/git-subrepo.git /tmp/git-subrepo
  cd /tmp/git-subrepo
  git -c advice.detachedHead=false checkout ec1d487312d6f20473b7eac94ef87d8bde422f8b # Release 0.4.9
  sudo make install
  rm -rf /tmp/git-subrepo
)

# Install git-imerge
echo "(*) Installing git-imerge..."
pipx install git-imerge

if ! command -v cargo &> /dev/null; then
  echo "Error: cargo not found. Requires Rust installation." >&2
  exit 1
fi

# Install git-absorb
echo "(*) Installing git-absorb..."
cargo install git-absorb

# Install git-interactive-rebase-tool
echo "(*) Installing git-interactive-rebase-tool..."
cargo install git-interactive-rebase-tool

git config --global sequence.editor interactive-rebase-tool
git config --global diff.external $(which patdiff)

echo "Done!"
