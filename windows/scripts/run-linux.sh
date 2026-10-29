#!/bin/sh
# Codenotch on Linux.
#
# A shell started from a snap (VS Code, for one) exports that snap's library
# paths, and they break a binary built against the system glibc. The binary
# chooses Wayland layer-shell or X11 itself.
set -eu

root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
bin=${CODENOTCH_BIN:-$root/target/release/codenotch}
[ -x "$bin" ] || bin=$root/target/debug/codenotch

if [ ! -x "$bin" ]; then
  echo "No binary yet. Build one first:  cargo build --release -p codenotch" >&2
  exit 1
fi

exec env -u LD_LIBRARY_PATH -u GTK_PATH -u GIO_MODULE_DIR -u GSETTINGS_SCHEMA_DIR \
         -u LOCPATH -u GDK_PIXBUF_MODULE_FILE -u GDK_PIXBUF_MODULEDIR \
    "$bin" "$@"
