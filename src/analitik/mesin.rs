//! Mesin hitung analitik: dari fakta aset menjadi ringkasan, proyeksi, dan kebutuhan.
//!
//! Seluruh fungsi di sini murni (tanpa basis data) sehingga dapat diuji dengan
//! fakta buatan. Alurnya dua lintasan atas seluruh aset:
//!
//! 1. **Bangun model** — menaksir kurva kerusakan per kategori, masa manfaat
//!    efektif, harga satuan acuan per kelompok barang, dan faktor kerapuhan tiap
//!    satuan kerja (kerusakan teramati dibanding harapan menurut umur).
//! 2. **Agregasi** — tiap aset diturunkan menjadi umur, sisa masa manfaat,
//!    peluang rusak berat per tahun, dan kebutuhan per skenario; lalu dijumlahkan
//!    ke tingkat satuan kerja × kategori, provinsi, dan nasional.

#![cfg(feature = "ssr")]

use super::kerusakan::{self, Hitung, Param};
use super::prakiraan;
use crate::catalog::TABLES;
use crate::dto::*;
use crate::fakta::*;
use crate::model::{JenisKebutuhan, TableDef};
use std::collections::{BTreeMap, HashMap};

/// Umur tertua yang dipisahkan pada hitungan kondisi per umur.
const UMUR_MAKS: usize = 80;
/// Jendela harga satuan acuan: perolehan pada tahun-tahun terakhir.
const JENDELA_HARGA: i32 = 5;
/// Sampel minimum harga satuan acuan per kelompok barang.
const SAMPEL_HARGA: usize = 5;
/// Eskalasi nilai perolehan lama paling jauh dihitung sekian tahun.
const ESKALASI_MAKS: f64 = 20.0;
/// KDP yang belum selesai lebih dari sekian tahun dianggap mangkrak.
const KDP_MANGKRAK: f64 = 3.0;

const T_TANAH: &str = "SIMAN2_M_ASET_TANAH";
const T_RUMAH: &str = "SIMAN2_M_ASET_RUMAH";
const T_GEDUNG: &str = "SIMAN2_M_ASET_GEDUNG_BANGUNAN";
const T_KDP: &str = "SIMAN2_M_ASET_KDP";

/// Satu aset dalam bentuk ringkas untuk dihitung.
#[derive(Debug, Clone, PartialEq)]
pub struct Fakta {
    pub id: i64,
    pub tabel: usize,
    pub satker: usize,
    pub prov: usize,
    pub kelompok: usize,
    pub kondisi: i64,
    pub nilai: f64,
    pub buku: f64,
    pub tahun: i32,
    pub bulan: u32,
    pub umur_sisa: Option<f64>,
    pub flags: i64,
}

impl Fakta {
    fn ada(&self, f: i64) -> bool {
        self.flags & f != 0
    }
}

/// Kamus nama yang dirujuk fakta lewat indeks.
#[derive(Debug, Clone, Default)]
pub struct Kamus {
    pub satker_kode: Vec<String>,
    pub satker_nama: Vec<String>,
    pub prov: Vec<String>,
    pub kelompok: Vec<String>,
    idx_satker: HashMap<String, usize>,
    idx_prov: HashMap<String, usize>,
    idx_kelompok: HashMap<String, usize>,
}

impl Kamus {
    pub fn satker(&mut self, kode: &str, nama: &str) -> usize {
        if let Some(i) = self.idx_satker.get(kode) {
            return *i;
        }
        let i = self.satker_kode.len();
        self.satker_kode.push(kode.to_string());
        self.satker_nama.push(if nama.trim().is_empty() {
            "(Satuan kerja tidak diisi)".to_string()
        } else {
            judul(nama)
        });
        self.idx_satker.insert(kode.to_string(), i);
        i
    }

    pub fn prov(&mut self, nama: &str) -> usize {
        let kunci = nama.trim().to_uppercase();
        if let Some(i) = self.idx_prov.get(&kunci) {
            return *i;
        }
        let i = self.prov.len();
        self.prov.push(if kunci.is_empty() {
            "(Tidak diisi)".to_string()
        } else {
            judul(&kunci)
        });
        self.idx_prov.insert(kunci, i);
        i
    }

    pub fn kelompok(&mut self, nama: &str) -> usize {
        let t = nama.trim();
        if let Some(i) = self.idx_kelompok.get(t) {
            return *i;
        }
        let i = self.kelompok.len();
        self.kelompok.push(if t.is_empty() {
            "(Kelompok tidak diisi)".to_string()
        } else {
            t.to_string()
        });
        self.idx_kelompok.insert(t.to_string(), i);
        i
    }
}

/// Keadaan saat analitik dijalankan.
#[derive(Debug, Clone, Default)]
pub struct Konteks {
    pub tahun0: i32,
    pub bulan0: u32,
    pub asumsi: Asumsi,
    pub is_demo: bool,
    pub tanggal_data: String,
    pub dibuat: String,
    pub tabel_tersinkron: i64,
    pub kunci_ganda: i64,
}

impl Konteks {
    /// Titik waktu acuan sebagai tahun pecahan (pertengahan bulan data).
    pub fn kini(&self) -> f64 {
        self.tahun0 as f64 + (self.bulan0 as f64 - 0.5) / 12.0
    }

    /// Jarak (tahun) dari sekarang ke akhir tiap tahun proyeksi.
    pub fn batas(&self) -> [f64; NH] {
        let mut b = [0.0; NH];
        for (k, x) in b.iter_mut().enumerate() {
            *x = (self.tahun0 + k as i32 + 1) as f64 - self.kini();
        }
        b
    }

    /// Ubah `umur_sisa` sumber menjadi tahun.
    fn sisa_tahun(&self, mentah: f64) -> f64 {
        match self.asumsi.satuan_umur_sisa.as_str() {
            "semester" => mentah / 2.0,
            "bulan" => mentah / 12.0,
            _ => mentah,
        }
    }
}

/// Parameter hasil taksiran yang dipakai untuk menurunkan tiap aset.
#[derive(Debug, Clone, Default)]
pub struct Model {
    /// Parameter kerusakan per tabel katalog.
    pub param: Vec<Param>,
    /// Parameter berasal dari kurva gabungan/bawaan (sampel kategori kurang).
    pub gabungan: Vec<bool>,
    /// Masa manfaat efektif per tabel (tahun).
    pub masa: Vec<f64>,
    /// Rata-rata umur per tabel, cadangan bila tanggal perolehan tidak sah.
    pub umur_rata: Vec<f64>,
    /// Faktor kerapuhan per kode satuan kerja.
    pub phi: HashMap<String, f64>,
    /// Harga satuan acuan per tabel → kelompok barang.
    pub harga: Vec<HashMap<String, f64>>,
    /// Hitungan kondisi per umur per tabel (untuk kurva pembanding).
    pub hitung: Vec<Vec<Hitung>>,
}

fn median(v: &mut [f64]) -> Option<f64> {
    if v.is_empty() {
        return None;
    }
    v.sort_by(|a, b| a.total_cmp(b));
    let n = v.len();
    Some(if n % 2 == 1 {
        v[n / 2]
    } else {
        (v[n / 2 - 1] + v[n / 2]) / 2.0
    })
}

fn umur_aset(tahun: i32, bulan: u32, k: &Konteks) -> Option<f64> {
    if tahun <= 0 || tahun > k.tahun0 {
        return None;
    }
    let b = if (1..=12).contains(&bulan) { bulan } else { 6 };
    Some((k.kini() - (tahun as f64 + (b as f64 - 0.5) / 12.0)).max(0.0))
}

