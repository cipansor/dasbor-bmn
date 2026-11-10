//! Penyusun laporan PDF.
//!
//! Dua laporan tersedia: **nasional** (ringkasan eksekutif, rekomendasi,
//! kondisi, prediksi, kebutuhan, forecast, wilayah, dan lampiran seluruh satuan
//! kerja) dan **satuan kerja** (profil satu satuan kerja beserta daftar aset
//! prioritasnya). Laporan menyajikan posisi data pada sinkronisasi terakhir;
//! bulan pelaporan menentukan judul dan bagian perolehan periode.

#![cfg(feature = "ssr")]

pub mod charts;
pub mod http;
pub mod pdf;
pub mod tata;

use crate::dto::*;
use crate::model::{
    format_desimal, format_number, format_persen, format_rupiah_kata, format_rupiah_short,
    format_unit,
};
use charts::*;
use tata::{kanan, kiri, Kol, Penulis, LEBAR, MARGIN};

/// Bulan pelaporan yang dipilih pengguna.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Periode {
    pub tahun: i32,
    pub bulan: u32,
}

const NAMA_BULAN: [&str; 12] = [
    "Januari",
    "Februari",
    "Maret",
    "April",
    "Mei",
    "Juni",
    "Juli",
    "Agustus",
    "September",
    "Oktober",
    "November",
    "Desember",
];

impl Periode {
    /// Bulan berjalan menurut zona waktu Indonesia Barat (UTC+7).
    pub fn sekarang() -> Self {
        let (tahun, bulan) = crate::tanggal_kini();
        Periode { tahun, bulan }
    }

    /// Baca periode dari bentuk `YYYY-MM`.
    pub fn dari_str(s: &str) -> Option<Self> {
        let (y, m) = s.trim().split_once('-')?;
        let tahun: i32 = y.parse().ok()?;
        let bulan: u32 = m.parse().ok()?;
        if !(1900..=2100).contains(&tahun) || !(1..=12).contains(&bulan) {
            return None;
        }
        Some(Periode { tahun, bulan })
    }

    pub fn kode(&self) -> String {
        format!("{:04}-{:02}", self.tahun, self.bulan)
    }

    pub fn nama_bulan(&self) -> &'static str {
        NAMA_BULAN[(self.bulan.saturating_sub(1) as usize).min(11)]
    }

    pub fn label(&self) -> String {
        format!("{} {}", self.nama_bulan(), self.tahun)
    }
}

/// Ubah "YYYY-MM-DD HH:MM …" menjadi tanggal PDF "D:YYYYMMDDHHmm00".
fn tanggal_pdf(s: &str) -> String {
    let digit: String = s.chars().filter(|c| c.is_ascii_digit()).collect();
    if digit.len() >= 12 {
        format!("D:{}00+07'00'", &digit[..12])
    } else {
        "D:20260101000000Z".to_string()
    }
}

fn unit(v: i64) -> String {
    format_number(v)
}

fn unit_f(v: f64) -> String {
    format_unit(v)
}

fn rp(v: f64) -> String {
    format_rupiah_short(v)
}

fn pct(v: f64, ada: bool) -> String {
    if ada {
        format_persen(v)
    } else {
        "-".into()
    }
}

fn fmt_unit(v: f64) -> String {
    format_unit(v)
}

fn fmt_rp(v: f64) -> String {
    format_rupiah_short(v).replace("Rp ", "")
}

fn warna_nada(nada: &str) -> &'static str {
    match nada {
        "positif" => C_BAIK,
        "waspada" => C_RR,
        "kritis" => C_RB,
        _ => UTAMA,
    }
}

/// Halaman sampul: pita judul, identitas laporan, dan penanda data contoh.
fn sampul(w: &mut Penulis, judul_besar: &str, sub: &str, m: &Meta, periode: &Periode) {
    use pdf::A4_W;
    let p = &mut w.hal;
    p.kotak(0.0, 0.0, A4_W, 172.0, TINTA);
    p.kotak(0.0, 0.0, A4_W, 6.0, UTAMA);
    p.kotak(MARGIN, 36.0, 44.0, 44.0, UTAMA);
    p.teks_tengah(MARGIN + 22.0, 64.0, 14.0, true, "#ffffff", "BMN");
    p.teks(MARGIN + 56.0, 53.0, 13.0, true, "#ffffff", "Dasbor Analitik Barang Milik Negara");
    p.teks(MARGIN + 56.0, 69.0, 8.5, false, "#93a2c4", "Kejaksaan Republik Indonesia");
    p.teks(
        MARGIN,
        112.0,
        20.0,
        true,
        "#ffffff",
        &pdf::Page::potong(judul_besar, LEBAR, 20.0, true),
    );
    p.teks(
        MARGIN,
        132.0,
        11.0,
        false,
        "#c7d2ea",
        &pdf::Page::potong(sub, LEBAR, 11.0, false),
    );
    p.teks(
        MARGIN,
        152.0,
        8.3,
        false,
        "#93a2c4",
        &format!(
            "Bulan pelaporan {}  |  posisi data {}  |  dicetak {}",
            periode.label(),
            if m.tanggal_data.is_empty() { "-" } else { &m.tanggal_data },
            m.dibuat
        ),
    );
    w.y = 186.0;
    if m.is_demo {
        w.hal.kotak(MARGIN, w.y, LEBAR, 24.0, "#fef3c7");
        w.hal.kotak(MARGIN, w.y, 3.0, 24.0, "#b45309");
        w.hal.teks(
            MARGIN + 10.0,
            w.y + 15.0,
            8.5,
            true,
            "#92400e",
            "DATA CONTOH - BUKAN DATA RIIL. Seluruh angka, kondisi, dan kode satuan kerja bersifat ilustratif.",
        );
        w.y += 34.0;
    }
}

