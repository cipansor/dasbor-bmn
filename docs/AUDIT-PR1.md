# Audit PR #1 dan Catatan Rombakan

Dokumen ini merangkum hasil audit atas PR #1 (*dasbor analitik BMN di atas Web
Service SLDK SIMAN v2*, commit `8f11fd5`) dan apa yang diubah pada rombakan di
cabang `ccr-ed7867bd-5it9k2`. Rujukan `berkas:baris` menunjuk ke kode PR #1 pada
commit tersebut.

## 1. Kesimpulan

PR #1 adalah fondasi yang layak: klien SLDK berhati-hati terhadap balasan
ganjil, gerbang masuk dirancang serius (pembatasan percobaan, cookie aman,
perbandingan waktu konstan), dan 49 ujinya lulus. Namun sebagai **alat bantu
pimpinan** ia belum memadai, karena tiga hal:

1. **Angka per satuan kerja tidak dapat dipercaya** begitu satuan kerja lebih
   dari 25: agregat dipotong per tabel sebelum digabung.
2. **Tidak ada pandangan ke depan**: tidak ada prediksi, analisis kebutuhan,
   maupun forecast, padahal kolom `umur_sisa` dan `rph_buku` tersedia.
   "Laporan bulanan" hanya mengganti judul; isinya sama untuk bulan apa pun.
3. **Tidak akan bertahan pada data nyata**: tabel Non-TIK (867 MB) ditampung
   utuh di memori dalam satu permintaan HTTP; tiap buka halaman data mengurai
   ulang seluruh tabel.

Rombakan mengganti lapisan data dan analitik seluruhnya, menambah sembilan
halaman baru, dan menyusun ulang laporan PDF. Bagian 4 memuat apa yang **belum**
terbukti dan perlu divalidasi dengan kredensial nyata.

## 2. Cara audit

| Langkah | Hasil |
| --- | --- |
| Membaca seluruh sumber PR (41 berkas, ±8.500 baris) | selesai |
| `cargo test --features ssr --lib` pada PR | **49 lulus** (klaim PR benar) |
| `scripts/build.sh` pada checkout bersih | **gagal** di langkah CSS (temuan E1) |
| Build manual lalu menjalankan PR di peramban | 5 halaman tampil; mode gelap rusak (D1) |
| `cargo leptos build` | **tidak diuji** (cargo-leptos tidak terpasang di lingkungan audit) |
| Uji terhadap gateway SLDK sungguhan | **tidak dapat dilakukan** (tanpa kredensial) |

## 3. Temuan

Tingkat: **Kritis** = gagal pada data nyata; **Tinggi** = hasil salah atau
fungsi utama tidak terpakai; **Sedang** = menyesatkan atau rapuh; **Rendah** =
kerapian.

### A. Analitik