/// Lintasan pertama: taksir model dari seluruh fakta.
pub fn bangun_model(fakta: &[Fakta], kamus: &Kamus, k: &Konteks) -> Model {
    let nt = TABLES.len();
    let mut hitung = vec![vec![Hitung::default(); UMUR_MAKS]; nt];
    let mut masa_sampel: Vec<Vec<f64>> = vec![Vec::new(); nt];
    let mut harga_sampel: Vec<HashMap<usize, Vec<f64>>> = vec![HashMap::new(); nt];
    let mut umur_jml = vec![(0.0_f64, 0.0_f64); nt];

    for f in fakta {
        if f.ada(F_DIHAPUS) {
            continue;
        }
        let def = &TABLES[f.tabel];
        let umur = umur_aset(f.tahun, f.bulan, k);
        if let Some(a) = umur {
            umur_jml[f.tabel].0 += a;
            umur_jml[f.tabel].1 += 1.0;
            if def.fisik && f.kondisi != K_TAK {
                let h = &mut hitung[f.tabel][(a.floor() as usize).min(UMUR_MAKS - 1)];
                h.n += 1.0;
                if f.kondisi == K_BAIK {
                    h.baik += 1.0;
                } else if f.kondisi == K_RB {
                    h.rb += 1.0;
                }
            }
            if def.disusutkan {
                if let Some(s) = f.umur_sisa.map(|x| k.sisa_tahun(x)).filter(|x| *x > 0.0) {
                    masa_sampel[f.tabel].push(a + s);
                }
            }
        }
        if f.tahun >= k.tahun0 - JENDELA_HARGA && f.tahun <= k.tahun0 && f.nilai > 1000.0 {
            harga_sampel[f.tabel]
                .entry(f.kelompok)
                .or_default()
                .push(f.nilai);
        }
    }

    let masa: Vec<f64> = (0..nt)
        .map(|t| {
            if masa_sampel[t].len() >= 20 {
                median(&mut masa_sampel[t]).unwrap_or(TABLES[t].masa_manfaat)
            } else {
                TABLES[t].masa_manfaat
            }
        })
        .collect();

    // Kurva gabungan pada umur relatif (umur ÷ masa manfaat, pita 0,1) sebagai
    // cadangan kategori yang sampelnya terlalu sedikit.
    let mut relatif = vec![Hitung::default(); UMUR_MAKS];
    for t in 0..nt {
        if !TABLES[t].fisik || masa[t] <= 0.0 {
            continue;
        }
        for (a, h) in hitung[t].iter().enumerate() {
            if h.n <= 0.0 {
                continue;
            }
            let u = (((a as f64 + 0.5) / masa[t]) * 10.0).floor() as usize;
            let r = &mut relatif[u.min(UMUR_MAKS - 1)];
            r.n += h.n;
            r.baik += h.baik;
            r.rb += h.rb;
        }
    }
    let gabung = kerusakan::taksir(&relatif);

    let mut param = Vec::with_capacity(nt);
    let mut gabungan = Vec::with_capacity(nt);
    for t in 0..nt {
        let def = &TABLES[t];
        if !def.fisik {
            param.push(Param::bawaan(def.masa_manfaat));
            gabungan.push(true);
            continue;
        }
        match kerusakan::taksir(&hitung[t]) {
            Some(p) => {
                param.push(p);
                gabungan.push(false);
            }
            None => {
                let m = if masa[t] > 0.0 { masa[t] } else { 10.0 };
                param.push(match gabung {
                    Some(g) => Param {
                        eta: g.eta * m / 10.0,
                        beta: g.beta,
                        mu: g.mu * 10.0 / m,
                    },
                    None => Param::bawaan(m),
                });
                gabungan.push(true);
            }
        }
    }

    // Kerapuhan satuan kerja: kerusakan teramati dibanding harapan menurut umur.
    let mut oe: Vec<(f64, f64)> = vec![(0.0, 0.0); kamus.satker_kode.len()];
    for f in fakta {
        if f.ada(F_DIHAPUS) || !TABLES[f.tabel].fisik || f.kondisi == K_TAK {
            continue;
        }
        if let Some(a) = umur_aset(f.tahun, f.bulan, k) {
            let e = &mut oe[f.satker];
            if f.kondisi != K_BAIK {
                e.0 += 1.0;
            }
            e.1 += 1.0 - param[f.tabel].s_baik(a, 1.0);
        }
    }
    let phi = kamus
        .satker_kode
        .iter()
        .enumerate()
        .map(|(i, kode)| {
            let r = Ringkas {
                rusak_obs: oe[i].0,
                rusak_harap: oe[i].1,
                ..Default::default()
            };
            (kode.clone(), r.rku().clamp(0.5, 2.5))
        })
        .collect();

    let harga = harga_sampel
        .into_iter()
        .map(|peta| {
            peta.into_iter()
                .filter(|(_, v)| v.len() >= SAMPEL_HARGA)
                .filter_map(|(ki, mut v)| median(&mut v).map(|m| (kamus.kelompok[ki].clone(), m)))
                .collect()
        })
        .collect();

    Model {
        param,
        gabungan,
        masa,
        umur_rata: umur_jml
            .iter()
            .map(|(j, n)| if *n > 0.0 { j / n } else { 0.0 })
            .collect(),
        phi,
        harga,
        hitung,
    }
}

/// Besaran turunan satu aset.
#[derive(Debug, Clone, Default)]
pub struct Turunan {
    pub umur: Option<f64>,
    /// Sisa masa manfaat (tahun) bagi aset yang disusutkan.
    pub sisa: Option<f64>,
    pub habis: bool,
    /// Indeks tahun proyeksi saat masa manfaat habis (bila dalam cakrawala).
    pub idx_habis: Option<usize>,
    /// Peluang kumulatif tidak baik dan rusak berat pada akhir tiap tahun proyeksi.
    pub tb: [f64; NH],
    pub rb: [f64; NH],
    /// Proyeksi kondisi berlaku (aset fisik yang belum rusak berat dan tidak hilang).
    pub diproyeksikan: bool,
    pub butuh_kini: [f64; 3],
    pub butuh_th: [[f64; NH]; 3],
    /// Harga satuan acuan dan apakah berasal dari median perolehan terbaru.
    pub harga: f64,
    pub harga_median: bool,
    /// `umur_sisa` nol padahal masih bernilai buku (tidak konsisten).
    pub sisa_janggal: bool,
}

/// Turunkan besaran analitik satu aset dari model.
pub fn turunkan(
    f: &Fakta,
    kelompok: &str,
    kode_satker: &str,
    m: &Model,
    k: &Konteks,
) -> Turunan {
    let def: &TableDef = &TABLES[f.tabel];
    let mut t = Turunan {
        umur: umur_aset(f.tahun, f.bulan, k),
        ..Default::default()
    };
    let masa = if m.masa[f.tabel] > 0.0 {
        m.masa[f.tabel]
    } else {
        def.masa_manfaat
    };

    if def.disusutkan {
        let porsi_buku = if f.nilai > 1.0 {
            Some((f.buku / f.nilai).clamp(0.0, 1.0))
        } else {
            None
        };
        let dari_buku = porsi_buku.map(|p| masa * p);
        t.sisa = match f.umur_sisa.map(|x| k.sisa_tahun(x)) {
            Some(x) if x > 0.0 => Some(x),
            Some(_) => {
                // Nol yang sah berarti nilai buku juga (hampir) habis.
                if porsi_buku.is_none_or(|p| p <= 0.02) {
                    Some(0.0)
                } else {
                    t.sisa_janggal = true;
                    dari_buku
                }
            }
            None => dari_buku.or(t.umur.map(|a| (masa - a).max(0.0))),
        };
        if let Some(s) = t.sisa {
            if s <= 0.0 {
                t.habis = true;
            } else {
                let idx = (k.kini() + s).floor() as i64 - k.tahun0 as i64;
                if (0..NH as i64).contains(&idx) {
                    t.idx_habis = Some(idx as usize);
                }
            }
        }
    }

    let batas = k.batas();
    let hilang = f.ada(F_HILANG);
    if def.fisik && f.kondisi != K_RB && !hilang {
        t.diproyeksikan = true;
        let p = &m.param[f.tabel];
        if f.kondisi == K_RR {
            for i in 0..NH {
                t.tb[i] = 1.0;
                t.rb[i] = p.rb_dari_rr(batas[i]);
            }
        } else {
            // Kondisi baik (atau tidak diisi, diperlakukan sebagai baik).
            let a = t.umur.unwrap_or(m.umur_rata[f.tabel]);
            let phi = m.phi.get(kode_satker).copied().unwrap_or(1.0);
            for (i, (tb, rb)) in p.lintasan_baik(a, phi, &batas).into_iter().enumerate() {
                t.tb[i] = tb;
                t.rb[i] = rb;
            }
        }
    }

    if def.kebutuhan != JenisKebutuhan::TidakAda && !hilang {
        let rusak_berat = f.kondisi == K_RB;
        let kini = [
            rusak_berat,
            rusak_berat || (f.kondisi == K_RR && t.habis),
            rusak_berat || t.habis,
        ];
        let lewat = |i: usize| t.sisa.is_some_and(|s| s <= batas[i]);
        for s in 0..3 {
            if kini[s] {
                t.butuh_kini[s] = 1.0;
                continue;
            }
            let mut sebelum = 0.0;
            for i in 0..NH {
                let kum = match s {
                    0 => t.rb[i],
                    1 => {
                        if lewat(i) {
                            t.tb[i]
                        } else {
                            t.rb[i]
                        }
                    }
                    _ => {
                        if lewat(i) {
                            1.0
                        } else {
                            t.rb[i]
                        }
                    }
                };
                t.butuh_th[s][i] = (kum - sebelum).max(0.0);
                sebelum = sebelum.max(kum);
            }
        }
        let e = k.asumsi.eskalasi;
        match def.kebutuhan {
            JenisKebutuhan::Penggantian => match m.harga[f.tabel].get(kelompok) {
                Some(h) => {
                    t.harga = *h;
                    t.harga_median = true;
                }
                None => {
                    let lama = t.umur.unwrap_or(0.0).min(ESKALASI_MAKS);
                    t.harga = f.nilai * (1.0 + e).powf(lama);
                }
            },
            _ => t.harga = f.nilai * k.asumsi.faktor_rehab,
        }
    }
    t
}