/// Tabel berkolom tahun proyeksi: nama | sudah/tunggakan | tiap tahun | total [| biaya].
fn tabel_tahun(
    w: &mut Penulis,
    m: &Meta,
    kepala_awal: &str,
    kepala_kini: &str,
    baris: Vec<(String, f64, [f64; NH], Option<f64>)>,
    total: Option<(String, f64, [f64; NH], Option<f64>)>,
) {
    let ada_biaya = baris.iter().any(|b| b.3.is_some());
    let mut kol: Vec<Kol> = vec![kiri(kepala_awal, 3.4), kanan(kepala_kini, 1.25)];
    for k in 0..NH {
        kol.push(kanan(m.label_tahun(k), 1.0));
    }
    kol.push(kanan("Total", 1.2));
    if ada_biaya {
        kol.push(kanan("Perkiraan biaya", 1.7));
    }
    let susun = |b: &(String, f64, [f64; NH], Option<f64>)| -> Vec<String> {
        let mut v = vec![b.0.clone(), unit_f(b.1)];
        v.extend(b.2.iter().map(|x| unit_f(*x)));
        v.push(unit_f(b.1 + b.2.iter().sum::<f64>()));
        if ada_biaya {
            v.push(b.3.map(rp).unwrap_or_else(|| "-".into()));
        }
        v
    };
    let isi: Vec<Vec<String>> = baris.iter().map(susun).collect();
    let tot = total.as_ref().map(susun);
    w.tabel(&kol, &isi, tot.as_deref());
}

fn bagian_kpi_utama(w: &mut Penulis, r: &Ringkas, skor: &Skor, m: &Meta) {
    let (predikat, nada) = Skor::predikat(skor.total);
    let akhir = m.tahun_akhir();
    w.kpi(&[
        (
            "Total BMN".into(),
            format!("{} unit", unit(r.unit)),
            format!("nilai buku {}", rp(r.buku)),
            "blue",
        ),
        (
            "Nilai aset".into(),
            rp(r.nilai),
            format!("tersusut {}", format_persen(r.pct_susut())),
            "emerald",
        ),
        (
            "Kondisi baik".into(),
            pct(r.pct_baik(), r.kondisi_diketahui() > 0),
            format!("rusak ringan {}", unit(r.rr)),
            if r.pct_baik() >= 85.0 { "emerald" } else { "amber" },
        ),
        (
            "Skor kesehatan".into(),
            format_desimal(skor.total, 1),
            format!("predikat {predikat}"),
            nada,
        ),
        (
            "Rusak berat".into(),
            format!("{} unit", unit(r.rb)),
            format!("senilai {}", rp(r.nilai_rb)),
            "rose",
        ),
        (
            "Habis masa manfaat".into(),
            format!("{} unit", unit(r.habis)),
            format!("{} dari yang disusutkan", pct(r.pct_habis(), r.susut_unit > 0)),
            "violet",
        ),
        (
            "Aset idle".into(),
            format!("{} unit", unit(r.idle)),
            format!("senilai {}", rp(r.nilai_idle)),
            "amber",
        ),
        (
            "Barang hilang".into(),
            format!("{} unit", unit(r.hilang)),
            format!("senilai {}", rp(r.nilai_hilang)),
            "rose",
        ),
        (
            format!("RB baru s.d. {akhir}"),
            format!("{} unit", unit_f(r.rb_5())),
            "ekspektasi model kerusakan".into(),
            "rose",
        ),
        (
            format!("Habis MM s.d. {akhir}"),
            format!("+{} unit", unit_f(r.habis_5())),
            "tambahan dari posisi kini".into(),
            "violet",
        ),
        (
            "Kebutuhan moderat".into(),
            format!("{} unit", unit_f(r.ganti_total(1))),
            "termasuk tunggakan".into(),
            "blue",
        ),
        (
            "Perkiraan biaya".into(),
            rp(r.biaya_total(1)),
            format!("{} - {}", rp(r.biaya_total(0)), rp(r.biaya_total(2))),
            "emerald",
        ),
    ]);
}

fn grafik_proyeksi(w: &mut Penulis, r: &Ringkas, m: &Meta) {
    let label: Vec<String> = (0..NH).map(|k| m.label_tahun(k)).collect();
    let y = w.bidang(150.0);
    let setengah = (LEBAR - 14.0) / 2.0;
    w.hal.teks(MARGIN, y + 8.0, 8.3, true, TEKS, "Habis masa manfaat per tahun (unit)");
    kolom(
        &mut w.hal,
        MARGIN,
        y + 12.0,
        setengah,
        132.0,
        &label,
        &[SeriPdf {
            nama: "Habis masa manfaat",
            warna: C_UNGU,
            nilai: r.habis_th.to_vec(),
        }],
        &fmt_unit,
        None,
    );
    let x2 = MARGIN + setengah + 14.0;
    w.hal.teks(x2, y + 8.0, 8.3, true, TEKS, "Perkiraan rusak berat baru per tahun (unit)");
    kolom(
        &mut w.hal,
        x2,
        y + 12.0,
        setengah,
        132.0,
        &label,
        &[SeriPdf {
            nama: "Rusak berat baru",
            warna: C_RB,
            nilai: r.rb_th.to_vec(),
        }],
        &fmt_unit,
        None,
    );
    w.catatan("Tanda * = sisa tahun berjalan sejak tanggal data. Habis masa manfaat mengikuti sisa umur pada SIMAN; rusak berat adalah ekspektasi model kerusakan (batas bawah).");
}

fn tabel_skenario(w: &mut Penulis, r: &Ringkas, m: &Meta) {
    let baris: Vec<(String, f64, [f64; NH], Option<f64>)> = (0..3)
        .map(|s| {
            (
                SKENARIO[s].0.to_string(),
                r.ganti_kini[s],
                r.ganti_th[s],
                Some(r.biaya_total(s)),
            )
        })
        .collect();
    tabel_tahun(w, m, "Skenario", "Tunggakan", baris, None);
    for (nama, uraian) in SKENARIO {
        w.catatan(&format!("{nama}: {uraian}"));
    }
}

