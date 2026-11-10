//! Mesin tata letak laporan: kursor menurun, ganti halaman otomatis, tabel
//! multi-halaman dengan kepala berulang, serta kepala/kaki halaman bernomor.

#![cfg(feature = "ssr")]

use super::charts::*;
use super::pdf::{bungkus, lebar_teks, Dokumen, Page, A4_H, A4_W};
use crate::dto::Rekomendasi;

pub const MARGIN: f64 = 40.0;
pub const LEBAR: f64 = A4_W - 2.0 * MARGIN;
/// Batas atas dan bawah bidang isi (di antara kepala dan kaki halaman).
const ISI_ATAS: f64 = 48.0;
const ISI_BAWAH: f64 = A4_H - 48.0;

/// Satu kolom tabel: judul, bobot lebar, dan rata kanan untuk angka.
pub struct Kol {
    pub judul: String,
    pub bobot: f64,
    pub kanan: bool,
}

/// Kolom teks rata kiri.
pub fn kiri(judul: impl Into<String>, bobot: f64) -> Kol {
    Kol {
        judul: judul.into(),
        bobot,
        kanan: false,
    }
}

/// Kolom angka rata kanan.
pub fn kanan(judul: impl Into<String>, bobot: f64) -> Kol {
    Kol {
        judul: judul.into(),
        bobot,
        kanan: true,
    }
}

/// Penulis laporan dengan kursor vertikal.
pub struct Penulis {
    selesai: Vec<Page>,
    pub hal: Page,
    /// Posisi kursor dari tepi atas halaman.
    pub y: f64,
    judul: String,
    contoh: bool,
}

impl Penulis {
    pub fn baru(judul: &str, contoh: bool) -> Self {
        Penulis {
            selesai: Vec::new(),
            hal: Page::new(A4_W, A4_H),
            y: ISI_ATAS,
            judul: judul.to_string(),
            contoh,
        }
    }

    pub fn halaman_baru(&mut self) {
        let lama = std::mem::replace(&mut self.hal, Page::new(A4_W, A4_H));
        self.selesai.push(lama);
        self.y = ISI_ATAS;
    }

    /// Pindah halaman bila sisa ruang kurang dari `tinggi`.
    pub fn pastikan(&mut self, tinggi: f64) {
        if self.y + tinggi > ISI_BAWAH && self.y > ISI_ATAS + 1.0 {
            self.halaman_baru();
        }
    }

    pub fn jarak(&mut self, t: f64) {
        self.y += t;
    }

    /// Judul bab dengan garis aksen.
    pub fn bab(&mut self, teks: &str, sub: Option<&str>) {
        self.pastikan(90.0);
        if self.y > ISI_ATAS + 1.0 {
            self.y += 10.0;
        }
        self.hal.kotak(MARGIN, self.y, 3.5, 15.0, UTAMA);
        self.hal.teks(MARGIN + 10.0, self.y + 11.5, 12.5, true, TINTA, teks);
        if let Some(s) = sub {
            let lw = lebar_teks(teks, 12.5, true);
            let ruang = LEBAR - 22.0 - lw;
            self.hal.teks(
                MARGIN + 18.0 + lw,
                self.y + 11.5,
                8.0,
                false,
                REDUP,
                &Page::potong(s, ruang, 8.0, false),
            );
        }
        self.hal
            .garis(MARGIN, self.y + 20.0, MARGIN + LEBAR, self.y + 20.0, GARIS, 0.8);
        self.y += 30.0;
    }

    /// Subjudul dalam bab.
    pub fn sub(&mut self, teks: &str) {
        self.pastikan(50.0);
        self.y += 4.0;
        self.hal.teks(MARGIN, self.y + 9.0, 9.5, true, TEKS, teks);
        self.y += 16.0;
    }

    fn teks_bungkus(&mut self, x: f64, lebar: f64, ukuran: f64, tebal: bool, warna: &str, isi: &str) {
        let tinggi = ukuran * 1.42;
        for baris in bungkus(isi, lebar, ukuran, tebal) {
            self.pastikan(tinggi);
            self.hal.teks(x, self.y + ukuran, ukuran, tebal, warna, &baris);
            self.y += tinggi;
        }
    }

    pub fn paragraf(&mut self, isi: &str) {
        self.teks_bungkus(MARGIN, LEBAR, 9.0, false, TEKS, isi);
        self.y += 5.0;
    }