/// Catat satu aset ke sebuah ringkasan.
fn catat(r: &mut Ringkas, f: &Fakta, def: &TableDef, t: &Turunan, m: &Model, k: &Konteks) {
    r.unit += 1;
    r.nilai += f.nilai;
    r.buku += f.buku;
    if def.fisik {
        r.fisik += 1;
        match f.kondisi {
            K_BAIK => r.baik += 1,
            K_RR => r.rr += 1,
            K_RB => {
                r.rb += 1;
                r.nilai_rb += f.nilai;
            }
            _ => r.tak += 1,
        }
        if let (Some(a), true) = (t.umur, f.kondisi != K_TAK) {
            if f.kondisi != K_BAIK {
                r.rusak_obs += 1.0;
            }
            r.rusak_harap += 1.0 - m.param[f.tabel].s_baik(a, 1.0);
        }
        if matches!(f.kondisi, K_BAIK | K_RR)
            && !f.ada(F_IDLE)
            && !f.ada(F_HILANG)
            && !f.ada(F_DIHENTIKAN)
        {
            r.layak_pelihara += 1;
        }
    }
    if f.ada(F_IDLE) {
        r.idle += 1;
        r.nilai_idle += f.nilai;
    }
    if f.ada(F_HILANG) {
        r.hilang += 1;
        r.nilai_hilang += f.nilai;
    }
    if t.sisa.is_some() {
        r.susut_unit += 1;
        if t.habis {
            r.habis += 1;
            r.nilai_habis += f.nilai;
        } else if let Some(i) = t.idx_habis {
            r.habis_th[i] += 1.0;
        }
    }
    if t.diproyeksikan {
        let mut sebelum = 0.0;
        for i in 0..NH {
            r.rb_th[i] += (t.rb[i] - sebelum).max(0.0);
            sebelum = t.rb[i];
        }
    }
    let e = k.asumsi.eskalasi;
    for s in 0..3 {
        r.ganti_kini[s] += t.butuh_kini[s];
        r.biaya_kini[s] += t.butuh_kini[s] * t.harga;
        for i in 0..NH {
            r.ganti_th[s][i] += t.butuh_th[s][i];
            r.biaya_th[s][i] += t.butuh_th[s][i] * t.harga * (1.0 + e).powi(i as i32);
        }
    }
    if let Some(a) = t.umur {
        r.umur_jml += a;
        r.umur_n += 1;
    }
    if def.table != T_KDP {
        r.psp_n += 1;
        if f.ada(F_PSP) {
            r.psp_ya += 1;
        }
    }
    if def.table == T_TANAH {
        r.tanah_n += 1;
        if f.ada(F_BUKTI) {
            r.sertif_ya += 1;
        }
    }
    r.lengkap_n += 1;
    if (!def.fisik || f.kondisi != K_TAK) && t.umur.is_some() && f.nilai > 1.0 {
        r.lengkap_ya += 1;
    }
}

/// Ringkasan tersimpan satu satuan kerja.
#[derive(Debug, Clone, Default)]
pub struct SatkerSimpan {
    pub kode: String,
    pub nama: String,
    pub prov: String,
    pub r: Ringkas,
    /// Ringkasan per tabel katalog (indeks, ringkasan).
    pub per_tabel: Vec<(usize, Ringkas)>,
    pub skor: Skor,
    pub peringkat: i64,
}

/// Hasil lengkap satu putaran analitik.
#[derive(Debug, Clone, Default)]
pub struct Hasil {
    pub versi: i64,
    pub analisis: Analisis,
    pub satker: Vec<SatkerBaris>,
    pub simpan: HashMap<String, SatkerSimpan>,
    pub model: Model,
    pub konteks: Konteks,
}

fn baris_satker(s: &SatkerSimpan) -> SatkerBaris {
    let r = &s.r;
    SatkerBaris {
        kode: s.kode.clone(),
        nama: s.nama.clone(),
        prov: s.prov.clone(),
        unit: r.unit,
        nilai: r.nilai,
        pct_baik: r.pct_baik(),
        rr: r.rr,
        rb: r.rb,
        habis: r.habis,
        pct_habis: r.pct_habis(),
        idle: r.idle,
        hilang: r.hilang,
        rb5: r.rb_5(),
        habis5: r.habis_5(),
        ganti5: r.ganti_total(1),
        biaya5: r.biaya_total(1),
        rku: r.rku(),
        skor: s.skor.total,
    }
}

fn label_pita(i: usize) -> &'static str {
    ["0-2 th", "3-5 th", "6-10 th", "11-15 th", "16-20 th", "21-30 th", "> 30 th"][i]
}

fn pita_umur(a: f64) -> usize {
    match a {
        x if x < 3.0 => 0,
        x if x < 6.0 => 1,
        x if x < 11.0 => 2,
        x if x < 16.0 => 3,
        x if x < 21.0 => 4,
        x if x < 31.0 => 5,
        _ => 6,
    }
}

/// Susun satu entri daftar aset prioritas.
pub fn aset_dari(
    f: &Fakta,
    t: &Turunan,
    kelompok: &str,
    satker: &str,
    kode_satker: &str,
) -> AsetRisiko {
    AsetRisiko {
        id: f.id,
        kategori: TABLES[f.tabel].label.to_string(),
        kelompok: kelompok.to_string(),
        satker: satker.to_string(),
        kode_satker: kode_satker.to_string(),
        tahun: f.tahun,
        umur: t.umur.unwrap_or(-1.0),
        kondisi: label_kondisi(f.kondisi).to_string(),
        nilai: f.nilai,
        sisa: t.sisa.unwrap_or(-1.0),
        p_rb: if f.kondisi == K_RB {
            1.0
        } else if t.diproyeksikan {
            t.rb[NH - 1]
        } else {
            0.0
        },
        ..Default::default()
    }
}

/// Kurva pembanding model terhadap data untuk satu kategori.
fn kurva(hitung: &[Hitung], p: &Param) -> Vec<TitikKurva> {
    let terakhir = hitung.iter().rposition(|h| h.n > 0.0).unwrap_or(0);
    let lebar = match terakhir {
        0..=14 => 1,
        15..=29 => 2,
        _ => 5,
    };
    let mut out = Vec::new();
    let mut a = 0;
    while a <= terakhir {
        let pita = &hitung[a..(a + lebar).min(hitung.len())];
        let n: f64 = pita.iter().map(|h| h.n).sum();
        if n >= 5.0 {
            let baik: f64 = pita.iter().map(|h| h.baik).sum();
            let rb: f64 = pita.iter().map(|h| h.rb).sum();
            // Titik model memakai rata-rata berbobot sampel dalam pita.
            let (mut mtb, mut mrb) = (0.0, 0.0);
            for (i, h) in pita.iter().enumerate() {
                let umur = (a + i) as f64 + 0.5;
                mtb += h.n * (1.0 - p.s_baik(umur, 1.0));
                mrb += h.n * p.p_rb(umur, 1.0);
            }
            out.push(TitikKurva {
                umur: a as f64 + lebar as f64 / 2.0,
                n: n as i64,
                obs_tb: (n - baik) / n,
                obs_rb: rb / n,
                mod_tb: mtb / n,
                mod_rb: mrb / n,
            });
        }
        a += lebar;
    }
    out
}

/// Selisih mutlak rata-rata model terhadap data pada pita umur (poin persentase).
///
/// Dihitung pada pita tampilan, bukan per tahun umur, agar tidak didominasi
/// derau sampel kecil.
fn mae_pita(titik: &[TitikKurva]) -> f64 {
    let n: f64 = titik.iter().map(|t| t.n as f64).sum();
    if n <= 0.0 {
        return 0.0;
    }
    100.0
        * titik
            .iter()
            .map(|t| {
                t.n as f64 * ((t.obs_tb - t.mod_tb).abs() + (t.obs_rb - t.mod_rb).abs()) / 2.0
            })
            .sum::<f64>()
        / n
}

#[derive(Default)]
struct ButuhKelompok {
    unit_total: i64,
    unit: [f64; 3],
    biaya: [f64; 3],
    harga_jml: f64,
    harga_n: f64,
    median: bool,
    idle_baik: i64,
}