| ID | Tingkat | Temuan | Bukti | Pada rombakan |
| --- | --- | --- | --- | --- |
| A1 | Tinggi | Agregat provinsi/satker dipotong per tabel (40 provinsi, 25 satker) lalu potongan itu dijumlahkan lintas tabel. Satker di luar 25 teratas suatu tabel hilang dari total; peringkat nasional salah. | `src/analytics.rs:127-133`, `:230-233` | Agregat penuh per satker × kategori; diuji konsisten di semua tingkat (`agregat_konsisten_di_semua_tingkat`). |
| A2 | Tinggi | Laporan "bulanan" tidak bergantung pada bulan: periode hanya masuk judul; jumlah halaman dikunci 5. | `src/report/mod.rs:135-147` | Laporan menyatakan posisi data secara jujur, menambah bagian perolehan pada bulan/tahun pelaporan, dan halaman dinomori dari jumlah sebenarnya. |
| A3 | Tinggi | Tidak ada prediksi, analisis kebutuhan, maupun forecast. `umur_sisa`, `rph_buku`, `no_psp`, bukti kepemilikan tidak dipakai. | seluruh `analytics.rs` | Model kerusakan, jadwal habis masa manfaat, kebutuhan tiga skenario, forecast teruji balik, skor, dan rekomendasi. |
| A4 | Sedang | KPI "Barang Rusak" (dari `brg_rusak_yn`) bertentangan dengan grafik kondisi (dari `ur_kondisi`) pada layar yang sama. | `src/analytics.rs:113-115` vs `:116` | Satu definisi (uraian kondisi); ketidakselarasan penanda dilaporkan sebagai anomali data. |
| A5 | Sedang | Grafik menyesatkan: panjang batang selalu mengikuti jumlah unit walau angka yang tertulis rupiah; teks "puncak" menampilkan nilai tahun terakhir, bukan nilai tertinggi; 15 kategori memakai 12 warna berulang; legenda donut terpotong. | `src/components/chart.rs:118-124`, `:223`, `:299-304`, `:5-8` | Batang sebanding dengan nilai yang tertulis (diuji); warna kondisi tetap; donut menggabung sisa menjadi "Lainnya". |
| A6 | Sedang | Peringkat hanya menurut jumlah unit; tidak ada pembanding antarsatker yang adil terhadap umur aset. | `src/analytics.rs:75` | Skor kesehatan dan Rasio Kerusakan terstandar Umur (RKU). |
| A7 | Sedang | Data contoh berupa derau seragam: kondisi tidak bergantung umur, rusak berat lebih banyak dari rusak ringan, `kode_satker` acak per baris. Fitur analitik tidak dapat dievaluasi. | `src/demo.rs:58-61`, `:69-73`, `:97` | Data contoh berstruktur Kejagung–Kejati–Kejari dengan kerusakan bergantung umur dan mutu pemeliharaan satker. |
| A8 | Rendah | Angka berformat Indonesia (`1.234.567,89`) terbaca nol. | `src/model.rs:186-205` | `parse_angka` mengenali kedua gaya penulisan (diuji). |

### B. Ketahanan data dan skala

| ID | Tingkat | Temuan | Bukti | Pada rombakan |
| --- | --- | --- | --- | --- |
| B1 | Kritis | Sinkronisasi menampung seluruh baris satu tabel di memori sebelum disimpan. Non-TIK 867 MB JSON berarti beberapa gigabita RAM. | `src/server_fns.rs:222-235` | Tiap potongan 1.000 baris langsung ditulis ke *generasi* baru; memori tetap kecil. |
| B2 | Kritis | Sinkronisasi (termasuk "Sinkronkan Semua") berjalan di dalam satu permintaan HTTP; penarikan berjam-jam pasti kena batas waktu, tanpa kemajuan. | `src/server_fns.rs:268-289` | Pekerjaan latar dengan kemajuan per tabel; halaman boleh ditinggalkan. |
| B3 | Tinggi | Tiap buka/urut/cari halaman data memuat dan mengurai **seluruh** baris tabel. | `src/store.rs:258-316` | Saring, urut, dan paginasi dikerjakan SQL atas kolom fakta. |
| B4 | Tinggi | Ekspor CSV membentuk seluruh berkas sebagai satu teks server function lalu URL `data:` di peramban. | `src/server_fns.rs:362-401`, `src/pages/data.rs:214-218` | Unduhan HTTP beraliran (`/api/ekspor.csv`), diuji 2.500 baris. |
| B5 | Tinggi | Analitik hanya membaca kunci huruf kecil. `getRowCount` terbukti membalas huruf besar (`RCOUNT`); bila baris aset juga demikian, seluruh analitik diam-diam nol. | `src/analytics.rs:10-22` | Kunci diseragamkan saat simpan; terbukti pada gateway tiruan berhuruf besar. |
| B6 | Sedang | Nama kolom terpenggal akibat ekstraksi PDF: `no_dok_bukti_kepe`, `jns_dok_bukti_kepe`, `stat_dok_bukti_kep`, `tgl_dok_bukti_kepe`, `properti_investasi_y`; `konjas_yn` berlabel "Konstruksi Dalam Pengerjaan"; belasan uraian rusak ("…epemilikan kepemilikan"). Kolom itu selalu kosong di tabel dan ekspor. | `src/schema.rs:75`, `:81`, dst. | Disatukan kembali sesuai lampiran panduan SLDK. |
| B7 | Sedang | Tanpa percobaan ulang; satu galat jaringan menggagalkan seluruh tabel. | `src/server_fns.rs:226-228` | Tiga percobaan berjeda per potongan, token diperbarui. |
| B8 | Sedang | Pengisian data contoh dapat berjalan bersamaan dari beberapa permintaan. | `src/server_fns.rs:27-29` | Dikunci; pemanggil kedua memakai hasil yang pertama. |