    /// Catatan kecil (metodologi, keterangan tabel).
    pub fn catatan(&mut self, isi: &str) {
        self.teks_bungkus(MARGIN, LEBAR, 7.3, false, REDUP, isi);
        self.y += 4.0;
    }

    /// Butir bertanda warna dengan awalan tebal.
    pub fn butir(&mut self, warna: &str, awalan: &str, isi: &str) {
        let ukuran = 8.8;
        let tinggi = ukuran * 1.42;
        let baris = bungkus(&format!("{awalan} {isi}"), LEBAR - 14.0, ukuran, false);
        self.pastikan(tinggi * baris.len().min(3) as f64);
        for (i, b) in baris.iter().enumerate() {
            self.pastikan(tinggi);
            if i == 0 {
                self.hal.kotak(MARGIN, self.y + 2.5, 5.0, 5.0, warna);
                // Awalan tebal ditimpa di atas baris pertama agar menonjol.
                let sisa = b.strip_prefix(awalan).unwrap_or(b);
                self.hal.teks(MARGIN + 12.0, self.y + ukuran, ukuran, true, TINTA, awalan);
                self.hal.teks(
                    MARGIN + 12.0 + lebar_teks(awalan, ukuran, true),
                    self.y + ukuran,
                    ukuran,
                    false,
                    TEKS,
                    sisa,
                );
            } else {
                self.hal.teks(MARGIN + 12.0, self.y + ukuran, ukuran, false, TEKS, b);
            }
            self.y += tinggi;
        }
        self.y += 3.0;
    }

    /// Deretan kartu KPI, empat per baris.
    pub fn kpi(&mut self, kartu: &[(String, String, String, &str)]) {
        let per_baris = 4;
        let celah = 8.0;
        let w = (LEBAR - celah * (per_baris as f64 - 1.0)) / per_baris as f64;
        let h = 52.0;
        for baris in kartu.chunks(per_baris) {
            self.pastikan(h + 8.0);
            for (i, (label, nilai, hint, tone)) in baris.iter().enumerate() {
                kartu_kpi(
                    &mut self.hal,
                    MARGIN + i as f64 * (w + celah),
                    self.y,
                    w,
                    h,
                    label,
                    nilai,
                    hint,
                    tone,
                );
            }
            self.y += h + 8.0;
        }
    }

    fn kepala_tabel(&mut self, kol: &[Kol], lebar: &[f64], ukuran: f64) {
        self.hal.kotak(MARGIN, self.y, LEBAR, 16.0, "#eef1fa");
        let mut x = MARGIN;
        for (k, w) in kol.iter().zip(lebar) {
            let teks = Page::potong(&k.judul, w - 6.0, ukuran, true);
            if k.kanan {
                self.hal.teks_kanan(x + w - 4.0, self.y + 11.0, ukuran, true, "#475569", &teks);
            } else {
                self.hal.teks(x + 4.0, self.y + 11.0, ukuran, true, "#475569", &teks);
            }
            x += w;
        }
        self.y += 16.0;
    }