#[derive(Default)]
struct Mutu {
    fisik: i64,
    fisik_isi: i64,
    semua: i64,
    tgl_sah: i64,
    nilai_wajar: i64,
    satker_isi: i64,
    susut: i64,
    susut_isi: i64,
    lokasi_n: i64,
    lokasi_gps: i64,
    foto: i64,
    nilai_kecil: (i64, f64),
    tgl_janggal: i64,
    baik_tapi_rusak: (i64, f64),
    rb_aktif: (i64, f64),
    sisa_janggal: (i64, f64),
    kdp_mangkrak: (i64, f64),
    sengketa: (i64, f64),
    rumah_kosong: (i64, f64),
    rasio: Vec<f64>,
}

/// Lintasan kedua: hitung seluruh ringkasan, proyeksi, dan kebutuhan.
pub fn hitung(fakta: &[Fakta], kamus: &Kamus, k: &Konteks) -> Hasil {
    let nt = TABLES.len();
    let model = bangun_model(fakta, kamus, k);
    let batas = k.batas();
    let e = k.asumsi.eskalasi;

    let mut per_st: HashMap<(usize, usize), Ringkas> = HashMap::new();
    let mut per_prov: Vec<Ringkas> = vec![Ringkas::default(); kamus.prov.len()];
    let mut prov_satker: HashMap<(usize, usize), i64> = HashMap::new();
    let mut tren: BTreeMap<i32, Vec<(i64, f64)>> = BTreeMap::new();
    let mut pita: Vec<UmurPita> = (0..7)
        .map(|i| UmurPita {
            label: label_pita(i).to_string(),
            ..Default::default()
        })
        .collect();
    let mut calon_rb: Vec<(f64, usize)> = Vec::new();
    let mut calon_risiko: Vec<(f64, f64, usize)> = Vec::new();
    let mut butuh: HashMap<(usize, usize), ButuhKelompok> = HashMap::new();
    // Indeks 0 = posisi kini; 1..=NH = akhir tiap tahun proyeksi.
    let mut buku_th = vec![(0.0_f64, 0.0_f64); NH + 1];
    let mut mutu = Mutu::default();
    let mut dikecualikan = 0i64;

    for (fi, f) in fakta.iter().enumerate() {
        if f.ada(F_DIHAPUS) {
            dikecualikan += 1;
            continue;
        }
        let def = &TABLES[f.tabel];
        let kelompok = kamus.kelompok[f.kelompok].as_str();
        let kode = kamus.satker_kode[f.satker].as_str();
        let t = turunkan(f, kelompok, kode, &model, k);

        catat(
            per_st.entry((f.satker, f.tabel)).or_default(),
            f,
            def,
            &t,
            &model,
            k,
        );
        catat(&mut per_prov[f.prov], f, def, &t, &model, k);
        *prov_satker.entry((f.satker, f.prov)).or_default() += 1;

        if f.tahun > 0 && f.tahun <= k.tahun0 {
            let baris = tren.entry(f.tahun).or_insert_with(|| vec![(0, 0.0); nt]);
            baris[f.tabel].0 += 1;
            baris[f.tabel].1 += f.nilai;
        }
        if let (true, Some(a)) = (def.fisik, t.umur) {
            let p = &mut pita[pita_umur(a)];
            match f.kondisi {
                K_BAIK => p.baik += 1,
                K_RR => p.rr += 1,
                K_RB => p.rb += 1,
                _ => p.tak += 1,
            }
            p.nilai += f.nilai;
        }
        if def.fisik && f.kondisi == K_RB {
            calon_rb.push((f.nilai, fi));
        }
        if t.diproyeksikan && t.rb[NH - 1] >= 0.25 {
            calon_risiko.push((t.rb[NH - 1], f.nilai, fi));
            if calon_risiko.len() > 20_000 {
                calon_risiko.sort_by(|a, b| b.0.total_cmp(&a.0).then(b.1.total_cmp(&a.1)));
                calon_risiko.truncate(2_000);
            }
        }
        if def.kebutuhan != JenisKebutuhan::TidakAda {
            let b = butuh.entry((f.tabel, f.kelompok)).or_default();
            b.unit_total += 1;
            for s in 0..3 {
                b.unit[s] += t.butuh_kini[s] + t.butuh_th[s].iter().sum::<f64>();
                b.biaya[s] += t.butuh_kini[s] * t.harga;
                for i in 0..NH {
                    b.biaya[s] += t.butuh_th[s][i] * t.harga * (1.0 + e).powi(i as i32);
                }
            }
            if t.harga > 0.0 {
                b.harga_jml += t.harga;
                b.harga_n += 1.0;
            }
            b.median |= t.harga_median;
            if f.ada(F_IDLE) && f.kondisi == K_BAIK {
                b.idle_baik += 1;
            }
        }

        // Proyeksi nilai buku portofolio yang ada (garis lurus atas sisa umur).
        let peralatan = def.kebutuhan == JenisKebutuhan::Penggantian;
        for i in 0..=NH {
            let waktu = if i == 0 { 0.0 } else { batas[i - 1] };
            let b = match t.sisa {
                Some(s) if s > 0.0 => f.buku * (1.0 - waktu / s).max(0.0),
                _ => f.buku,
            };
            buku_th[i].0 += b;
            if peralatan {
                buku_th[i].1 += b;
            }
        }

        // Mutu data dan kepatuhan.
        mutu.semua += 1;
        if def.fisik {
            mutu.fisik += 1;
            if f.kondisi != K_TAK {
                mutu.fisik_isi += 1;
            }
        }
        if t.umur.is_some() {
            mutu.tgl_sah += 1;
        } else {
            mutu.tgl_janggal += 1;
        }
        if f.nilai > 1.0 {
            mutu.nilai_wajar += 1;
        } else {
            mutu.nilai_kecil.0 += 1;
            mutu.nilai_kecil.1 += f.nilai;
        }
        if kode != "TANPA-SATKER" {
            mutu.satker_isi += 1;
        }
        if def.disusutkan {
            mutu.susut += 1;
            if f.umur_sisa.is_some() {
                mutu.susut_isi += 1;
            }
        }
        if matches!(def.table, T_TANAH | T_GEDUNG | T_RUMAH) {
            mutu.lokasi_n += 1;
            if f.ada(F_GPS) {
                mutu.lokasi_gps += 1;
            }
        }
        if f.ada(F_FOTO) {
            mutu.foto += 1;
        }
        let tambah = |x: &mut (i64, f64)| {
            x.0 += 1;
            x.1 += f.nilai;
        };
        if def.fisik && f.kondisi == K_BAIK && f.ada(F_RUSAK_YN) {
            tambah(&mut mutu.baik_tapi_rusak);
        }
        if def.fisik && f.kondisi == K_RB && !f.ada(F_DIHENTIKAN) {
            tambah(&mut mutu.rb_aktif);
        }
        if t.sisa_janggal {
            tambah(&mut mutu.sisa_janggal);
        }
        if def.table == T_KDP && t.umur.is_some_and(|a| a > KDP_MANGKRAK) {
            tambah(&mut mutu.kdp_mangkrak);
        }
        if f.ada(F_SENGKETA) {
            tambah(&mut mutu.sengketa);
        }
        if def.table == T_RUMAH && !f.ada(F_PENGHUNI) {
            tambah(&mut mutu.rumah_kosong);
        }
        // Uji satuan umur_sisa: bandingkan dengan taksiran dari porsi nilai buku.
        if let (Some(s), Some(a)) = (f.umur_sisa.map(|x| k.sisa_tahun(x)), t.umur) {
            if s > 0.0 && a >= 1.0 && f.nilai > 1000.0 && mutu.rasio.len() < 200_000 {
                let porsi = f.buku / f.nilai;
                if porsi > 0.05 && porsi < 0.95 {
                    mutu.rasio.push((s / porsi) / (a / (1.0 - porsi)));
                }
            }
        }
    }

    // Gulung ke satuan kerja, kategori, dan nasional.
    let mut per_tabel: Vec<Ringkas> = vec![Ringkas::default(); nt];
    let mut satker_r: Vec<Ringkas> = vec![Ringkas::default(); kamus.satker_kode.len()];
    let mut satker_tabel: Vec<Vec<(usize, Ringkas)>> = vec![Vec::new(); kamus.satker_kode.len()];
    let mut nasional = Ringkas::default();
    let mut kunci: Vec<(usize, usize)> = per_st.keys().copied().collect();
    kunci.sort();
    for key in kunci {
        let mut r = per_st.remove(&key).unwrap_or_default();
        per_tabel[key.1].gabung(&r);
        satker_r[key.0].gabung(&r);
        nasional.gabung(&r);
        r.bulatkan();
        satker_tabel[key.0].push((key.1, r));
    }

    // Provinsi utama tiap satuan kerja = lokasi sebagian besar asetnya.
    let mut prov_utama: Vec<(usize, i64)> = vec![(0, -1); kamus.satker_kode.len()];
    for ((s, p), n) in &prov_satker {
        if *n > prov_utama[*s].1 || (*n == prov_utama[*s].1 && *p < prov_utama[*s].0) {
            prov_utama[*s] = (*p, *n);
        }
    }

    let mut simpan: Vec<SatkerSimpan> = Vec::new();
    for (i, kode) in kamus.satker_kode.iter().enumerate() {
        if satker_r[i].unit == 0 {
            continue;
        }
        let mut r = std::mem::take(&mut satker_r[i]);
        let skor = Skor::dari(&r);
        r.bulatkan();
        simpan.push(SatkerSimpan {
            kode: kode.clone(),
            nama: kamus.satker_nama[i].clone(),
            prov: kamus.prov.get(prov_utama[i].0).cloned().unwrap_or_default(),
            r,
            per_tabel: std::mem::take(&mut satker_tabel[i]),
            skor,
            peringkat: 0,
        });
    }
    simpan.sort_by(|a, b| b.skor.total.total_cmp(&a.skor.total).then(a.nama.cmp(&b.nama)));
    for (i, s) in simpan.iter_mut().enumerate() {
        s.peringkat = i as i64 + 1;
    }
    let mut satker: Vec<SatkerBaris> = simpan.iter().map(baris_satker).collect();
    satker.sort_by(|a, b| a.nama.cmp(&b.nama));

    let ambil = |urut: &dyn Fn(&SatkerBaris, &SatkerBaris) -> std::cmp::Ordering,
                 saring: &dyn Fn(&SatkerBaris) -> bool,
                 n: usize|
     -> Vec<SatkerBaris> {
        let mut v: Vec<SatkerBaris> = satker.iter().filter(|s| saring(s)).cloned().collect();
        v.sort_by(|a, b| urut(a, b));
        v.truncate(n);
        v
    };
    // Satuan kerja sangat kecil tidak diperingkat agar tidak menyesatkan.
    let cukup = |s: &SatkerBaris| s.unit >= 20;
    let satker_skor = ambil(&|a, b| a.skor.total_cmp(&b.skor), &cukup, 10);
    let satker_butuh = ambil(&|a, b| b.biaya5.total_cmp(&a.biaya5), &|_| true, 10);
    let satker_rb = ambil(&|a, b| b.rb5.total_cmp(&a.rb5), &|_| true, 10);
    let satker_rku = ambil(&|a, b| b.rku.total_cmp(&a.rku), &cukup, 10);

    // Kategori.
    let kategori: Vec<KategoriBaris> = (0..nt)
        .filter(|t| per_tabel[*t].unit > 0)
        .map(|t| {
            let def = &TABLES[t];
            let mut r = per_tabel[t].clone();
            r.bulatkan();
            KategoriBaris {
                tabel: def.table.to_string(),
                label: def.label.to_string(),
                icon: def.icon.to_string(),
                fisik: def.fisik,
                disusutkan: def.disusutkan,
                kebutuhan: def.kebutuhan.label().to_string(),
                masa_manfaat: (model.masa[t] * 10.0).round() / 10.0,
                r,
            }
        })
        .collect();

    // Provinsi.
    let mut prov_n_satker: Vec<i64> = vec![0; kamus.prov.len()];
    for s in &simpan {
        if let Some(i) = kamus.prov.iter().position(|p| *p == s.prov) {
            prov_n_satker[i] += 1;
        }
    }
    let mut provinsi: Vec<ProvBaris> = per_prov
        .into_iter()
        .enumerate()
        .filter(|(_, r)| r.unit > 0)
        .map(|(i, mut r)| {
            let skor = Skor::dari(&r).total;
            r.bulatkan();
            ProvBaris {
                nama: kamus.prov[i].clone(),
                satker: prov_n_satker[i],
                skor,
                r,
            }
        })
        .collect();
    provinsi.sort_by(|a, b| b.r.nilai.total_cmp(&a.r.nilai));

    // Tren dan prakiraan perolehan.
    let tahun_awal = k.tahun0 - 29;
    let tren_total: Vec<TrenTahun> = tren
        .iter()
        .filter(|(y, _)| **y >= tahun_awal)
        .map(|(y, v)| TrenTahun {
            tahun: *y,
            unit: v.iter().map(|x| x.0).sum(),
            nilai: v.iter().map(|x| x.1).sum(),
        })
        .collect();
    let deret = |pilih: &dyn Fn(&Vec<(i64, f64)>) -> (f64, f64)| -> (Vec<(i32, f64)>, Vec<(i32, f64)>, (f64, f64)) {
        // Tahun penuh saja; tahun berjalan belum selesai sehingga dipisahkan.
        let awal = tren.keys().next().copied().unwrap_or(k.tahun0).max(k.tahun0 - 20);
        let mut unit = Vec::new();
        let mut nilai = Vec::new();
        for y in awal..k.tahun0 {
            let (u, n) = tren.get(&y).map(pilih).unwrap_or((0.0, 0.0));
            unit.push((y, u));
            nilai.push((y, n));
        }
        let berjalan = tren.get(&k.tahun0).map(pilih).unwrap_or((0.0, 0.0));
        (unit, nilai, berjalan)
    };
    let mut prakiraan_semua = Vec::new();
    {
        let (u, n, b) = deret(&|v| {
            (
                v.iter().map(|x| x.0 as f64).sum(),
                v.iter().map(|x| x.1).sum(),
            )
        });
        prakiraan_semua.push(Prakiraan {
            kunci: "TOTAL".into(),
            label: "Seluruh kategori".into(),
            unit: prakiraan::ramal(&u, NH),
            nilai: prakiraan::ramal(&n, NH),
            berjalan: b,
        });
    }
    for t in 0..nt {
        if per_tabel[t].unit == 0 {
            continue;
        }
        let (u, n, b) = deret(&|v| (v[t].0 as f64, v[t].1));
        prakiraan_semua.push(Prakiraan {
            kunci: TABLES[t].table.into(),
            label: TABLES[t].label.into(),
            unit: prakiraan::ramal(&u, NH),
            nilai: prakiraan::ramal(&n, NH),
            berjalan: b,
        });
    }

    // Kapasitas pengadaan historis kategori yang dapat diganti/direhabilitasi.
    let mut kapasitas = Kapasitas {
        tahun_awal: k.tahun0 - 5,
        tahun_akhir: k.tahun0 - 1,
        ..Default::default()
    };
    for y in kapasitas.tahun_awal..=kapasitas.tahun_akhir {
        if let Some(v) = tren.get(&y) {
            for t in 0..nt {
                if TABLES[t].kebutuhan != JenisKebutuhan::TidakAda {
                    kapasitas.unit_per_tahun += v[t].0 as f64 / 5.0;
                    kapasitas.nilai_per_tahun += v[t].1 / 5.0;
                }
            }
        }
    }

    // Model per kategori fisik.
    let model_info: Vec<ModelInfo> = (0..nt)
        .filter(|t| TABLES[*t].fisik && per_tabel[*t].unit > 0)
        .map(|t| {
            let p = &model.param[t];
            let n: f64 = model.hitung[t].iter().map(|h| h.n).sum();
            let titik = kurva(&model.hitung[t], p);
            ModelInfo {
                tabel: TABLES[t].table.into(),
                label: TABLES[t].label.into(),
                n: n as i64,
                gabungan: model.gabungan[t],
                median_mulai: (p.median_mulai() * 10.0).round() / 10.0,
                lama_rr: (p.lama_rr() * 10.0).round() / 10.0,
                bentuk: (p.beta * 100.0).round() / 100.0,
                mae: (mae_pita(&titik) * 10.0).round() / 10.0,
                kurva: titik,
            }
        })
        .collect();

    // Kebutuhan per kelompok barang.
    let mut kelompok_butuh: Vec<KelompokButuh> = butuh
        .into_iter()
        .map(|((t, ki), b)| KelompokButuh {
            kategori: TABLES[t].label.into(),
            kelompok: kamus.kelompok[ki].clone(),
            unit_total: b.unit_total,
            unit: b.unit,
            biaya: b.biaya,
            harga: if b.harga_n > 0.0 {
                (b.harga_jml / b.harga_n).round()
            } else {
                0.0
            },
            sumber_harga: if TABLES[t].kebutuhan == JenisKebutuhan::Rehabilitasi {
                "porsi nilai bangunan".into()
            } else if b.median {
                "median perolehan 5 tahun".into()
            } else {
                "nilai perolehan dieskalasi".into()
            },
            idle_baik: b.idle_baik,
        })
        .collect();
    kelompok_butuh.sort_by(|a, b| {
        b.biaya[1]
            .total_cmp(&a.biaya[1])
            .then(a.kelompok.cmp(&b.kelompok))
    });
    kelompok_butuh.truncate(40);

    // Daftar aset prioritas nasional.
    calon_rb.sort_by(|a, b| b.0.total_cmp(&a.0).then(a.1.cmp(&b.1)));
    calon_rb.truncate(25);
    calon_risiko.sort_by(|a, b| {
        b.0.total_cmp(&a.0)
            .then(b.1.total_cmp(&a.1))
            .then(a.2.cmp(&b.2))
    });
    calon_risiko.truncate(25);
    let rinci = |fi: usize| {
        let f = &fakta[fi];
        let kelompok = kamus.kelompok[f.kelompok].as_str();
        let kode = kamus.satker_kode[f.satker].as_str();
        let t = turunkan(f, kelompok, kode, &model, k);
        aset_dari(f, &t, kelompok, &kamus.satker_nama[f.satker], kode)
    };
    let aset_rb: Vec<AsetRisiko> = calon_rb.iter().map(|c| rinci(c.1)).collect();
    let aset_risiko: Vec<AsetRisiko> = calon_risiko.iter().map(|c| rinci(c.2)).collect();

    // Mutu data.
    let rasio = median(&mut mutu.rasio).unwrap_or(0.0);
    let catatan_umur_sisa = if rasio <= 0.0 {
        "Belum cukup data untuk menguji satuan kolom umur_sisa.".to_string()
    } else if (0.6..=1.6).contains(&rasio) {
        format!(
            "Kolom umur_sisa konsisten dengan penyusutan garis lurus dalam satuan {}.",
            k.asumsi.satuan_umur_sisa
        )
    } else {
        format!(
            "Kolom umur_sisa tampak tidak selaras dengan penyusutan (rasio {rasio:.1}). \
             Periksa satuan kolom ini (tahun/semester/bulan) pada Pengaturan."
        )
    };
    let km = |label: &str, lingkup: &str, terisi: i64, total: i64| KolomMutu {
        label: label.into(),
        lingkup: lingkup.into(),
        terisi,
        total,
    };
    let an = |kunci: &str, judul: &str, x: (i64, f64), uraian: &str| Anomali {
        kunci: kunci.into(),
        judul: judul.into(),
        jumlah: x.0,
        nilai: x.1.round(),
        uraian: uraian.into(),
    };
    let tanah_tanpa = nasional.tanah_n - nasional.sertif_ya;
    let kualitas = Kualitas {
        kolom: vec![
            km("Kondisi fisik", "aset fisik", mutu.fisik_isi, mutu.fisik),
            km("Tanggal perolehan sah", "seluruh aset", mutu.tgl_sah, mutu.semua),
            km("Nilai di atas Rp1", "seluruh aset", mutu.nilai_wajar, mutu.semua),
            km("Satuan kerja", "seluruh aset", mutu.satker_isi, mutu.semua),
            km("Sisa umur ekonomis", "aset yang disusutkan", mutu.susut_isi, mutu.susut),
            km("Penetapan Status Penggunaan", "selain KDP", nasional.psp_ya, nasional.psp_n),
            km("Bukti kepemilikan", "tanah", nasional.sertif_ya, nasional.tanah_n),
            km("Koordinat lokasi", "tanah dan bangunan", mutu.lokasi_gps, mutu.lokasi_n),
            km("Foto aset", "seluruh aset", mutu.foto, mutu.semua),
        ],
        anomali: vec![
            an("rb_aktif", "Rusak berat belum dihentikan penggunaannya", mutu.rb_aktif,
               "Aset rusak berat yang belum ditandai dihentikan dari penggunaan; calon usulan penghapusan."),
            an("tanah_tanpa_bukti", "Tanah tanpa bukti kepemilikan tercatat", (tanah_tanpa, 0.0),
               "Bidang tanah yang nomor/jenis dokumen kepemilikannya kosong."),
            an("sengketa", "Aset dalam perkara hukum", mutu.sengketa,
               "Aset yang memuat nomor perkara hukum."),
            an("kdp_mangkrak", "KDP lebih dari 3 tahun", mutu.kdp_mangkrak,
               "Konstruksi dalam pengerjaan yang belum direklasifikasi lebih dari tiga tahun."),
            an("rumah_kosong", "Rumah negara tanpa penghuni tercatat", mutu.rumah_kosong,
               "Rumah negara yang nama penghuninya kosong."),
            an("nilai_kecil", "Nilai aset Rp1 atau kosong", mutu.nilai_kecil,
               "Aset yang belum dinilai wajar sehingga nilai portofolio terlalu rendah."),
            an("tgl_janggal", "Tanggal perolehan kosong atau tidak sah", (mutu.tgl_janggal, 0.0),
               "Umur aset tidak dapat dihitung; proyeksi memakai rata-rata umur kategori."),
            an("baik_tapi_rusak", "Kondisi baik tetapi ditandai rusak", mutu.baik_tapi_rusak,
               "Uraian kondisi dan penanda barang rusak tidak selaras."),
            an("sisa_janggal", "Sisa umur nol tetapi masih bernilai buku", mutu.sisa_janggal,
               "Sisa umur ditaksir ulang dari porsi nilai buku."),
            an("kunci_ganda", "Kode register/kunci aset ganda", (k.kunci_ganda, 0.0),
               "Aset dengan kunci yang sama muncul lebih dari sekali."),
        ],
        per_kategori: kategori
            .iter()
            .map(|c| (c.label.clone(), c.r.pct_lengkap(), c.r.unit))
            .collect(),
        rasio_umur_sisa: (rasio * 100.0).round() / 100.0,
        catatan_umur_sisa,
    };

    let skor = Skor::dari(&nasional);
    nasional.bulatkan();
    let meta = Meta {
        is_demo: k.is_demo,
        kosong: nasional.unit == 0,
        dibuat: k.dibuat.clone(),
        tanggal_data: k.tanggal_data.clone(),
        tahun0: k.tahun0,
        total_satker: simpan.len() as i64,
        total_provinsi: provinsi.len() as i64,
        tabel_tersinkron: k.tabel_tersinkron,
        dikecualikan,
        asumsi: Some(k.asumsi.clone()),
    };

    let mut analisis = Analisis {
        meta,
        nasional,
        skor,
        kategori,
        provinsi,
        tren: tren_total,
        umur: pita,
        model: model_info,
        prakiraan: prakiraan_semua,
        buku_th: buku_th
            .iter()
            .enumerate()
            .map(|(i, (total, alat))| BukuTahun {
                tahun: if i == 0 { 0 } else { k.tahun0 + i as i32 - 1 },
                total: total.round(),
                peralatan: alat.round(),
            })
            .collect(),
        kapasitas,
        kelompok_butuh,
        satker_skor,
        satker_butuh,
        satker_rb,
        satker_rku,
        aset_rb,
        aset_risiko,
        kualitas,
        sorotan: vec![],
        rekomendasi: vec![],
        delta: None,
    };
    let (sorotan, rekomendasi) = super::rekomendasi::susun(&analisis, &satker);
    analisis.sorotan = sorotan;
    analisis.rekomendasi = rekomendasi;

    Hasil {
        versi: 0,
        analisis,
        satker,
        simpan: simpan.into_iter().map(|s| (s.kode.clone(), s)).collect(),
        model,
        konteks: k.clone(),
    }
}

