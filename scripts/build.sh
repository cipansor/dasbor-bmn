#!/usr/bin/env bash
# Bangun ulang aset dasbor dari sumber: CSS Tailwind, WASM hidrasi + glue JS, dan
# biner SSR. Hasilnya cukup untuk menjalankan `scripts/serve.sh`.
#
# Penting: hidrasi Leptos mencocokkan pohon view di WASM dengan HTML yang dikirim
# server. Bila WASM tertinggal dari sumber, penanda hidrasi bergeser dan peramban
# gagal total ("expected a marker node"), sehingga semua tombol mati. Karena itu
# WASM wajib dibangun ulang setiap kali `src/` berubah, dan harus dihasilkan lewat
# `wasm-bindgen` agar glue JS merujuk berkas `*_bg.wasm` yang benar.
#
# Perkakas: cargo, wasm-bindgen, dan Tailwind CSS v4 (standalone CLI `tailwindcss`,
# atau paket npm yang dikunci di package.json). Skrip berhenti dengan galat bila
# salah satu langkah gagal, termasuk pembuatan CSS.
#
# Pemakaian: scripts/build.sh [debug|release]

set -eu

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

OUTPUT_NAME="${LEPTOS_OUTPUT_NAME:-dasbor-bmn}"
SITE_ROOT="${LEPTOS_SITE_ROOT:-target/site}"
PKG_DIR="${LEPTOS_SITE_PKG_DIR:-pkg}"
PKG="$SITE_ROOT/$PKG_DIR"

# Profil build: `debug` (bawaan, cepat untuk pengembangan) atau `release`
# (`scripts/build.sh release`) untuk penempatan. Build debug menghasilkan WASM
# puluhan megabita; jangan dipakai untuk produksi.
PROFIL="${1:-${DASBOR_PROFIL:-debug}}"
case "$PROFIL" in
  debug) CARGO_FLAG=""; WASM_FLAG="" ;;
  release) CARGO_FLAG="--release"; WASM_FLAG="--profile wasm-release" ;;
  *) echo "Profil tidak dikenal: $PROFIL (pakai debug atau release)" >&2; exit 1 ;;
esac
WASM_PROFIL_DIR="$PROFIL"
[ "$PROFIL" = "release" ] && WASM_PROFIL_DIR="wasm-release"

# Target WASM terpisah dari target host agar cache tidak saling mengganggu.
WASM_TARGET_DIR="target/front"
WASM_ARTIFACT="$WASM_TARGET_DIR/wasm32-unknown-unknown/$WASM_PROFIL_DIR/dasbor_bmn.wasm"

# Biner SSR wajib mendarat di target/debug karena dari situlah scripts/serve.sh
# menjalankannya. Nilai ini dipaksa, bukan diwarisi: bila CARGO_TARGET_DIR di
# lingkungan sudah terisi (mis. target/front untuk WASM), biner akan tertulis di
# tempat lain dan serve.sh diam-diam menjalankan biner basi — sumber perubahan yang
# tidak pernah ikut terpakai.
HOST_TARGET_DIR="target"

need() {
  command -v "$1" >/dev/null 2>&1 || {
    echo "Alat '$1' tidak ditemukan. Pasang lebih dulu." >&2
    exit 1
  }
}
need cargo
need wasm-bindgen
need rustup

# Tailwind CSS v4: pakai CLI standalone bila ada; jika tidak, pakai paket npm
# yang versinya dikunci di package.json. `npx @tailwindcss/cli` tanpa paket
# `tailwindcss` terpasang gagal me-resolve `@import "tailwindcss"`, jadi
# dependensi dipasang lebih dulu.
TAILWIND=""
if command -v tailwindcss >/dev/null 2>&1; then
  TAILWIND="tailwindcss"
elif command -v npm >/dev/null 2>&1; then
  if [ ! -x node_modules/.bin/tailwindcss ]; then
    echo "==> Memasang Tailwind CSS (npm ci)"
    npm ci --no-audit --no-fund
  fi
  TAILWIND="node_modules/.bin/tailwindcss"
else
  echo "Alat Tailwind CSS tidak ditemukan (butuh 'tailwindcss' atau 'npm')." >&2
  exit 1
fi

mkdir -p "$PKG"

echo "==> 1/4 Bangun CSS Tailwind -> $PKG/$OUTPUT_NAME.css"
# Dihasilkan lebih dulu agar dasbor tidak pernah tersaji tanpa gaya, termasuk
# pada checkout bersih yang belum punya CSS sama sekali.
$TAILWIND -i style/tailwind.css -o "$PKG/$OUTPUT_NAME.css" --minify
if [ ! -s "$PKG/$OUTPUT_NAME.css" ]; then
  echo "GAGAL: CSS tidak terbentuk di $PKG/$OUTPUT_NAME.css" >&2
  exit 1
fi

echo "==> 2/4 Bangun WASM hidrasi"
rustup target add wasm32-unknown-unknown >/dev/null 2>&1 || true
# shellcheck disable=SC2086
CARGO_TARGET_DIR="$WASM_TARGET_DIR" cargo build $WASM_FLAG \
  --lib --target wasm32-unknown-unknown \
  --no-default-features --features hydrate

echo "==> 3/4 Hasilkan glue JS + WASM lewat wasm-bindgen"
wasm-bindgen --target web --out-dir "$PKG" --out-name "$OUTPUT_NAME" "$WASM_ARTIFACT"

# Nama berkas WASM yang diminta HTML ditentukan Leptos saat biner SSR dikompilasi:
# akhiran "_bg" hanya dipakai bila LEPTOS_OUTPUT_NAME tidak diset waktu itu
# (leptos/src/hydration/mod.rs). Karena nilai itu bisa terisi di lingkungan,
# sediakan KEDUA nama agar HTML selalu menemukan berkas yang benar. Hard link
# dipakai supaya tidak menggandakan berkas puluhan MB.
WASM_BG="$PKG/${OUTPUT_NAME}_bg.wasm"
WASM_HTML="$PKG/${OUTPUT_NAME}.wasm"
# Hapus dulu: bila keduanya sudah saling ter-link, `ln`/`cp` menolak berkas yang
# sama dan henti `set -e` akan mematikan build ulang.
rm -f "$WASM_HTML"
ln "$WASM_BG" "$WASM_HTML" 2>/dev/null || cp "$WASM_BG" "$WASM_HTML"

if ! cmp -s "$WASM_BG" "$WASM_HTML"; then
  echo "GAGAL: $WASM_HTML tidak identik dengan $WASM_BG; hidrasi akan gagal." >&2
  exit 1
fi

echo "==> 4/4 Bangun biner SSR -> $HOST_TARGET_DIR/$PROFIL/dasbor-bmn"
# shellcheck disable=SC2086
CARGO_TARGET_DIR="$HOST_TARGET_DIR" cargo build $CARGO_FLAG --no-default-features --features ssr

if [ ! -x "$HOST_TARGET_DIR/$PROFIL/dasbor-bmn" ]; then
  echo "GAGAL: biner SSR tidak ada di $HOST_TARGET_DIR/$PROFIL/dasbor-bmn" >&2
  exit 1
fi

echo "Selesai ($PROFIL). Aset di $PKG, biner di $HOST_TARGET_DIR/$PROFIL/dasbor-bmn."
[ "$PROFIL" = "release" ] && echo "Jalankan: DASBOR_BIN=$HOST_TARGET_DIR/release/dasbor-bmn scripts/serve.sh"
exit 0
