#!/usr/bin/env bash
# Scarica la libreria nativa Pdfium (build di bblanchon/pdfium-binaries) e la
# mette nella cartella di destinazione. Usato sia in sviluppo che nella CI.
#
# Uso:   scripts/fetch-pdfium.sh [cartella_destinazione]
#   default destinazione: src-tauri/pdfium
#
# Rileva sistema/architettura, ma si possono forzare con le variabili:
#   PDFIUM_OS   = linux | mac | win
#   PDFIUM_ARCH = x64 | arm64
set -euo pipefail

DEST="${1:-src-tauri/pdfium}"
REPO="bblanchon/pdfium-binaries"

# --- Rileva sistema operativo ---
os="${PDFIUM_OS:-}"
if [ -z "$os" ]; then
  case "$(uname -s)" in
    Linux*)  os="linux" ;;
    Darwin*) os="mac" ;;
    MINGW*|MSYS*|CYGWIN*) os="win" ;;
    *) echo "OS non riconosciuto: $(uname -s)" >&2; exit 1 ;;
  esac
fi

# --- Rileva architettura ---
arch="${PDFIUM_ARCH:-}"
if [ -z "$arch" ]; then
  case "$(uname -m)" in
    x86_64|amd64) arch="x64" ;;
    arm64|aarch64) arch="arm64" ;;
    *) echo "Architettura non riconosciuta: $(uname -m)" >&2; exit 1 ;;
  esac
fi

asset="pdfium-${os}-${arch}.tgz"
url="https://github.com/${REPO}/releases/latest/download/${asset}"

echo "Scarico ${asset} -> ${DEST}"
mkdir -p "$DEST"
tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT

curl -fL --retry 3 -o "$tmp/pdfium.tgz" "$url"
tar -xzf "$tmp/pdfium.tgz" -C "$tmp"

# La libreria sta in bin/ (Windows) o lib/ (Linux/mac).
lib="$(find "$tmp" -maxdepth 2 -type f \
  \( -name 'pdfium.dll' -o -name 'libpdfium.so' -o -name 'libpdfium.dylib' \) | head -1)"

if [ -z "$lib" ]; then
  echo "Libreria non trovata nell'archivio" >&2
  find "$tmp" -type f >&2
  exit 1
fi

cp "$lib" "$DEST/"
echo "OK: $(basename "$lib") in $DEST/"