#[cfg(test)]
pub mod uji {
    use super::*;

    pub fn konteks() -> Konteks {
        Konteks {
            tahun0: 2026,
            bulan0: 6,
            asumsi: Asumsi::default(),
            tanggal_data: "2026-06-15".into(),
            dibuat: "2026-06-15 10:00 WIB".into(),
            tabel_tersinkron: 15,
            ..Default::default()
        }
    }

    pub fn tabel(nama: &str) -> usize {
        crate::catalog::indeks(nama).unwrap()
    }

    /// Bangun fakta dari data contoh (melalui jalur normalisasi yang sama dengan sinkronisasi).
    pub fn fakta_contoh(tabel_dipakai: &[&str]) -> (Vec<Fakta>, Kamus) {
        let mut kamus = Kamus::default();
        let mut out = Vec::new();
        let mut id = 0;
        for nama in tabel_dipakai {
            let ti = tabel(nama);
            for (i, row) in crate::demo::baris_tabel(nama, 2026, 6).into_iter().enumerate() {
                let row = crate::fakta::normalkan(row);
                let c = crate::fakta::ekstrak(&row, i);
                id += 1;
                out.push(Fakta {
                    id,
                    tabel: ti,
                    satker: kamus.satker(&c.kode_satker, &c.nama_satker),
                    prov: kamus.prov(&c.prov),
                    kelompok: kamus.kelompok(&c.kelompok),
                    kondisi: c.kondisi,
                    nilai: c.nilai,
                    buku: c.buku,
                    tahun: c.tahun as i32,
                    bulan: c.bulan as u32,
                    umur_sisa: c.umur_sisa,
                    flags: c.flags,
                });
            }
        }
        (out, kamus)
    }