fn daftar_aset(w: &mut Penulis, aset: &[AsetRisiko], dengan_satker: bool, dengan_peluang: bool) {
    let mut kol = vec![kiri("Aset", 3.2), kiri("Kategori", 1.8)];
    if dengan_satker {
        kol.push(kiri("Satuan kerja", 2.6));
    }
    kol.extend([
        kanan("Tahun", 0.8),
        kanan("Umur", 0.8),
        kiri("Kondisi", 1.3),
        kanan("Sisa umur", 0.9),
        kanan("Nilai", 1.3),
    ]);
    if dengan_peluang {
        kol.push(kanan("Peluang RB", 1.1));
    }
    let baris: Vec<Vec<String>> = aset
        .iter()
        .map(|a| {
            let mut v = vec![
                if a.nama.is_empty() { a.kelompok.clone() } else { a.nama.clone() },
                a.kategori.clone(),
            ];
            if dengan_satker {
                v.push(a.satker.clone());
            }
            v.push(if a.tahun > 0 { a.tahun.to_string() } else { "-".into() });
            v.push(if a.umur >= 0.0 { format_desimal(a.umur, 1) } else { "-".into() });
            v.push(a.kondisi.clone());
            v.push(if a.sisa >= 0.0 { format_desimal(a.sisa, 1) } else { "-".into() });
            v.push(rp(a.nilai));
            if dengan_peluang {
                v.push(format_persen(a.p_rb * 100.0));
            }
            v
        })
        .collect();
    w.tabel(&kol, &baris, None);
}

fn bagian_metodologi(w: &mut Penulis, m: &Meta) {
    let asumsi = m.asumsi.clone().unwrap_or_default();
    w.bab("Metodologi, Asumsi, dan Keterbatasan", None);
    w.butir(UTAMA, "Sumber.", "Lima belas tabel Master Aset BMN dari Web Service SLDK SIMAN v2; posisi pada sinkronisasi terakhir. Baris bertanggal hapus sah dikecualikan.");
    w.butir(UTAMA, "Habis masa manfaat.", &format!("Mengikuti kolom sisa umur ekonomis SIMAN (satuan {}). Bila kosong atau janggal, ditaksir dari porsi nilai buku (garis lurus). Habis masa manfaat berarti nilai buku habis, bukan berarti rusak.", asumsi.satuan_umur_sisa));
    w.butir(UTAMA, "Prediksi rusak berat.", "Model dua tahap baik - rusak ringan - rusak berat. Kurva peluang masih baik menurut umur (Weibull) dan laju rusak ringan ke rusak berat ditaksir per kategori dari potret umur-kondisi. Tiap aset diberi peluang menurut kategori, umur, kondisi kini, dan rekam kerusakan satuan kerjanya; dijumlahkan menjadi ekspektasi.");
    w.butir(UTAMA, "Kebutuhan.", &format!("Tiga skenario (dasar, moderat, maksimal). Harga satuan peralatan = median perolehan lima tahun terakhir pada kelompok barang yang sama, atau nilai perolehan dieskalasi {} per tahun. Rehabilitasi bangunan = {} dari nilai aset. Biaya tahun mendatang dieskalasi {} per tahun.", format_persen(asumsi.eskalasi * 100.0), format_persen(asumsi.faktor_rehab * 100.0), format_persen(asumsi.eskalasi * 100.0)));
    w.butir(UTAMA, "Forecast.", "Rata-rata tiga tahun, tren linear, dan pemulusan eksponensial Holt teredam diadu dengan uji balik; metode bergalat terkecil dipakai dan galatnya dilaporkan.");
    w.butir(UTAMA, "Skor kesehatan.", "Gabungan kondisi fisik 35%, masa manfaat 20%, pemanfaatan 15%, pengamanan 15%, tertib administrasi 15%. Indikator internal dasbor, bukan indikator resmi.");
    w.butir(C_RR, "Keterbatasan.", "Aset rusak berat yang sudah dihapus tidak tercatat lagi sehingga proyeksi kerusakan cenderung batas bawah. Proyeksi tidak memperhitungkan pemeliharaan besar, pengadaan pengganti, maupun kebutuhan baru karena perubahan organisasi. Angka biaya bersifat indikatif untuk perencanaan, bukan pagu.");
    w.butir(C_RR, "Rujukan ketentuan.", "PP 27/2014 jo. PP 28/2020; PMK 181/PMK.06/2016; PMK 65/PMK.06/2017; KMK 295/KM.6/2019 beserta perubahannya; PMK 153/PMK.06/2021; PMK 83/PMK.06/2016; PMK 71/PMK.06/2016; PMK 246/PMK.06/2014 beserta perubahannya; PMK 207/PMK.06/2021. Pastikan ketentuan yang berlaku saat tindak lanjut.");
}