    /// Tabel dengan kepala berulang di tiap halaman dan baris total opsional.
    pub fn tabel(&mut self, kol: &[Kol], baris: &[Vec<String>], total: Option<&[String]>) {
        let ukuran = if kol.len() >= 10 { 6.6 } else { 7.4 };
        let tinggi = 13.5;
        let jumlah_bobot: f64 = kol.iter().map(|k| k.bobot).sum();
        let lebar: Vec<f64> = kol.iter().map(|k| LEBAR * k.bobot / jumlah_bobot).collect();
        self.pastikan(16.0 + tinggi * 3.0);
        self.kepala_tabel(kol, &lebar, ukuran);
        let tulis = |p: &mut Page, y: f64, sel: &[String], tebal: bool| {
            let mut x = MARGIN;
            for ((k, w), isi) in kol.iter().zip(&lebar).zip(sel) {
                let teks = Page::potong(isi, w - 6.0, ukuran, tebal);
                let warna = if tebal { TINTA } else { TEKS };
                if k.kanan {
                    p.teks_kanan(x + w - 4.0, y + 9.5, ukuran, tebal, warna, &teks);
                } else {
                    p.teks(x + 4.0, y + 9.5, ukuran, tebal, warna, &teks);
                }
                x += w;
            }
        };
        if baris.is_empty() {
            self.hal
                .teks(MARGIN + 4.0, self.y + 10.0, ukuran, false, REDUP, "Tidak ada data.");
            self.y += tinggi;
        }
        for (i, sel) in baris.iter().enumerate() {
            if self.y + tinggi > ISI_BAWAH {
                self.halaman_baru();
                self.kepala_tabel(kol, &lebar, ukuran);
            }
            if i % 2 == 1 {
                self.hal.kotak(MARGIN, self.y, LEBAR, tinggi, "#f8f9fd");
            }
            tulis(&mut self.hal, self.y, sel, false);
            self.y += tinggi;
        }
        if let Some(sel) = total {
            if self.y + tinggi + 2.0 > ISI_BAWAH {
                self.halaman_baru();
                self.kepala_tabel(kol, &lebar, ukuran);
            }
            self.hal
                .garis(MARGIN, self.y + 0.5, MARGIN + LEBAR, self.y + 0.5, "#94a3b8", 0.8);
            self.hal.kotak(MARGIN, self.y + 1.0, LEBAR, tinggi, "#eef1fa");
            tulis(&mut self.hal, self.y + 1.0, sel, true);
            self.y += tinggi + 1.0;
        }
        self.y += 8.0;
    }

    /// Kartu satu rekomendasi: prioritas, temuan, langkah, satuan kerja, rujukan.
    pub fn rekomendasi(&mut self, nomor: usize, r: &Rekomendasi) {
        let (label, nada) = r.label_prioritas();
        let warna = warna_tone(nada);
        let dalam = LEBAR - 16.0;
        let temuan = bungkus(&r.temuan, dalam, 8.5, false);
        let langkah: Vec<Vec<String>> = r
            .tindakan
            .iter()
            .map(|t| bungkus(t, dalam - 12.0, 8.3, false))
            .collect();
        let satker = if r.satker.is_empty() {
            vec![]
        } else {
            bungkus(
                &format!("Satuan kerja terkait: {}", r.satker.join("; ")),
                dalam,
                7.5,
                false,
            )
        };
        let rujukan = bungkus(&format!("Rujukan: {}", r.dasar), dalam, 7.3, false);
        let tinggi = 30.0
            + temuan.len() as f64 * 12.0
            + 4.0
            + langkah.iter().map(|l| l.len() as f64 * 11.6).sum::<f64>()
            + 3.0
            + satker.len() as f64 * 10.5
            + rujukan.len() as f64 * 10.2
            + 8.0;
        self.pastikan(tinggi);
        let y0 = self.y;
        self.hal.kotak(MARGIN, y0, LEBAR, tinggi - 6.0, "#f8f9fd");
        self.hal.kotak(MARGIN, y0, 3.0, tinggi - 6.0, warna);
        let x = MARGIN + 10.0;
        self.hal.teks(
            x,
            y0 + 11.0,
            6.8,
            true,
            warna,
            &format!("{} - {}", label.to_uppercase(), r.bidang.to_uppercase()),
        );
        self.hal.teks(
            x,
            y0 + 24.0,
            10.0,
            true,
            TINTA,
            &Page::potong(&format!("{nomor}. {}", r.judul), dalam, 10.0, true),
        );
        let mut y = y0 + 30.0;
        for b in &temuan {
            self.hal.teks(x, y + 8.5, 8.5, false, TEKS, b);
            y += 12.0;
        }
        y += 4.0;
        for (i, l) in langkah.iter().enumerate() {
            for (j, b) in l.iter().enumerate() {
                if j == 0 {
                    self.hal.teks(x, y + 8.3, 8.3, true, UTAMA, &format!("{}.", i + 1));
                }
                self.hal.teks(x + 12.0, y + 8.3, 8.3, false, TINTA, b);
                y += 11.6;
            }
        }
        y += 3.0;
        for b in &satker {
            self.hal.teks(x, y + 7.5, 7.5, false, TEKS, b);
            y += 10.5;
        }
        for b in &rujukan {
            self.hal.teks(x, y + 7.3, 7.3, false, REDUP, b);
            y += 10.2;
        }
        self.y = y0 + tinggi;
    }