    fn satu(tabel_nama: &str, kondisi: i64, tahun: i32, sisa: Option<f64>, nilai: f64) -> (Vec<Fakta>, Kamus) {
        let mut kamus = Kamus::default();
        let f = Fakta {
            id: 1,
            tabel: tabel(tabel_nama),
            satker: kamus.satker("S1", "KEJAKSAAN NEGERI UJI"),
            prov: kamus.prov("JAWA BARAT"),
            kelompok: kamus.kelompok("Kelompok Uji"),
            kondisi,
            nilai,
            buku: 0.0,
            tahun,
            bulan: 6,
            umur_sisa: sisa,
            flags: 0,
        };
        (vec![f], kamus)
    }

    #[test]
    fn rusak_berat_menjadi_tunggakan_semua_skenario() {
        let (f, kamus) = satu("SIMAN2_M_ASET_KHUSUS_TIK", K_RB, 2016, Some(0.0), 10_000_000.0);
        let h = hitung(&f, &kamus, &konteks());
        let r = &h.analisis.nasional;
        assert_eq!(r.rb, 1);
        assert_eq!(r.habis, 1);
        assert_eq!(r.ganti_kini, [1.0, 1.0, 1.0]);
        assert_eq!(r.rb_5(), 0.0, "yang sudah rusak berat tidak diproyeksikan lagi");
        assert!(r.ganti_th.iter().all(|s| s.iter().all(|x| *x == 0.0)));
        // Biaya memakai nilai perolehan dieskalasi (kelompok tanpa harga acuan).
        assert!(r.biaya_kini[0] > 10_000_000.0);
    }

