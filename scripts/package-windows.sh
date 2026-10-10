#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
OUT="$ROOT/dist/Maze-Windows-Portable"
BIN="/ucrt64/bin"
SHARE="/ucrt64/share"
EXE="$ROOT/target/x86_64-pc-windows-gnu/release/maze.exe"
rm -rf "$OUT" "$ROOT/dist/Maze-Windows-Portable.zip"
mkdir -p "$OUT"
cp "$EXE" "$OUT/maze.exe"

# Copy only the recursive DLL import closure instead of every DLL in UCRT64.
# This is much smaller than bundling the entire MSYS2 runtime directory.
declare -A copied=()
queue=("$EXE")
while ((${#queue[@]})); do
  current="${queue[0]}"
  queue=("${queue[@]:1}")
  while IFS= read -r dll; do
    [[ -n "$dll" ]] || continue
    key="${dll,,}"
    [[ -n "${copied[$key]:-}" ]] && continue
    source="$(find "$BIN" -maxdepth 1 -type f -iname "$dll" -print -quit)"
    if [[ -n "$source" ]]; then
      cp -f "$source" "$OUT/"
      copied[$key]=1
      queue+=("$source")
    fi
  done < <(objdump -p "$current" 2>/dev/null | sed -n 's/^[[:space:]]*DLL Name: //p' | tr -d '\r')
done

# GTK loads pixbuf loaders dynamically, so bundle the loader modules and cache.
# These are small compared with copying every DLL from the toolchain.
PIXBUF_DIR="$BIN/../lib/gdk-pixbuf-2.0/2.10.0/loaders"
if [[ -d "$PIXBUF_DIR" ]]; then
  mkdir -p "$OUT/lib/gdk-pixbuf-2.0/2.10.0/loaders"
  cp -f "$PIXBUF_DIR"/*.dll "$OUT/lib/gdk-pixbuf-2.0/2.10.0/loaders/" 2>/dev/null || true
  if [[ -f "$PIXBUF_DIR/loaders.cache" ]]; then
    cp -f "$PIXBUF_DIR/loaders.cache" "$OUT/lib/gdk-pixbuf-2.0/2.10.0/"
  fi
fi

# GTK/Libadwaita runtime data required for themes, schemas, and icons.
for item in \
  glib-2.0/schemas \
  icons \
  themes \
  gtk-4.0 \
  libadwaita-1 \
  gdk-pixbuf-2.0; do
  if [[ -e "$SHARE/$item" ]]; then
    mkdir -p "$OUT/share/$(dirname "$item")"
    cp -a "$SHARE/$item" "$OUT/share/$item"
  fi
done
if [[ -d /ucrt64/etc/gtk-4.0 ]]; then
  mkdir -p "$OUT/etc"
  cp -a /ucrt64/etc/gtk-4.0 "$OUT/etc/"
fi

cat > "$OUT/Run-Maze.bat" <<'BAT'
@echo off
setlocal
set "APP_DIR=%~dp0"
set "PATH=%APP_DIR%;%APP_DIR%\bin;%PATH%"
set "XDG_DATA_DIRS=%APP_DIR%\share"
set "GSETTINGS_SCHEMA_DIR=%APP_DIR%\share\glib-2.0\schemas"
set "GTK_DATA_PREFIX=%APP_DIR%"
set "GTK_EXE_PREFIX=%APP_DIR%"
set "GDK_PIXBUF_MODULEDIR=%APP_DIR%\lib\gdk-pixbuf-2.0\2.10.0\loaders"
set "GDK_PIXBUF_MODULE_FILE=%APP_DIR%\lib\gdk-pixbuf-2.0\2.10.0\loaders.cache"
set "GTK_THEME=Adwaita"
start "" "%APP_DIR%maze.exe"
BAT

cat > "$OUT/README-Windows.txt" <<'TXT'
Maze - Portable Windows build

Run Run-Maze.bat. Keep the extracted folder together. The package includes
only the recursive DLL import dependencies plus GTK's dynamically loaded
pixbuf modules and runtime data, rather than every DLL from MSYS2 UCRT64.
TXT

mkdir -p "$ROOT/dist"
cd "$ROOT/dist"
zip -qr Maze-Windows-Portable.zip Maze-Windows-Portable
