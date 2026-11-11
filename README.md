# Dasbor BMN — Kejaksaan RI

Dasbor analitik **Barang Milik Negara (BMN)** Kejaksaan Republik Indonesia di
atas **Web Service SLDK SIMAN v2** (DJKN, Kementerian Keuangan RI), dirancang
untuk dibaca pimpinan: apa posisi aset hari ini, apa risikonya, apa yang akan
terjadi lima tahun ke depan, berapa kebutuhannya, dan apa yang perlu dilakukan —
sampai rincian tiap satuan kerja.

Dibangun dengan **Rust**: Leptos 0.8 (SSR + hidrasi) di atas Axum, SQLite
sebagai penyimpanan lokal, Tailwind CSS v4, serta grafik SVG dan penulis PDF
buatan sendiri (tanpa pustaka JavaScript).

> Hasil audit atas versi awal (PR #1) dan daftar hal yang masih perlu divalidasi
> dengan data nyata ada di [`docs/AUDIT-PR1.md`](docs/AUDIT-PR1.md).

## Halaman

| Kelompok | Halaman | Isi |
| --- | --- | --- |
| Pimpinan | **Ringkasan Eksekutif** `/` | KPI utama, sorotan berkalimat, pandangan lima tahun, rekomendasi prioritas, kondisi per kategori, satker perlu perhatian |
| | **Kondisi & Risiko** `/kondisi` | Kondisi dan masa manfaat per kategori, kondisi menurut umur, kurva kerusakan data vs model, kerusakan di atas kewajaran umur, aset rusak berat terbesar |
| | **Prediksi 5 Tahun** `/prediksi` | Jadwal habis masa manfaat dan perkiraan rusak berat per tahun, per kategori, per satker; aset paling berisiko; parameter model |
| | **Analisis Kebutuhan** `/kebutuhan` | Kebutuhan penggantian/rehabilitasi tiga skenario, biaya per tahun, kelompok barang yang perlu diadakan, potensi aset idle |
| | **Forecast Perolehan** `/prakiraan` | Prakiraan perolehan per kategori dengan rentang ketidakpastian dan galat uji balik, proyeksi nilai buku |
| | **Rekomendasi** `/rekomendasi` | Tindak lanjut berprioritas: temuan berangka, langkah, satker terkait, rujukan ketentuan |
| Rincian | **Satuan Kerja** `/satker`, `/satker/:kode` | Peringkat seluruh satker (cari, saring, urut) dan profil lengkap tiap satker |
| | **Wilayah** `/wilayah` | Perbandingan antarprovinsi |
| | **Kategori Aset** `/kategori` | 15 kategori dan struktur 106 kolom |
| | **Data Aset** `/data` | Telusur, saring, urut, paginasi, ekspor CSV |
| Tata kelola | **Mutu Data** `/kualitas` | Kelengkapan isian, cakupan PSP dan sertipikat, anomali |
| | **Laporan PDF** `/laporan` | Laporan nasional dan laporan per satuan kerja |
| | **Metodologi** `/metodologi` | Cara hitung, asumsi, keterbatasan |
| Sistem | **Sinkronisasi**, **Pengaturan** | Tarik data di latar dengan kemajuan; kredensial dan asumsi analitik |

Unduhan: `/api/laporan.pdf?periode=YYYY-MM[&satker=KODE]`, `/api/satker.csv`,
`/api/kebutuhan.csv`, `/api/ekspor.csv?tabel=…`.

## Cara analitik bekerja (ringkas)

Penjelasan lengkap ada di halaman **Metodologi**.

- **Habis masa manfaat** — mengikuti `umur_sisa` dari SIMAN; bila kosong atau
  janggal, ditaksir dari porsi nilai buku. Bersifat pasti.
- **Prediksi rusak berat** — model dua tahap *baik → rusak ringan → rusak
  berat*. Kurva "masih baik menurut umur" (Weibull) dan laju rusak ringan ke
  rusak berat ditaksir per kategori dari potret umur–kondisi seluruh aset. Tiap
  aset mendapat peluang menurut kategori, umur, kondisi kini, dan rekam
  kerusakan satkernya. Bersifat peluang, cenderung batas bawah.
- **Analisis kebutuhan** — tiga skenario: *dasar* (hanya rusak berat),
  *moderat* (ditambah rusak ringan yang habis masa manfaat), *maksimal*
  (peremajaan penuh). Harga satuan dari median perolehan lima tahun terakhir
  per kelompok barang.
- **Forecast** — tiga metode deret waktu diadu lewat uji balik; yang bergalat
  terkecil dipakai dan galatnya ditampilkan.
- **Skor kesehatan & RKU** — skor 0–100 per satker, dan Rasio Kerusakan
  terstandar Umur (kerusakan teramati dibanding yang wajar menurut umur aset).
- **Rekomendasi** — aturan baku yang menghasilkan temuan berangka, langkah, dan
  rujukan ketentuan (penunjuk awal; pastikan ketentuan yang berlaku).

## Mode data

| Mode | Kapan | Perilaku |
| --- | --- | --- |
| **Contoh** | Kredensial SLDK belum diisi | Data ilustratif deterministik; setiap halaman dan setiap halaman PDF bertanda "data contoh — bukan data riil" |
| **Langsung** | `BA_KEY`, Client ID, Client Secret terisi | Data ditarik dari gateway SLDK; data contoh dibuang saat tabel pertama berhasil disimpan |

## Menjalankan

```bash
# Perkakas: Rust, target WASM, wasm-bindgen (versi sama dengan Cargo.lock), npm
rustup target add wasm32-unknown-unknown
cargo install wasm-bindgen-cli --version 0.2.129 --locked

# Kredensial (opsional — tanpa ini dasbor memakai data contoh)
cp .env.example .env          # lalu isi nilainya dan muat ke environment

scripts/build.sh              # build pengembangan (cepat, berkas besar)
scripts/build.sh release      # build penempatan (biner ±20 MB, WASM ±2,3 MB)

scripts/serve.sh                                              # jalankan build pengembangan
DASBOR_BIN=target/release/dasbor-bmn scripts/serve.sh         # jalankan build rilis
```

`scripts/build.sh` membangun CSS Tailwind (CLI mandiri bila ada, jika tidak
`npm ci` dari `package.json`), WASM hidrasi lewat `wasm-bindgen`, dan biner SSR.
Jalur `cargo leptos watch/build` juga tersedia lewat `[package.metadata.leptos]`
tetapi belum diuji pada rombakan ini.

> **Penting.** Hidrasi mencocokkan pohon tampilan di WASM dengan HTML dari
> server. Jalankan ulang `scripts/build.sh` setiap kali `src/` berubah.

### Konfigurasi

| Variabel | Keterangan | Bawaan |
| --- | --- | --- |
| `SIMAN_BASE_URL` | Akar gateway KSB | `https://apigateway.kemenkeu.go.id` |
| `SIMAN_TOKEN_URL` | Endpoint token SSO | `https://sso.kemenkeu.go.id/connect/token` |
| `SIMAN_CLIENT_ID`, `SIMAN_CLIENT_SECRET` | Kredensial dari Kemenkeu | — |
| `SIMAN_GRANT_TYPE` | Tipe grant OAuth2 | `client_credentials` |
| `SIMAN_BA_KEY` | Kode BA Kejaksaan | — |
| `SIMAN_DATA_DIR` | Direktori data lokal | `data` |
| `DASBOR_PASSWORD` | Kata sandi gerbang dasbor | dibuat otomatis di `data/auth.json` |
| `BMN_ESKALASI_HARGA` | Eskalasi harga per tahun (pecahan) | `0.03` |
| `BMN_FAKTOR_REHAB` | Biaya rehabilitasi berat sebagai porsi nilai bangunan | `0.65` |
| `BMN_SATUAN_UMUR_SISA` | Satuan kolom `umur_sisa`: `tahun`, `semester`, `bulan` | `tahun` |

Kredensial dan asumsi analitik juga dapat diisi lewat halaman **Pengaturan**
(tersimpan di `data/config.json`, mengalahkan environment).

## Arsitektur

```
src/
  main.rs          Server Axum: rute halaman, unduhan, gerbang masuk
  app.rs           Kerangka HTML dan perutean halaman
  auth.rs          Gerbang masuk, sesi, pembatasan percobaan, kepala keamanan
  config.rs        Kredensial dan asumsi analitik (environment + berkas)
  catalog.rs       15 tabel Master Aset beserta sifat analitik tiap kategori
  schema.rs        106 kolom sesuai panduan SLDK
  sldk.rs          Klien Web Service SLDK (token SSO, hitung baris, tarik baris)
  sinkron.rs       Pekerjaan sinkronisasi latar dengan kemajuan dan percobaan ulang
  fakta.rs         Normalisasi baris mentah menjadi kolom fakta
  store.rs         SQLite: baris + kolom fakta, generasi, telusur, riwayat posisi
  demo.rs          Pembangkit data contoh
  analitik/
    kerusakan.rs   Model kerusakan dua tahap dan penaksirannya
    prakiraan.rs   Forecast deret tahunan dengan uji balik
    mesin.rs       Agregasi, proyeksi, kebutuhan, skor per satker/kategori/provinsi
    rekomendasi.rs Sorotan dan rekomendasi berbasis aturan
    mod.rs         Tembolok hasil, rincian satker, perolehan per periode
  dto.rs           Struktur hasil analitik (dipakai server dan peramban)
  model.rs         Tipe dasar dan pemformatan angka/rupiah
  server_fns.rs    Server functions Leptos
  components/      Kerangka navigasi, komponen UI, grafik SVG
  pages/           16 halaman
  report/          Penulis PDF, tata letak, grafik PDF, penyusun laporan, unduhan
```

### Alur data

1. **Token** — `client_credentials` ke SSO Kemenkeu, disimpan selama berlaku.
2. **Hitung** — `getRowCount/{BA_KEY}/{TABEL}`.
3. **Tarik** — resource aset dengan `ID_1`/`ID_2` per 1.000 baris; tiap potongan
   dinormalkan dan langsung ditulis ke *generasi* baru di SQLite.
4. **Tukar** — bila jumlah baris lengkap, generasi baru menggantikan isi lama
   dalam satu transaksi; bila tidak, generasi dibuang dan isi lama tetap tersaji.
5. **Analitik** — dihitung sekali per perubahan data (±1 detik untuk ±460 ribu
   aset) lalu disimpan di memori untuk seluruh halaman dan laporan.

## Pengujian

```bash
cargo test --features ssr --lib
```

105 uji: normalisasi data, penyimpanan bergenerasi, model kerusakan (termasuk
pemulihan parameter dari data simulasi), kebutuhan per skenario, konsistensi
agregat di semua tingkat, forecast, rekomendasi, grafik, laporan PDF, ekspor
beraliran, dan gerbang masuk. `cetak_ringkasan_contoh` (diabaikan secara
bawaan) mencetak ringkasan data contoh untuk pemeriksaan manual.

## Keamanan

- Seluruh halaman, aset, dan `/api/*` berada di balik gerbang masuk
  (`DASBOR_PASSWORD` atau kata sandi acak di `data/auth.json`). Permintaan API
  tanpa sesi dijawab `401`.
- Sesi memakai cookie `HttpOnly`, `SameSite=Strict`, dan `Secure` (kecuali
  pengembangan lokal); token sesi acak 256-bit disimpan di memori proses dan
  kedaluwarsa setelah 8 jam. Percobaan masuk dibatasi dan diperlambat.
- Setiap respons membawa `X-Content-Type-Options: nosniff`,
  `X-Frame-Options: DENY`, `Referrer-Policy: no-referrer`, dan
  `Cache-Control: no-store` (selain aset statis).
- Rahasia hanya tersimpan di server dan tidak pernah dikirim balik ke peramban.
- Direktori data dibatasi `0700`; berkas kredensial dan basis data `0600`.
- Ekspor CSV menetralkan sel yang dimulai `= + - @`.
- Unduhan laporan, ekspor, dan sinkronisasi dicatat ke log proses.
- **Belum ada**: akun per pengguna/peran, jejak audit tersimpan, dan
  Content-Security-Policy. Jalankan di belakang proxy ber-TLS pada jaringan
  internal. Data BMN berklasifikasi **Terbatas**.

## Berkas referensi

- `docs/AUDIT-PR1.md` — audit versi awal, yang sudah dan belum terbukti, saran lanjutan.
- `docs/panduan-web-service-sldk.pdf` — panduan resmi Web Service SLDK.
- `docs/postman-collection.json` — koleksi Postman "API Siman v2.0" (variabel rahasia dikosongkan).