### C. Keamanan

| ID | Tingkat | Temuan | Bukti | Pada rombakan |
| --- | --- | --- | --- | --- |
| C1 | Sedang | Basis data berisi data **Terbatas** dibuat dengan izin bawaan; direktori data baru dibatasi bila berkas kata sandi ikut ditulis. | `src/store.rs:18-19` | Berkas 0600, direktori 0700 (menutup juga berkas WAL/SHM). |
| C2 | Sedang | Tidak ada kepala keamanan respons. | `src/main.rs` | `nosniff`, `X-Frame-Options: DENY`, `no-referrer`, `no-store`. |
| C3 | Rendah | README bertentangan dengan kode: menyebut `SameSite=Lax` (kode `Strict`) dan "token disimpan sebagai hash" (kode menyimpan token apa adanya di memori). | `README.md` vs `src/auth.rs:35`, `:356` | README dikoreksi. |
| C4 | Rendah | `mask()` memotong berdasarkan bita; panik bila `BA_KEY` memuat karakter multibita. | `src/server_fns.rs:122-129` | Pemotongan per karakter (diuji). |
| C5 | — | **Baik dan dipertahankan**: pembatasan percobaan masuk, perbandingan waktu konstan, cookie `HttpOnly`/`Secure`, gerbang `/api`, penetralan formula CSV, rahasia tidak pernah dikirim balik. | `src/auth.rs` | Tidak diubah. |
| C6 | Sedang | **Masih terbuka**: satu kata sandi bersama, tanpa akun per pengguna, peran, maupun jejak audit tersimpan. | `src/auth.rs` | Hanya ditambah catatan audit ke log proses (masuk laporan, ekspor, sinkronisasi). Lihat bagian 5. |
| C7 | Sedang | **Masih terbuka**: belum ada Content-Security-Policy (skrip hidrasi Leptos perlu nonce). | — | Lihat bagian 5. |

### D. Pengalaman pimpinan

| ID | Tingkat | Temuan | Bukti | Pada rombakan |
| --- | --- | --- | --- | --- |
| D1 | Tinggi | Mode gelap rusak: tombol tema hanya mengubah kartu, sedangkan utilitas `dark:` mengikuti preferensi sistem. Angka KPI tak terbaca (teks gelap di kartu gelap). Terukur: latar halaman tetap `rgb(246,247,249)` sementara kartu `rgb(27,31,40)`. | `style/tailwind.css` | `@custom-variant dark`; terukur konsisten (latar `rgb(14,17,22)`, teks terang). |
| D2 | Tinggi | Dasbor hanya memajang angka: tanpa sorotan, tanpa rekomendasi, tanpa rincian satuan kerja. Pimpinan tidak mendapat jawaban "lalu apa". | `src/pages/overview.rs` | Sorotan berkalimat, rekomendasi berprioritas, profil tiap satker. |
| D3 | Rendah | KPI memuat "Volume sumber ±1,1 GB" — tidak relevan bagi pembaca. | `src/pages/overview.rs:118` | Diganti nilai buku dan porsi tersusut. |

### E. Build dan operasi