    #[test]
    fn baik_habis_masa_manfaat_hanya_skenario_maksimal() {
        let (f, kamus) = satu("SIMAN2_M_ASET_KHUSUS_TIK", K_BAIK, 2018, Some(0.0), 10_000_000.0);
        let r = hitung(&f, &kamus, &konteks()).analisis.nasional;
        assert_eq!(r.habis, 1);
        assert_eq!(r.ganti_kini, [0.0, 0.0, 1.0]);
        // Skenario dasar & moderat tetap memuat risiko rusak berat ke depan.
        assert!(r.ganti_total(0) > 0.0 && r.ganti_total(0) < 1.0);
        assert!(r.ganti_total(1) >= r.ganti_total(0));
        assert!((r.ganti_total(2) - 1.0).abs() < 1e-9);
    }

    #[test]
    fn rusak_ringan_habis_masa_manfaat_masuk_moderat() {
        let (f, kamus) = satu("SIMAN2_M_ASET_KHUSUS_TIK", K_RR, 2018, Some(0.0), 10_000_000.0);
        let r = hitung(&f, &kamus, &konteks()).analisis.nasional;
        assert_eq!(r.ganti_kini, [0.0, 1.0, 1.0]);
        assert!(r.rb_5() > 0.3, "rusak ringan berpeluang besar menjadi rusak berat");
    }

    #[test]
    fn habis_masa_manfaat_dipetakan_ke_tahun_yang_benar() {
        // Data per pertengahan Juni 2026; sisa 0,3 th → habis 2026; sisa 2 th → 2028.
        let (mut f, kamus) = satu("SIMAN2_M_ASET_NON_TIK", K_BAIK, 2022, Some(0.3), 5_000_000.0);
        let mut g = f[0].clone();
        g.id = 2;
        g.umur_sisa = Some(2.0);
        let mut j = f[0].clone();
        j.id = 3;
        j.umur_sisa = Some(9.0);
        f.push(g);
        f.push(j);
        let r = hitung(&f, &kamus, &konteks()).analisis.nasional;
        assert_eq!(r.habis, 0);
        assert_eq!(r.habis_th[0], 1.0, "habis pada sisa tahun berjalan");
        assert_eq!(r.habis_th[2], 1.0, "habis pada 2028");
        assert_eq!(r.habis_5(), 2.0, "yang habis setelah 2031 tidak dihitung");
        assert_eq!(r.susut_unit, 3);
        // Skenario maksimal mengikuti jadwal habis masa manfaat.
        assert!(r.ganti_th[2][0] > 0.99 && r.ganti_th[2][2] > 0.9);
    }

    #[test]
    fn tanah_tidak_disusutkan_dan_tidak_menimbulkan_kebutuhan() {
        let (f, kamus) = satu("SIMAN2_M_ASET_TANAH", K_BAIK, 1995, None, 9e9);
        let r = hitung(&f, &kamus, &konteks()).analisis.nasional;
        assert_eq!(r.susut_unit, 0);
        assert_eq!(r.fisik, 0);
        assert_eq!(r.ganti_total(2), 0.0);
        assert_eq!(r.tanah_n, 1);
        assert_eq!(r.sertif_ya, 0);
    }

    #[test]
    fn baris_dihapus_dikecualikan() {
        let (mut f, kamus) = satu("SIMAN2_M_ASET_NON_TIK", K_BAIK, 2022, Some(1.0), 5e6);
        f[0].flags = F_DIHAPUS;
        let h = hitung(&f, &kamus, &konteks());
        assert_eq!(h.analisis.nasional.unit, 0);
        assert_eq!(h.analisis.meta.dikecualikan, 1);
        assert!(h.analisis.meta.kosong);
    }

    #[test]
    fn satuan_umur_sisa_semester_dikonversi() {
        let (f, kamus) = satu("SIMAN2_M_ASET_NON_TIK", K_BAIK, 2022, Some(4.0), 5e6);
        let mut k = konteks();
        k.asumsi.satuan_umur_sisa = "semester".into();
        let r = hitung(&f, &kamus, &k).analisis.nasional;
        // 4 semester = 2 tahun → habis 2028.
        assert_eq!(r.habis_th[2], 1.0);
    }

    #[test]
    fn umur_sisa_nol_tetapi_bernilai_buku_ditaksir_dari_nilai_buku() {
        let (mut f, kamus) = satu("SIMAN2_M_ASET_NON_TIK", K_BAIK, 2024, Some(0.0), 10e6);
        f[0].buku = 6e6;
        let h = hitung(&f, &kamus, &konteks());
        let r = &h.analisis.nasional;
        assert_eq!(r.habis, 0, "nilai buku 60% berarti belum habis masa manfaat");
        let an = h
            .analisis
            .kualitas
            .anomali
            .iter()
            .find(|a| a.kunci == "sisa_janggal")
            .unwrap();
        assert_eq!(an.jumlah, 1);
    }

    #[test]
    fn agregat_konsisten_di_semua_tingkat() {
        let (f, kamus) = fakta_contoh(&[
            "SIMAN2_M_ASET_ANGKUTAN_BERMOTOR",
            "SIMAN2_M_ASET_KHUSUS_TIK",
            "SIMAN2_M_ASET_TANAH",
            "SIMAN2_M_ASET_GEDUNG_BANGUNAN",
        ]);
        let h = hitung(&f, &kamus, &konteks());
        let a = &h.analisis;
        let nas = &a.nasional;
        assert_eq!(nas.unit as usize, f.len());
        // Jumlah kategori, provinsi, dan satuan kerja sama dengan nasional.
        assert_eq!(a.kategori.iter().map(|c| c.r.unit).sum::<i64>(), nas.unit);
        assert_eq!(a.provinsi.iter().map(|p| p.r.unit).sum::<i64>(), nas.unit);
        assert_eq!(h.satker.iter().map(|s| s.unit).sum::<i64>(), nas.unit);
        assert_eq!(h.satker.iter().map(|s| s.rb).sum::<i64>(), nas.rb);
        let nilai_kat: f64 = a.kategori.iter().map(|c| c.r.nilai).sum();
        assert!((nilai_kat - nas.nilai).abs() / nas.nilai < 1e-9);
        let rb5_sat: f64 = h.satker.iter().map(|s| s.rb5).sum();
        assert!((rb5_sat - nas.rb_5()).abs() < 1.0, "{rb5_sat} vs {}", nas.rb_5());
        assert_eq!(a.meta.total_satker as usize, h.satker.len());
        assert_eq!(a.meta.total_provinsi, 38);
        // Skenario makin luas makin besar kebutuhannya.
        assert!(nas.ganti_total(0) <= nas.ganti_total(1) + 1e-6);
        assert!(nas.ganti_total(1) <= nas.ganti_total(2) + 1e-6);
        assert!(nas.biaya_total(0) <= nas.biaya_total(2));
        // Peringkat unik 1..n.
        let mut p: Vec<i64> = h.simpan.values().map(|s| s.peringkat).collect();
        p.sort();
        assert_eq!(p, (1..=h.simpan.len() as i64).collect::<Vec<_>>());
    }