/// Susun laporan nasional sebagai byte PDF.
pub fn bangun_nasional(
    a: &Analisis,
    satker: &[SatkerBaris],
    periode: &Periode,
    aktivitas: &AktivitasPeriode,
) -> Vec<u8> {
    let m = &a.meta;
    let r = &a.nasional;
    let akhir = m.tahun_akhir();
    let judul = format!("Laporan Analitik BMN - {}", periode.label());
    let mut w = Penulis::baru(&judul, m.is_demo);
    sampul(
        &mut w,
        "Laporan Analitik Barang Milik Negara",
        &format!("Kejaksaan Republik Indonesia  |  {}", periode.label()),
        m,
        periode,
    );

    // 1. Ringkasan eksekutif.
    w.bab("Ringkasan Eksekutif", None);
    let tinggi = a.rekomendasi.iter().filter(|x| x.prioritas == 1).count();
    w.paragraf(&format!(
        "Kejaksaan RI mengelola {} unit Barang Milik Negara senilai {} (nilai buku {}) pada {} satuan kerja di {} provinsi. \
         Sebanyak {} aset fisik berkondisi baik; {} unit rusak berat senilai {} masih tercatat, dan {} aset yang disusutkan \
         sudah habis masa manfaat. Sampai akhir {akhir}, sekitar {} unit lagi diperkirakan menjadi rusak berat dan {} unit lagi habis \
         masa manfaat. Kebutuhan penggantian dan rehabilitasi skenario moderat diperkirakan {} (rentang {} sampai {}). \
         Laporan ini memuat {} rekomendasi, {} di antaranya berprioritas tinggi.",
        unit(r.unit),
        format_rupiah_kata(r.nilai),
        format_rupiah_kata(r.buku),
        unit(m.total_satker),
        m.total_provinsi,
        pct(r.pct_baik(), r.kondisi_diketahui() > 0),
        unit(r.rb),
        format_rupiah_kata(r.nilai_rb),
        pct(r.pct_habis(), r.susut_unit > 0),
        unit_f(r.rb_5()),
        unit_f(r.habis_5()),
        format_rupiah_kata(r.biaya_total(1)),
        format_rupiah_kata(r.biaya_total(0)),
        format_rupiah_kata(r.biaya_total(2)),
        a.rekomendasi.len(),
        tinggi,
    ));
    bagian_kpi_utama(&mut w, r, &a.skor, m);
    if let Some(d) = &a.delta {
        let tanda = |v: i64| if v > 0 { format!("+{}", unit(v)) } else { unit(v) };
        w.catatan(&format!(
            "Perubahan sejak posisi {}: {} unit; rusak berat {}; habis masa manfaat {}; idle {}; hilang {}.",
            d.tanggal,
            tanda(d.unit),
            tanda(d.rb),
            tanda(d.habis),
            tanda(d.idle),
            tanda(d.hilang)
        ));
    }

    // 2. Sorotan dan rekomendasi.
    w.halaman_baru();
    w.bab("Sorotan Utama", Some("temuan terpenting dari data"));
    for s in &a.sorotan {
        w.butir(warna_nada(&s.nada), &format!("{}.", s.judul), &s.isi);
    }
    w.bab("Rekomendasi Tindak Lanjut", Some("urut prioritas"));
    for (i, x) in a.rekomendasi.iter().enumerate() {
        w.rekomendasi(i + 1, x);
    }

    // 3. Kondisi dan masa manfaat.
    w.halaman_baru();
    w.bab("Kondisi dan Masa Manfaat", Some("per kategori"));
    let baris: Vec<Vec<String>> = a
        .kategori
        .iter()
        .map(|c| {
            let k = &c.r;
            let f = |ada: bool, s: String| if ada { s } else { "-".into() };
            vec![
                c.label.clone(),
                unit(k.unit),
                rp(k.nilai),
                pct(k.pct_baik(), c.fisik && k.kondisi_diketahui() > 0),
                f(c.fisik, unit(k.rr)),
                f(c.fisik, unit(k.rb)),
                f(c.fisik, rp(k.nilai_rb)),
                f(c.disusutkan, unit(k.habis)),
                pct(k.pct_habis(), c.disusutkan && k.susut_unit > 0),
                f(k.umur_n > 0, format_desimal(k.umur_rata(), 1)),
                unit(k.idle),
                unit(k.hilang),
            ]
        })
        .collect();
    let total = vec![
        "Seluruh kategori".to_string(),
        unit(r.unit),
        rp(r.nilai),
        pct(r.pct_baik(), true),
        unit(r.rr),
        unit(r.rb),
        rp(r.nilai_rb),
        unit(r.habis),
        pct(r.pct_habis(), true),
        format_desimal(r.umur_rata(), 1),
        unit(r.idle),
        unit(r.hilang),
    ];
    w.tabel(
        &[
            kiri("Kategori", 3.3),
            kanan("Unit", 1.1),
            kanan("Nilai", 1.4),
            kanan("Baik", 1.0),
            kanan("R. ringan", 1.1),
            kanan("R. berat", 1.0),
            kanan("Nilai RB", 1.3),
            kanan("Habis MM", 1.1),
            kanan("% habis", 1.0),
            kanan("Umur (th)", 1.1),
            kanan("Idle", 0.8),
            kanan("Hilang", 0.9),
        ],
        &baris,
        Some(&total),
    );
    w.catatan("MM = masa manfaat. Tanda - berarti ukuran tidak berlaku bagi kategori tersebut (kondisi fisik tidak relevan atau tidak disusutkan).");

    w.sub("Profil umur dan kondisi aset fisik (unit)");
    let y = w.bidang(150.0);
    let label: Vec<String> = a.umur.iter().map(|u| u.label.clone()).collect();
    kolom(
        &mut w.hal,
        MARGIN,
        y,
        LEBAR,
        146.0,
        &label,
        &[
            SeriPdf { nama: "Baik", warna: C_BAIK, nilai: a.umur.iter().map(|u| u.baik as f64).collect() },
            SeriPdf { nama: "Rusak ringan", warna: C_RR, nilai: a.umur.iter().map(|u| u.rr as f64).collect() },
            SeriPdf { nama: "Rusak berat", warna: C_RB, nilai: a.umur.iter().map(|u| u.rb as f64).collect() },
            SeriPdf { nama: "Tidak diisi", warna: C_TAK, nilai: a.umur.iter().map(|u| u.tak as f64).collect() },
        ],
        &fmt_unit,
        None,
    );

    // 4. Prediksi lima tahun.
    w.halaman_baru();
    w.bab("Prediksi Lima Tahun", Some(&format!("sampai akhir {akhir}")));
    grafik_proyeksi(&mut w, r, m);
    w.sub("Jadwal habis masa manfaat per kategori (unit)");
    tabel_tahun(
        &mut w,
        m,
        "Kategori",
        "Sudah",
        a.kategori
            .iter()
            .filter(|c| c.disusutkan)
            .map(|c| (c.label.clone(), c.r.habis as f64, c.r.habis_th, None))
            .collect(),
        Some(("Seluruh kategori".into(), r.habis as f64, r.habis_th, None)),
    );
    w.sub("Perkiraan rusak berat baru per kategori (unit)");
    tabel_tahun(
        &mut w,
        m,
        "Kategori",
        "RB kini",
        a.kategori
            .iter()
            .filter(|c| c.fisik)
            .map(|c| (c.label.clone(), c.r.rb as f64, c.r.rb_th, None))
            .collect(),
        Some(("Seluruh kategori".into(), r.rb as f64, r.rb_th, None)),
    );
    w.catatan("Kolom Total pada kedua tabel = posisi kini ditambah proyeksi sampai akhir cakrawala.");
    w.sub("Parameter model kerusakan");
    w.tabel(
        &[
            kiri("Kategori", 3.0),
            kanan("Sampel", 1.0),
            kanan("Median mulai tidak baik (th)", 2.2),
            kanan("Rusak ringan ke berat (th)", 2.1),
            kanan("Selisih model-data (poin %)", 2.2),
            kiri("Sumber", 1.6),
        ],
        &a.model
            .iter()
            .map(|x| {
                vec![
                    x.label.clone(),
                    unit(x.n),
                    format_desimal(x.median_mulai, 1),
                    format_desimal(x.lama_rr, 1),
                    format_desimal(x.mae, 1),
                    if x.gabungan { "kurva gabungan".into() } else { "data kategori".into() },
                ]
            })
            .collect::<Vec<_>>(),
        None,
    );
    w.sub("Aset paling berisiko menjadi rusak berat");
    daftar_aset(&mut w, &a.aset_risiko, true, true);

    // 5. Analisis kebutuhan.
    w.halaman_baru();
    w.bab("Analisis Kebutuhan Lima Tahun", Some("penggantian dan rehabilitasi aset yang ada"));
    let per_tahun = r.biaya_total(1) / HORIZON as f64;
    w.paragraf(&format!(
        "Skenario moderat memerlukan {} unit dengan perkiraan biaya {} sampai {akhir}, atau rata-rata {} per tahun. \
         Sebagai pembanding, perolehan kategori sejenis {}-{} rata-rata {} per tahun ({} unit per tahun).",
        unit_f(r.ganti_total(1)),
        format_rupiah_kata(r.biaya_total(1)),
        format_rupiah_kata(per_tahun),
        a.kapasitas.tahun_awal,
        a.kapasitas.tahun_akhir,
        format_rupiah_kata(a.kapasitas.nilai_per_tahun),
        unit_f(a.kapasitas.unit_per_tahun),
    ));
    tabel_skenario(&mut w, r, m);
    w.sub("Perkiraan biaya per tahun - skenario moderat (rupiah)");
    let y = w.bidang(150.0);
    let label: Vec<String> = std::iter::once("Tunggakan".to_string())
        .chain((0..NH).map(|k| m.label_tahun(k)))
        .collect();
    let biaya: Vec<f64> = std::iter::once(r.biaya_kini[1])
        .chain(r.biaya_th[1].iter().copied())
        .collect();
    let teks_acuan = format!(
        "rata-rata perolehan {}-{}",
        a.kapasitas.tahun_awal, a.kapasitas.tahun_akhir
    );
    kolom(
        &mut w.hal,
        MARGIN,
        y,
        LEBAR,
        146.0,
        &label,
        &[SeriPdf { nama: "Biaya", warna: C_BIRU2, nilai: biaya }],
        &fmt_rp,
        (a.kapasitas.nilai_per_tahun > 0.0).then_some((a.kapasitas.nilai_per_tahun, teks_acuan.as_str())),
    );
    w.sub("Kebutuhan per kategori - skenario moderat (unit)");
    tabel_tahun(
        &mut w,
        m,
        "Kategori",
        "Tunggakan",
        a.kategori
            .iter()
            .filter(|c| c.kebutuhan != "-")
            .map(|c| {
                (
                    format!("{} ({})", c.label, c.kebutuhan.to_lowercase()),
                    c.r.ganti_kini[1],
                    c.r.ganti_th[1],
                    Some(c.r.biaya_total(1)),
                )
            })
            .collect(),
        Some((
            "Seluruh kategori".into(),
            r.ganti_kini[1],
            r.ganti_th[1],
            Some(r.biaya_total(1)),
        )),
    );
    w.sub("Kelompok barang dengan kebutuhan terbesar - skenario moderat");
    w.tabel(
        &[
            kiri("Kelompok barang", 3.4),
            kiri("Kategori", 2.0),
            kanan("Unit ada", 1.0),
            kanan("Kebutuhan", 1.1),
            kanan("Harga acuan", 1.3),
            kanan("Perkiraan biaya", 1.5),
            kanan("Idle baik", 1.0),
        ],
        &a.kelompok_butuh
            .iter()
            .filter(|k| k.unit[1] >= 0.5)
            .take(18)
            .map(|k| {
                vec![
                    k.kelompok.clone(),
                    k.kategori.clone(),
                    unit(k.unit_total),
                    unit_f(k.unit[1]),
                    rp(k.harga),
                    rp(k.biaya[1]),
                    if k.idle_baik > 0 { unit(k.idle_baik) } else { "-".into() },
                ]
            })
            .collect::<Vec<_>>(),
        None,
    );
    w.catatan(&format!(
        "Idle baik = unit idle berkondisi baik pada kelompok yang sama, dapat dialihkan sebelum pengadaan. Pemeliharaan: {} unit layak dipelihara, {} unit rusak ringan perlu perbaikan.",
        unit(r.layak_pelihara),
        unit(r.rr)
    ));

    // 6. Forecast perolehan.
    w.halaman_baru();
    w.bab("Forecast Perolehan", Some("bila pola historis berlanjut"));
    if let Some(total) = a.prakiraan.first() {
        w.sub("Jumlah unit perolehan per tahun - seluruh kategori");
        let y = w.bidang(140.0);
        garis_ramal(&mut w.hal, MARGIN, y, LEBAR, 136.0, &total.unit.riwayat, &total.unit.ramal, &fmt_unit);
        w.catatan(&format!(
            "Garis utuh = realisasi tahun penuh; garis putus = prakiraan; pita = rentang 80%. Metode: {}; galat uji balik {}.",
            total.unit.metode,
            if total.unit.galat < 0.0 { "-".into() } else { format_persen(total.unit.galat) }
        ));
        let mut kol = vec![kiri("Kategori", 3.2)];
        // Kolom tahun prakiraan mengikuti titik deret total.
        let tahun: Vec<i32> = total.unit.ramal.iter().map(|t| t.tahun).collect();
        for t in &tahun {
            kol.push(kanan(
                if *t == m.tahun0 {
                    format!("{t}*")
                } else {
                    t.to_string()
                },
                1.0,
            ));
        }
        kol.push(kanan("Galat", 0.9));
        kol.push(kiri("Metode", 2.6));
        let baris: Vec<Vec<String>> = a
            .prakiraan
            .iter()
            .map(|p| {
                let mut v = vec![p.label.clone()];
                v.extend(p.unit.ramal.iter().map(|t| unit_f(t.nilai)));
                v.push(if p.unit.galat < 0.0 { "-".into() } else { format_persen(p.unit.galat) });
                v.push(p.unit.metode.clone());
                v
            })
            .collect();
        w.sub("Prakiraan jumlah unit per kategori");
        w.tabel(&kol, &baris, None);
        w.catatan("Tanda * = taksiran setahun penuh tahun berjalan. Riwayat hanya memuat aset yang masih tercatat; galat di atas 30% berarti pola tidak stabil dan prakiraan dibaca sebagai arah.");
    }
    if !aktivitas.per_kategori.is_empty() || aktivitas.unit_tahun > 0 {
        w.sub(&format!("Perolehan pada {}", aktivitas.label));
        w.paragraf(&format!(
            "Tercatat {} unit senilai {} dengan tanggal perolehan pada bulan pelaporan, dan {} unit senilai {} sepanjang tahun {}.",
            unit(aktivitas.unit_bulan),
            format_rupiah_kata(aktivitas.nilai_bulan),
            unit(aktivitas.unit_tahun),
            format_rupiah_kata(aktivitas.nilai_tahun),
            periode.tahun
        ));
        if !aktivitas.per_kategori.is_empty() {
            w.tabel(
                &[kiri("Kategori", 4.0), kanan("Unit", 1.2), kanan("Nilai", 1.6)],
                &aktivitas
                    .per_kategori
                    .iter()
                    .map(|(k, n, v)| vec![k.clone(), unit(*n), rp(*v)])
                    .collect::<Vec<_>>(),
                None,
            );
        }
    }

    // 7. Wilayah.
    w.halaman_baru();
    w.bab("Sebaran per Provinsi", Some("menurut lokasi aset, urut nilai"));
    w.tabel(
        &[
            kiri("Provinsi", 3.0),
            kanan("Satker", 0.8),
            kanan("Unit", 1.0),
            kanan("Nilai", 1.3),
            kanan("Baik", 0.9),
            kanan("RB", 0.8),
            kanan("Habis MM", 1.0),
            kanan("RB baru 5 th", 1.2),
            kanan("Kebutuhan", 1.1),
            kanan("Biaya", 1.3),
            kanan("Skor", 0.8),
        ],
        &a.provinsi
            .iter()
            .map(|p| {
                let k = &p.r;
                vec![
                    p.nama.clone(),
                    unit(p.satker),
                    unit(k.unit),
                    rp(k.nilai),
                    pct(k.pct_baik(), k.kondisi_diketahui() > 0),
                    unit(k.rb),
                    unit(k.habis),
                    unit_f(k.rb_5()),
                    unit_f(k.ganti_total(1)),
                    rp(k.biaya_total(1)),
                    format_desimal(p.skor, 1),
                ]
            })
            .collect::<Vec<_>>(),
        None,
    );

    // 8. Satuan kerja.
    w.bab("Satuan Kerja Perlu Perhatian", None);
    let kol_satker = || {
        vec![
            kiri("Satuan kerja", 3.6),
            kiri("Provinsi", 1.9),
            kanan("Unit", 0.9),
            kanan("Baik", 0.9),
            kanan("RB", 0.7),
            kanan("Habis MM", 1.0),
            kanan("RB baru 5 th", 1.2),
            kanan("Kebutuhan", 1.1),
            kanan("Biaya", 1.3),
            kanan("RKU", 0.8),
            kanan("Skor", 0.8),
        ]
    };
    let baris_satker = |s: &SatkerBaris| {
        vec![
            s.nama.clone(),
            s.prov.clone(),
            unit(s.unit),
            format_persen(s.pct_baik),
            unit(s.rb),
            unit(s.habis),
            unit_f(s.rb5),
            unit_f(s.ganti5),
            rp(s.biaya5),
            format_desimal(s.rku, 2),
            format_desimal(s.skor, 1),
        ]
    };
    w.sub("Skor kesehatan terendah");
    w.tabel(&kol_satker(), &a.satker_skor.iter().map(baris_satker).collect::<Vec<_>>(), None);
    w.sub("Kebutuhan biaya terbesar (skenario moderat)");
    w.tabel(&kol_satker(), &a.satker_butuh.iter().map(baris_satker).collect::<Vec<_>>(), None);
    w.sub("Kerusakan paling jauh di atas kewajaran umur (RKU)");
    w.tabel(&kol_satker(), &a.satker_rku.iter().map(baris_satker).collect::<Vec<_>>(), None);
    w.catatan("RKU = Rasio Kerusakan terstandar Umur: aset tidak baik yang teramati dibagi yang wajar menurut umur aset. Di atas 1 berarti lebih rusak daripada wajarnya.");

    // 9. Mutu data.
    w.bab("Mutu Data dan Tertib Administrasi", None);
    w.tabel(
        &[
            kiri("Isian", 3.0),
            kiri("Lingkup", 2.2),
            kanan("Terisi", 1.2),
            kanan("Seharusnya", 1.2),
            kanan("Cakupan", 1.0),
        ],
        &a.kualitas
            .kolom
            .iter()
            .map(|k| {
                vec![
                    k.label.clone(),
                    k.lingkup.clone(),
                    unit(k.terisi),
                    unit(k.total),
                    pct(k.pct(), k.total > 0),
                ]
            })
            .collect::<Vec<_>>(),
        None,
    );
    w.tabel(
        &[kiri("Temuan administrasi", 5.0), kanan("Jumlah", 1.0), kanan("Nilai", 1.3)],
        &a.kualitas
            .anomali
            .iter()
            .filter(|x| x.jumlah > 0)
            .map(|x| {
                vec![
                    x.judul.clone(),
                    unit(x.jumlah),
                    if x.nilai > 0.0 { rp(x.nilai) } else { "-".into() },
                ]
            })
            .collect::<Vec<_>>(),
        None,
    );
    w.catatan(&a.kualitas.catatan_umur_sisa);

    bagian_metodologi(&mut w, m);

    // Lampiran: seluruh satuan kerja.
    w.halaman_baru();
    w.bab("Lampiran - Rincian Seluruh Satuan Kerja", Some("urut provinsi lalu nama"));
    let mut semua: Vec<&SatkerBaris> = satker.iter().collect();
    semua.sort_by(|x, y| x.prov.cmp(&y.prov).then(x.nama.cmp(&y.nama)));
    w.tabel(
        &kol_satker(),
        &semua.into_iter().map(baris_satker).collect::<Vec<_>>(),
        None,
    );
    w.catatan("Kebutuhan dan biaya = skenario moderat sampai akhir cakrawala termasuk tunggakan. Rincian tiap satuan kerja tersedia pada laporan satuan kerja.");

    w.selesai(&tanggal_pdf(&m.dibuat))
}