    /// Sediakan bidang setinggi `tinggi` untuk digambar; mengembalikan ordinat atasnya.
    pub fn bidang(&mut self, tinggi: f64) -> f64 {
        self.pastikan(tinggi);
        let y = self.y;
        self.y += tinggi;
        y
    }

    /// Bubuhkan kepala/kaki bernomor lalu susun berkas PDF.
    pub fn selesai(mut self, tanggal_pdf: &str) -> Vec<u8> {
        self.selesai.push(self.hal);
        let total = self.selesai.len();
        let mut d = Dokumen::new(&self.judul);
        for (i, mut p) in self.selesai.into_iter().enumerate() {
            let nomor = i + 1;
            if nomor > 1 {
                p.kotak(0.0, 0.0, A4_W, 26.0, TINTA);
                p.kotak(MARGIN, 6.0, 3.5, 14.0, UTAMA);
                p.teks(MARGIN + 10.0, 16.5, 8.0, true, "#ffffff", &self.judul);
                p.teks_kanan(A4_W - MARGIN, 16.5, 7.5, false, "#93a2c4", "Kejaksaan Republik Indonesia");
            }
            p.garis(MARGIN, A4_H - 34.0, A4_W - MARGIN, A4_H - 34.0, GARIS, 0.6);
            let kaki = if self.contoh {
                "DATA CONTOH - BUKAN DATA RIIL. Seluruh angka dan satuan kerja bersifat ilustratif."
            } else {
                "Sumber: Web Service SLDK SIMAN v2 (DJKN, Kemenkeu). Klasifikasi: Terbatas."
            };
            p.teks(
                MARGIN,
                A4_H - 21.0,
                7.3,
                self.contoh,
                if self.contoh { "#b45309" } else { REDUP },
                kaki,
            );
            p.teks_kanan(
                A4_W - MARGIN,
                A4_H - 21.0,
                7.3,
                false,
                REDUP,
                &format!("Halaman {nomor} dari {total}"),
            );
            d.tambah(p);
        }
        d.bangun(tanggal_pdf)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tabel_panjang_berpindah_halaman_dengan_kepala_berulang() {
        let mut w = Penulis::baru("Uji Tabel", false);
        w.bab("Lampiran", Some("uji"));
        let baris: Vec<Vec<String>> = (0..150)
            .map(|i| vec![format!("Satuan kerja {i}"), i.to_string()])
            .collect();
        w.tabel(
            &[kiri("NAMA-KOLOM-UJI", 3.0), kanan("Unit", 1.0)],
            &baris,
            Some(&["Total".to_string(), "150".to_string()]),
        );
        let b = w.selesai("D:20260101120000Z");
        let teks = String::from_utf8_lossy(&b);
        assert!(teks.contains("/Count 3"), "150 baris memerlukan tiga halaman");
        assert_eq!(teks.matches("NAMA-KOLOM-UJI").count(), 3, "kepala berulang");
        assert!(teks.contains("Halaman 3 dari 3"));
        assert!(teks.contains("Satuan kerja 149"));
    }

    #[test]
    fn data_contoh_ditandai_di_setiap_halaman() {
        let mut w = Penulis::baru("Uji", true);
        w.paragraf("Halaman satu.");
        w.halaman_baru();
        w.paragraf("Halaman dua.");
        let b = w.selesai("D:20260101120000Z");
        let teks = String::from_utf8_lossy(&b);
        assert_eq!(teks.matches("DATA CONTOH - BUKAN DATA RIIL").count(), 2);
    }

    #[test]
    fn rekomendasi_panjang_tidak_terpotong_halaman() {
        let mut w = Penulis::baru("Uji", false);
        let r = Rekomendasi {
            prioritas: 1,
            bidang: "Penghapusan".into(),
            judul: "Judul".into(),
            temuan: "temuan ".repeat(60),
            tindakan: vec!["langkah ".repeat(40), "langkah kedua".into()],
            dasar: "PMK".into(),
            satker: vec!["Kejari A".into()],
        };
        for i in 0..8 {
            w.rekomendasi(i + 1, &r);
        }
        let b = w.selesai("D:20260101120000Z");
        let teks = String::from_utf8_lossy(&b);
        assert!(teks.contains("/Count 2") || teks.contains("/Count 3"));
        assert_eq!(teks.matches("PRIORITAS TINGGI - PENGHAPUSAN").count(), 8);
    }
}