    #[test]
    fn model_data_contoh_tertaksir_dan_masuk_akal() {
        let (f, kamus) = fakta_contoh(&[
            "SIMAN2_M_ASET_ANGKUTAN_BERMOTOR",
            "SIMAN2_M_ASET_KHUSUS_TIK",
            "SIMAN2_M_ASET_ALAT_BESAR",
        ]);
        let h = hitung(&f, &kamus, &konteks());
        let tik = h.analisis.model.iter().find(|m| m.label == "Peralatan TIK").unwrap();
        let angkutan = h
            .analisis
            .model
            .iter()
            .find(|m| m.label == "Angkutan Bermotor")
            .unwrap();
        assert!(!tik.gabungan && !angkutan.gabungan);
        assert!(tik.median_mulai < angkutan.median_mulai, "TIK lebih cepat rusak");
        assert!(tik.mae < 6.0 && angkutan.mae < 6.0, "mae {} {}", tik.mae, angkutan.mae);
        assert!(tik.bentuk > 1.0, "laju kerusakan naik seiring umur");
        let besar = h.analisis.model.iter().find(|m| m.label == "Alat Besar").unwrap();
        assert!(besar.median_mulai > 0.0);
        // Masa manfaat efektif dari data mendekati masa manfaat sebenarnya.
        let kat = h.analisis.kategori.iter().find(|c| c.label == "Peralatan TIK").unwrap();
        assert!((kat.masa_manfaat - 4.0).abs() < 0.6, "masa {}", kat.masa_manfaat);
        // Uji satuan umur_sisa: data contoh konsisten dalam tahun.
        let q = &h.analisis.kualitas;
        assert!((0.7..=1.4).contains(&q.rasio_umur_sisa), "rasio {}", q.rasio_umur_sisa);
    }

    #[test]
    fn satker_rapuh_mendapat_rku_tinggi() {
        let (f, kamus) = fakta_contoh(&[
            "SIMAN2_M_ASET_KHUSUS_TIK",
            "SIMAN2_M_ASET_NON_TIK",
            "SIMAN2_M_ASET_ANGKUTAN_BERMOTOR",
        ]);
        let h = hitung(&f, &kamus, &konteks());
        let contoh = crate::demo::satker_contoh();
        // Bandingkan satuan kerja contoh paling rapuh dan paling terawat.
        let rapuh = contoh
            .iter()
            .filter(|s| s.skala == 1.0)
            .max_by(|a, b| a.kerapuhan.total_cmp(&b.kerapuhan))
            .unwrap();
        let terawat = contoh
            .iter()
            .filter(|s| s.skala == 1.0)
            .min_by(|a, b| a.kerapuhan.total_cmp(&b.kerapuhan))
            .unwrap();
        let rku = |kode: &str| h.simpan[kode].r.rku();
        assert!(rku(&rapuh.kode) > 1.15, "rapuh {}", rku(&rapuh.kode));
        assert!(rku(&terawat.kode) < 0.9, "terawat {}", rku(&terawat.kode));
        assert!(h.simpan[&rapuh.kode].skor.total < h.simpan[&terawat.kode].skor.total);
    }

    #[test]
    fn prakiraan_dan_kapasitas_terisi() {
        let (f, kamus) = fakta_contoh(&["SIMAN2_M_ASET_NON_TIK"]);
        let h = hitung(&f, &kamus, &konteks());
        let a = &h.analisis;
        let total = &a.prakiraan[0];
        assert_eq!(total.kunci, "TOTAL");
        // Titik pertama = taksiran setahun penuh tahun berjalan, lalu lima tahun berikutnya.
        assert_eq!(total.unit.ramal.len(), NH);
        assert_eq!(total.unit.ramal[0].tahun, 2026);
        assert_eq!(total.unit.ramal[NH - 1].tahun, 2031);
        assert!(total.unit.riwayat.iter().all(|p| p.0 < 2026), "tahun berjalan dipisah");
        assert!(total.berjalan.0 > 0.0);
        assert!(a.kapasitas.unit_per_tahun > 0.0);
        assert_eq!((a.kapasitas.tahun_awal, a.kapasitas.tahun_akhir), (2021, 2025));
        // Nilai buku portofolio yang ada turun bila tidak ada pengadaan baru.
        assert_eq!(a.buku_th.len(), NH + 1);
        assert!(a.buku_th[NH].total < a.buku_th[0].total);
        assert!(!a.kelompok_butuh.is_empty());
        assert!(a.kelompok_butuh[0].harga > 0.0);
    }

    /// Cetak ringkasan data contoh penuh untuk pemeriksaan manual:
    /// `cargo test --features ssr --lib cetak_ringkasan -- --ignored --nocapture`.
    #[test]
    #[ignore]
    fn cetak_ringkasan_contoh() {
        let semua: Vec<&str> = TABLES.iter().map(|t| t.table).collect();
        let mulai = std::time::Instant::now();
        let (f, kamus) = fakta_contoh(&semua);
        let muat = mulai.elapsed();
        let mulai = std::time::Instant::now();
        let h = hitung(&f, &kamus, &konteks());
        println!("fakta {} | muat {:?} | hitung {:?}", f.len(), muat, mulai.elapsed());
        let a = &h.analisis;
        let r = &a.nasional;
        println!("unit {} nilai {:.3e} buku {:.3e} satker {} prov {}", r.unit, r.nilai, r.buku, a.meta.total_satker, a.meta.total_provinsi);
        println!("baik {:.1}% rr {} rb {} tak {} | habis {} ({:.1}%) | idle {} hilang {}", r.pct_baik(), r.rr, r.rb, r.tak, r.habis, r.pct_habis(), r.idle, r.hilang);
        println!("habis_th {:?}", r.habis_th);
        println!("rb_th {:?}", r.rb_th);
        for s in 0..3 {
            println!("skenario {s}: kini {:.0} unit Rp{:.3e} | 5th {:.0} unit Rp{:.3e}", r.ganti_kini[s], r.biaya_kini[s], r.ganti_total(s), r.biaya_total(s));
        }
        println!("kapasitas {:.0} unit/th Rp{:.3e}/th | skor {:.1} | psp {:.1}% sertifikat {:.1}% lengkap {:.1}%", a.kapasitas.unit_per_tahun, a.kapasitas.nilai_per_tahun, a.skor.total, r.pct_psp(), r.pct_sertifikat(), r.pct_lengkap());
        for c in &a.kategori {
            println!("  {:<28} unit {:>6} nilai {:>10.3e} baik {:>5.1}% rb {:>5} habis {:>5.1}% rb5 {:>7.1} masa {:>4.1} butuh1 {:>7.0}", c.label, c.r.unit, c.r.nilai, c.r.pct_baik(), c.r.rb, c.r.pct_habis(), c.r.rb_5(), c.masa_manfaat, c.r.ganti_total(1));
        }
        for m in &a.model {
            println!("  model {:<22} n {:>6} gab {} median {:>5.1} lamaRR {:>4.1} beta {:.2} mae {:.1}", m.label, m.n, m.gabungan, m.median_mulai, m.lama_rr, m.bentuk, m.mae);
        }
        for p in a.prakiraan.iter().take(3) {
            println!("  prakiraan {:<20} {} galat {:.1} | {:?}", p.label, p.unit.metode, p.unit.galat, p.unit.ramal.iter().map(|t| t.nilai.round()).collect::<Vec<_>>());
        }
        let mut skor: Vec<f64> = h.satker.iter().map(|s| s.skor).collect();
        skor.sort_by(|x, y| x.total_cmp(y));
        println!("skor satker min {:.1} med {:.1} max {:.1} | kritis {} perhatian {}", skor[0], skor[skor.len() / 2], skor[skor.len() - 1], skor.iter().filter(|x| **x < 65.0).count(), skor.iter().filter(|x| **x >= 65.0 && **x < 80.0).count());
        println!("rasio umur_sisa {} | {}", a.kualitas.rasio_umur_sisa, a.kualitas.catatan_umur_sisa);
        for s in &a.sorotan {
            println!("  [{}] {}: {}", s.nada, s.judul, s.isi);
        }
        for x in &a.rekomendasi {
            println!("  (P{}) {} — {}", x.prioritas, x.judul, x.temuan);
        }
        println!("json analisis {} KB", serde_json::to_string(a).unwrap().len() / 1024);
        println!("json satker {} KB", serde_json::to_string(&h.satker).unwrap().len() / 1024);
    }
}