/// Susun laporan satu satuan kerja sebagai byte PDF.
pub fn bangun_satker(s: &SatkerRinci, periode: &Periode) -> Vec<u8> {
    let m = &s.meta;
    let r = &s.r;
    let akhir = m.tahun_akhir();
    let judul = format!("Laporan BMN {} - {}", s.nama, periode.label());
    let mut w = Penulis::baru(&judul, m.is_demo);
    sampul(
        &mut w,
        &s.nama,
        &format!("Profil Barang Milik Negara  |  {}  |  kode {}", s.prov, s.kode),
        m,
        periode,
    );

    w.bab("Ringkasan", Some(&format!("peringkat {} dari {} satuan kerja", s.peringkat, s.dari)));
    let (predikat, _) = Skor::predikat(s.skor.total);
    let banding = |nilai: f64, nasional: f64, tinggi_baik: bool| {
        let lebih = (nilai >= nasional) == tinggi_baik;
        format!(
            "{} {} poin dari rata-rata nasional {}",
            if lebih { "lebih baik" } else { "lebih buruk" },
            format_desimal((nilai - nasional).abs(), 1),
            format_persen(nasional)
        )
    };
    w.paragraf(&format!(
        "{} mengelola {} unit BMN senilai {} (nilai buku {}). Skor kesehatan BMN {} ({predikat}); nasional {}. \
         Kondisi baik {} ({}). Habis masa manfaat {} ({}). Sampai akhir {akhir}, sekitar {} unit lagi diperkirakan menjadi \
         rusak berat dan {} unit lagi habis masa manfaat; kebutuhan skenario moderat {} unit dengan perkiraan biaya {}.",
        s.nama,
        unit(r.unit),
        format_rupiah_kata(r.nilai),
        format_rupiah_kata(r.buku),
        format_desimal(s.skor.total, 1),
        format_desimal(s.skor_nasional, 1),
        pct(r.pct_baik(), r.kondisi_diketahui() > 0),
        banding(r.pct_baik(), s.nasional_pct_baik, true),
        pct(r.pct_habis(), r.susut_unit > 0),
        banding(r.pct_habis(), s.nasional_pct_habis, false),
        unit_f(r.rb_5()),
        unit_f(r.habis_5()),
        unit_f(r.ganti_total(1)),
        format_rupiah_kata(r.biaya_total(1)),
    ));
    bagian_kpi_utama(&mut w, r, &s.skor, m);

    w.sub("Komponen skor kesehatan");
    let komponen = [
        s.skor.kondisi,
        s.skor.umur,
        s.skor.pemanfaatan,
        s.skor.pengamanan,
        s.skor.tertib,
    ];
    for ((nama, bobot, _), nilai) in BOBOT_SKOR.iter().zip(komponen) {
        let y = w.bidang(15.0);
        w.hal.teks(MARGIN, y + 9.0, 8.0, false, TEKS, &format!("{nama} (bobot {:.0}%)", bobot * 100.0));
        bilah_nilai(
            &mut w.hal,
            MARGIN + 170.0,
            y + 3.0,
            LEBAR - 215.0,
            6.5,
            nilai,
            if nilai >= 80.0 { C_BAIK } else if nilai >= 65.0 { C_RR } else { C_RB },
        );
        w.hal.teks_kanan(MARGIN + LEBAR, y + 9.0, 8.0, true, TINTA, &format_desimal(nilai, 1));
    }
    w.jarak(4.0);
    w.sub("Sebaran kondisi aset fisik");
    let y = w.bidang(24.0);
    bilah_kondisi(&mut w.hal, MARGIN, y, LEBAR, 9.0, r.baik, r.rr, r.rb, r.tak);
    w.hal.teks(
        MARGIN,
        y + 20.0,
        7.5,
        false,
        REDUP,
        &format!(
            "Baik {} | rusak ringan {} | rusak berat {} | tidak diisi {} | umur rata-rata {} th | RKU {}",
            unit(r.baik),
            unit(r.rr),
            unit(r.rb),
            unit(r.tak),
            format_desimal(r.umur_rata(), 1),
            format_desimal(r.rku(), 2)
        ),
    );

    if !s.rekomendasi.is_empty() {
        w.halaman_baru();
        w.bab("Rekomendasi Tindak Lanjut", Some("urut prioritas"));
        for (i, x) in s.rekomendasi.iter().enumerate() {
            w.rekomendasi(i + 1, x);
        }
    }

    w.halaman_baru();
    w.bab("Prediksi dan Kebutuhan Lima Tahun", Some(&format!("sampai akhir {akhir}")));
    grafik_proyeksi(&mut w, r, m);
    w.sub("Kebutuhan per skenario (unit)");
    tabel_skenario(&mut w, r, m);

    w.sub("Rincian per kategori");
    w.tabel(
        &[
            kiri("Kategori", 3.0),
            kanan("Unit", 0.9),
            kanan("Nilai", 1.3),
            kanan("Baik", 0.9),
            kanan("R. ringan", 1.0),
            kanan("RB", 0.7),
            kanan("Habis MM", 1.0),
            kanan("Habis MM 5 th", 1.2),
            kanan("RB baru 5 th", 1.2),
            kanan("Kebutuhan", 1.1),
            kanan("Biaya", 1.3),
        ],
        &s.kategori
            .iter()
            .map(|c| {
                let k = &c.r;
                let f = |ada: bool, t: String| if ada { t } else { "-".into() };
                vec![
                    c.label.clone(),
                    unit(k.unit),
                    rp(k.nilai),
                    pct(k.pct_baik(), c.fisik && k.kondisi_diketahui() > 0),
                    f(c.fisik, unit(k.rr)),
                    f(c.fisik, unit(k.rb)),
                    f(c.disusutkan, unit(k.habis)),
                    f(c.disusutkan, unit_f(k.habis_5())),
                    f(c.fisik, unit_f(k.rb_5())),
                    f(c.kebutuhan != "-", unit_f(k.ganti_total(1))),
                    f(c.kebutuhan != "-", rp(k.biaya_total(1))),
                ]
            })
            .collect::<Vec<_>>(),
        None,
    );
    w.catatan("Kebutuhan dan biaya per kategori = skenario moderat termasuk tunggakan. MM = masa manfaat; RB = rusak berat.");

    w.bab("Daftar Aset Prioritas", Some("15 teratas tiap jenis"));
    w.sub("Aset rusak berat bernilai terbesar - calon usulan penghapusan");
    daftar_aset(&mut w, &s.aset_rb, false, false);
    w.sub("Aset paling berisiko menjadi rusak berat dalam 5 tahun");
    daftar_aset(&mut w, &s.aset_risiko, false, true);
    w.sub("Aset habis masa manfaat yang masih tercatat - bahan rencana peremajaan");
    daftar_aset(&mut w, &s.aset_habis, false, true);

    bagian_metodologi(&mut w, m);
    w.selesai(&tanggal_pdf(&m.dibuat))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analitik::mesin::{hitung, uji};

    fn contoh() -> crate::analitik::Hasil {
        let (f, kamus) = uji::fakta_contoh(&[
            "SIMAN2_M_ASET_ANGKUTAN_BERMOTOR",
            "SIMAN2_M_ASET_KHUSUS_TIK",
            "SIMAN2_M_ASET_GEDUNG_BANGUNAN",
            "SIMAN2_M_ASET_TANAH",
        ]);
        let mut k = uji::konteks();
        k.is_demo = true;
        hitung(&f, &kamus, &k)
    }

    #[test]
    fn laporan_nasional_lengkap_dan_sah() {
        let h = contoh();
        let periode = Periode { tahun: 2026, bulan: 6 };
        let aktivitas = AktivitasPeriode {
            label: periode.label(),
            unit_bulan: 12,
            nilai_bulan: 3e9,
            unit_tahun: 80,
            nilai_tahun: 2e10,
            per_kategori: vec![("Peralatan TIK".into(), 12, 3e9)],
        };
        let b = bangun_nasional(&h.analisis, &h.satker, &periode, &aktivitas);
        let teks = String::from_utf8_lossy(&b);
        assert!(teks.starts_with("%PDF-1.4"));
        assert!(teks.ends_with("%%EOF\n"));
        for bagian in [
            "Ringkasan Eksekutif",
            "Sorotan Utama",
            "Rekomendasi Tindak Lanjut",
            "Kondisi dan Masa Manfaat",
            "Prediksi Lima Tahun",
            "Analisis Kebutuhan Lima Tahun",
            "Forecast Perolehan",
            "Sebaran per Provinsi",
            "Satuan Kerja Perlu Perhatian",
            "Mutu Data dan Tertib Administrasi",
            "Metodologi, Asumsi, dan Keterbatasan",
            "Lampiran - Rincian Seluruh Satuan Kerja",
        ] {
            assert!(teks.contains(bagian), "bagian hilang: {bagian}");
        }
        assert!(teks.contains("Laporan Analitik BMN - Juni 2026"));
        assert!(teks.contains("DATA CONTOH"), "laporan data contoh wajib bertanda");
        // Seluruh satuan kerja masuk lampiran.
        assert!(teks.contains("Kejaksaan Negeri Jayawijaya"));
        // Penomoran halaman memakai jumlah halaman sebenarnya.
        let halaman = teks.matches("/Type /Page ").count();
        assert!(halaman >= 10, "halaman {halaman}");
        assert!(teks.contains(&format!("Halaman {halaman} dari {halaman}")));
        if let Ok(dir) = std::env::var("LAPORAN_TULIS_CONTOH") {
            let _ = std::fs::write(format!("{dir}/contoh-nasional.pdf"), &b);
        }
    }

    #[test]
    fn laporan_tanpa_data_tidak_panik() {
        let b = bangun_nasional(
            &Analisis::default(),
            &[],
            &Periode { tahun: 2026, bulan: 1 },
            &AktivitasPeriode::default(),
        );
        assert!(String::from_utf8_lossy(&b).starts_with("%PDF-1.4"));
        let b = bangun_satker(&SatkerRinci::default(), &Periode { tahun: 2026, bulan: 1 });
        assert!(String::from_utf8_lossy(&b).starts_with("%PDF-1.4"));
    }

    #[test]
    fn periode_parsing() {
        assert_eq!(
            Periode::dari_str("2026-10"),
            Some(Periode { tahun: 2026, bulan: 10 })
        );
        assert_eq!(Periode::dari_str("2026-13"), None);
        assert_eq!(Periode::dari_str("x"), None);
        let p = Periode { tahun: 2024, bulan: 2 };
        assert_eq!(p.label(), "Februari 2024");
        assert_eq!(p.kode(), "2024-02");
        assert_eq!(tanggal_pdf("2026-11-10 13:05 WIB"), "D:20261110130500+07'00'");
    }
}