| ID | Tingkat | Temuan | Bukti | Pada rombakan |
| --- | --- | --- | --- | --- |
| E1 | Tinggi | `scripts/build.sh` gagal pada checkout bersih tanpa CLI Tailwind mandiri: cadangan `npx @tailwindcss/cli` tidak dapat me-resolve `@import "tailwindcss"` (*Can't resolve 'tailwindcss'*). | `scripts/build.sh` | `package.json` mengunci versi; skrip menjalankan `npm ci`. |
| E2 | Tinggi | Hanya ada jalur build debug: biner **928 MB**, WASM **37 MB** diunduh tiap pengguna. | `scripts/build.sh` | `scripts/build.sh release`: biner **20 MB**, WASM **2,3 MB**. |

## 4. Yang sudah dan belum terbukti

**Terbukti pada rombakan**

- 105 uji lulus (`cargo test --features ssr --lib`), termasuk pemulihan
  parameter model dari data simulasi, konsistensi agregat, laporan PDF, dan
  ekspor beraliran.
- Build debug dan rilis lulus; 15 halaman dimuat di peramban tanpa galat konsol
  maupun galat hidrasi; tanpa gulir mendatar pada 1440 px dan 390 px.
- Interaksi diuji di peramban: cari/saring/urut satker, navigasi ke profil,
  ganti skenario, pilih kurva, saring data, mode gelap, unduh PDF dan CSV.
- **Jalur data langsung** diuji ujung-ke-ujung terhadap gateway SLDK tiruan:
  kunci huruf besar ternormalisasi; galat HTTP 500 sementara pulih lewat
  percobaan ulang; unduhan tidak lengkap (300 dari 500 baris) ditolak dan tidak
  menimpa data; tanggal bawaan `1900-01-01` tidak dianggap penghapusan.
- Kinerja: analitik atas ±460.000 aset selesai ±1 detik (profil rilis), dihitung
  sekali per perubahan data lalu disimpan.

**Belum terbukti — wajib divalidasi dengan kredensial SLDK nyata**

| Asumsi | Risiko bila keliru | Cara memeriksa |
| --- | --- | --- |
| `ID_1`/`ID_2` adalah nomor urut rapat mulai 1 | Sinkronisasi berhenti dini dan melapor "tidak lengkap" (data lama tetap aman) | Sinkronkan satu tabel kecil; bandingkan jumlah dengan `getRowCount` |
| Kosakata `ur_kondisi` (baik / rusak ringan / rusak berat) | Kondisi terbaca "tidak diisi" | Halaman Mutu Data → cakupan "Kondisi fisik" |
| Satuan `umur_sisa` adalah tahun | Jadwal habis masa manfaat bergeser | Halaman Mutu Data → "Uji satuan sisa umur"; ubah di Pengaturan |
| `tgl_hapus` terisi hanya untuk aset yang sudah dihapus | Aset aktif terkecualikan | Halaman Mutu Data → "Baris dikecualikan" |
| `rph_aset` adalah nilai bruto, `rph_buku` nilai buku | Porsi tersusut dan biaya keliru | Cocokkan satu satker dengan laporan SIMAN/SAKTI |
| `cargo leptos build` berfungsi | Jalur build alternatif gagal | Jalankan sekali; `scripts/build.sh` sudah terbukti |

**Keterbatasan metode (melekat, bukan cacat)**

- Aset rusak berat yang sudah dihapus tidak tercatat lagi, sehingga laju
  kerusakan tertaksir terlalu rendah. Pada data contoh (yang mensimulasikan
  penghapusan) lama "rusak ringan → rusak berat" tertaksir beberapa kali lebih
  panjang dari sebenarnya. **Proyeksi rusak berat adalah batas bawah.**
- Masa manfaat bawaan per kategori adalah penyederhanaan dan hanya dipakai bila
  `umur_sisa` kosong.
- Angka biaya bersifat indikatif untuk perencanaan, bukan pagu.

## 5. Saran lanjutan

1. **Validasi dengan data nyata** memakai tabel pada bagian 4 sebelum angka
   dipakai untuk keputusan.
2. **Kalibrasi model dari perubahan nyata.** Dasbor sudah menyimpan posisi
   harian; langkah berikutnya mencatat perpindahan kondisi per aset antar
   sinkronisasi mingguan agar laju kerusakan tidak lagi bergantung pada potret
   satu waktu.
3. **Akun per pengguna dan peran** (pimpinan, pengelola BMN, satker) lewat SSO
   instansi, dengan jejak audit tersimpan — data berklasifikasi Terbatas.
4. **Content-Security-Policy** dengan nonce Leptos, dan terminasi TLS di proxy.
5. **Standar barang dan standar kebutuhan (SBSK).** Analisis kebutuhan saat ini
   hanya penggantian/rehabilitasi; menautkan jumlah pegawai dan SBSK akan
   melengkapi kebutuhan baru untuk RKBMN.
6. **Sinkronisasi terjadwal mingguan** mengikuti jadwal pembaruan sumber.
