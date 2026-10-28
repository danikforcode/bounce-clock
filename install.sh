#!/bin/sh
# bounce-clock — установка на Linux одной строкой:
#   curl -fsSL https://raw.githubusercontent.com/ЛОГИН/bounce-clock/main/install.sh | sh
# Удаление:
#   curl -fsSL https://raw.githubusercontent.com/ЛОГИН/bounce-clock/main/install.sh | sh -s -- --uninstall
#
# Переменные: BIN_DIR (куда ставить), BOUNCE_CLOCK_VERSION (например v0.0.2, по умолчанию последняя).
set -eu

REPO="${BOUNCE_CLOCK_REPO:-ЛОГИН/bounce-clock}"
NAME=bounce-clock

say() { printf '%s\n' "$*"; }
die() { printf 'ошибка: %s\n' "$*" >&2; exit 1; }

if [ -z "${BIN_DIR:-}" ]; then
    if [ "$(id -u)" = 0 ]; then BIN_DIR=/usr/local/bin; else BIN_DIR="$HOME/.local/bin"; fi
fi

if [ "${1:-}" = "--uninstall" ]; then
    rm -f "$BIN_DIR/$NAME" && say "удалено: $BIN_DIR/$NAME"
    say "конфиг остался в ~/.config/bounce-clock (удалите вручную, если не нужен)"
    exit 0
fi

fetch() {
    if command -v curl >/dev/null 2>&1; then curl -fsSL "$1" -o "$2"
    elif command -v wget >/dev/null 2>&1; then wget -qO "$2" "$1"
    else die "нужен curl или wget"; fi
}

from_source() {
    command -v cargo >/dev/null 2>&1 || die "готовой сборки нет, а cargo не найден. Установите Rust: https://rustup.rs"
    say "собираю из исходников через cargo…"
    cargo install --git "https://github.com/$REPO" --locked
    exit 0
}

[ "$(uname -s)" = Linux ] || from_source

case "$(uname -m)" in
    x86_64|amd64)  ARCH=x86_64 ;;
    aarch64|arm64) ARCH=aarch64 ;;
    *) say "архитектура $(uname -m) не собрана заранее"; from_source ;;
esac

TARGET="$ARCH-unknown-linux-musl"
FILE="$NAME-$TARGET.tar.gz"
if [ -n "${BOUNCE_CLOCK_VERSION:-}" ]; then
    URL="https://github.com/$REPO/releases/download/$BOUNCE_CLOCK_VERSION/$FILE"
else
    URL="https://github.com/$REPO/releases/latest/download/$FILE"
fi

TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT INT TERM

say "скачиваю $FILE…"
fetch "$URL" "$TMP/$FILE" || { say "релиз не найден"; from_source; }

if fetch "$URL.sha256" "$TMP/$FILE.sha256" 2>/dev/null && command -v sha256sum >/dev/null 2>&1; then
    (cd "$TMP" && sha256sum -c "$FILE.sha256" >/dev/null) || die "контрольная сумма не совпала"
fi

tar -xzf "$TMP/$FILE" -C "$TMP"
mkdir -p "$BIN_DIR"
install -m 755 "$TMP/$NAME-$TARGET/$NAME" "$BIN_DIR/$NAME"
say "установлено: $BIN_DIR/$NAME"

case ":$PATH:" in
    *":$BIN_DIR:"*) ;;
    *) say ""
       say "$BIN_DIR нет в PATH. Добавьте в ~/.bashrc или ~/.zshrc:"
       say "  export PATH=\"$BIN_DIR:\$PATH\"" ;;
esac
say "запуск: $NAME"
