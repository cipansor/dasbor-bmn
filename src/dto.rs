//! Model data hasil analitik yang dikirim ke antarmuka dan laporan.
//!
//! Seluruh struktur di sini murni data (serde) sehingga dapat dipakai baik di
//! server (SSR, laporan PDF) maupun di peramban (hidrasi WASM).

use serde::{Deserialize, Serialize};

/// Cakrawala proyeksi: lima tahun anggaran setelah tahun berjalan.
pub const HORIZON: usize = 5;
/// Panjang larik proyeksi. Indeks 0 = sisa tahun berjalan, 1..=5 = tahun berikutnya.
pub const NH: usize = HORIZON + 1;

/// Skenario kebutuhan penggantian/rehabilitasi.
pub const SKENARIO: [(&str, &str); 3] = [
    (
        "Dasar",
        "Hanya aset yang rusak berat (kini maupun diproyeksikan).",
    ),
    (
        "Moderat",
        "Rusak berat, ditambah aset rusak ringan yang sudah habis masa manfaat.",
    ),
    (
        "Maksimal",
        "Rusak berat, ditambah seluruh aset yang habis masa manfaat (peremajaan penuh).",
    ),
];

/// Ringkasan satu kelompok aset (nasional, kategori, provinsi, atau satuan kerja).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Ringkas {
    /// Jumlah unit aset aktif.
    pub unit: i64,
    /// Nilai aset bruto (Rp).
    pub nilai: f64,
    /// Nilai buku (Rp).
    pub buku: f64,
    /// Unit aset fisik, yaitu kategori yang kondisi fisiknya relevan.
    pub fisik: i64,
    pub baik: i64,
    pub rr: i64,
    pub rb: i64,
    /// Kondisi tidak diisi/tidak dikenali.
    pub tak: i64,
    pub nilai_rb: f64,
    pub idle: i64,
    pub nilai_idle: f64,
    pub hilang: i64,
    pub nilai_hilang: f64,
    /// Unit yang disusutkan dan sisa masa manfaatnya dapat ditentukan.
    pub susut_unit: i64,
    /// Unit yang sudah habis masa manfaat.
    pub habis: i64,
    pub nilai_habis: f64,
    /// Unit yang habis masa manfaat per tahun proyeksi.
    pub habis_th: [f64; NH],
    /// Ekspektasi unit yang menjadi rusak berat per tahun proyeksi.
    pub rb_th: [f64; NH],
    /// Tunggakan kebutuhan saat ini per skenario (unit).
    pub ganti_kini: [f64; 3],
    /// Tunggakan kebutuhan saat ini per skenario (Rp).
    pub biaya_kini: [f64; 3],
    /// Kebutuhan per tahun proyeksi per skenario (unit).
    pub ganti_th: [[f64; NH]; 3],
    /// Kebutuhan per tahun proyeksi per skenario (Rp).
    pub biaya_th: [[f64; NH]; 3],
    /// Jumlah umur (tahun) dan banyaknya aset berumur diketahui.
    pub umur_jml: f64,
    pub umur_n: i64,
    /// Unit layak dipelihara (baik/rusak ringan, tidak idle).
    pub layak_pelihara: i64,
    /// Aset dengan Penetapan Status Penggunaan dari yang wajib PSP.
    pub psp_ya: i64,
    pub psp_n: i64,
    /// Bidang tanah dengan bukti kepemilikan/sertifikat.
    pub sertif_ya: i64,
    pub tanah_n: i64,
    /// Kelengkapan isian kunci (kondisi, tanggal perolehan, nilai wajar).
    pub lengkap_ya: i64,
    pub lengkap_n: i64,
    /// Aset tidak baik yang teramati dan yang diharapkan menurut umur (untuk RKU).
    pub rusak_obs: f64,
    pub rusak_harap: f64,
}

fn bagi(a: f64, b: f64) -> f64 {
    if b > 0.0 {
        a / b
    } else {
        0.0
    }
}

impl Ringkas {
    /// Unit fisik yang kondisinya diketahui.
    pub fn kondisi_diketahui(&self) -> i64 {
        self.baik + self.rr + self.rb
    }
    pub fn pct_baik(&self) -> f64 {
        100.0 * bagi(self.baik as f64, self.kondisi_diketahui() as f64)
    }
    pub fn pct_rr(&self) -> f64 {
        100.0 * bagi(self.rr as f64, self.kondisi_diketahui() as f64)
    }
    pub fn pct_rb(&self) -> f64 {
        100.0 * bagi(self.rb as f64, self.kondisi_diketahui() as f64)
    }
    pub fn pct_habis(&self) -> f64 {
        100.0 * bagi(self.habis as f64, self.susut_unit as f64)
    }
    pub fn pct_idle(&self) -> f64 {
        100.0 * bagi(self.idle as f64, self.unit as f64)
    }
    pub fn pct_hilang(&self) -> f64 {
        100.0 * bagi(self.hilang as f64, self.unit as f64)
    }
    pub fn pct_psp(&self) -> f64 {
        100.0 * bagi(self.psp_ya as f64, self.psp_n as f64)
    }
    pub fn pct_sertifikat(&self) -> f64 {
        100.0 * bagi(self.sertif_ya as f64, self.tanah_n as f64)
    }
    pub fn pct_lengkap(&self) -> f64 {
        100.0 * bagi(self.lengkap_ya as f64, self.lengkap_n as f64)
    }
    pub fn pct_susut(&self) -> f64 {
        100.0 * bagi(self.nilai - self.buku, self.nilai)
    }
    pub fn umur_rata(&self) -> f64 {
        bagi(self.umur_jml, self.umur_n as f64)
    }
    /// Total habis masa manfaat selama cakrawala (tanpa yang sudah habis).
    pub fn habis_5(&self) -> f64 {
        self.habis_th.iter().sum()
    }
    /// Total ekspektasi rusak berat baru selama cakrawala.
    pub fn rb_5(&self) -> f64 {
        self.rb_th.iter().sum()
    }
    /// Kebutuhan unit selama cakrawala termasuk tunggakan.
    pub fn ganti_total(&self, s: usize) -> f64 {
        self.ganti_kini[s] + self.ganti_th[s].iter().sum::<f64>()
    }
    /// Kebutuhan biaya selama cakrawala termasuk tunggakan.
    pub fn biaya_total(&self, s: usize) -> f64 {
        self.biaya_kini[s] + self.biaya_th[s].iter().sum::<f64>()
    }
    /// Rasio Kerusakan terstandar Umur: teramati dibagi harapan menurut umur.
    /// Nilai di atas 1 berarti kondisi lebih buruk daripada wajarnya umur aset.
    pub fn rku(&self) -> f64 {
        const SUSUT: f64 = 8.0;
        (self.rusak_obs + SUSUT) / (self.rusak_harap + SUSUT)
    }

    /// Tambahkan ringkasan lain ke ringkasan ini.
    pub fn gabung(&mut self, o: &Ringkas) {
        self.unit += o.unit;
        self.nilai += o.nilai;
        self.buku += o.buku;
        self.fisik += o.fisik;
        self.baik += o.baik;
        self.rr += o.rr;
        self.rb += o.rb;
        self.tak += o.tak;
        self.nilai_rb += o.nilai_rb;
        self.idle += o.idle;
        self.nilai_idle += o.nilai_idle;
        self.hilang += o.hilang;
        self.nilai_hilang += o.nilai_hilang;
        self.susut_unit += o.susut_unit;
        self.habis += o.habis;
        self.nilai_habis += o.nilai_habis;
        for k in 0..NH {
            self.habis_th[k] += o.habis_th[k];
            self.rb_th[k] += o.rb_th[k];
        }
        for s in 0..3 {
            self.ganti_kini[s] += o.ganti_kini[s];
            self.biaya_kini[s] += o.biaya_kini[s];
            for k in 0..NH {
                self.ganti_th[s][k] += o.ganti_th[s][k];
                self.biaya_th[s][k] += o.biaya_th[s][k];
            }
        }
        self.umur_jml += o.umur_jml;
        self.umur_n += o.umur_n;
        self.layak_pelihara += o.layak_pelihara;
        self.psp_ya += o.psp_ya;
        self.psp_n += o.psp_n;
        self.sertif_ya += o.sertif_ya;
        self.tanah_n += o.tanah_n;
        self.lengkap_ya += o.lengkap_ya;
        self.lengkap_n += o.lengkap_n;
        self.rusak_obs += o.rusak_obs;
        self.rusak_harap += o.rusak_harap;
    }

    /// Bulatkan nilai pecahan agar muatan JSON tetap ringkas.
    pub fn bulatkan(&mut self) {
        let b = |x: &mut f64| *x = (*x * 100.0).round() / 100.0;
        let u = |x: &mut f64| *x = x.round();
        u(&mut self.nilai);
        u(&mut self.buku);
        u(&mut self.nilai_rb);
        u(&mut self.nilai_idle);
        u(&mut self.nilai_hilang);
        u(&mut self.nilai_habis);
        b(&mut self.umur_jml);
        b(&mut self.rusak_obs);
        b(&mut self.rusak_harap);
        for k in 0..NH {
            b(&mut self.habis_th[k]);
            b(&mut self.rb_th[k]);
        }
        for s in 0..3 {
            b(&mut self.ganti_kini[s]);
            u(&mut self.biaya_kini[s]);
            for k in 0..NH {
                b(&mut self.ganti_th[s][k]);
                u(&mut self.biaya_th[s][k]);
            }
        }
    }
}

/// Skor kesehatan pengelolaan BMN (0–100) beserta komponennya.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Skor {
    pub total: f64,
    pub kondisi: f64,
    pub umur: f64,
    pub pemanfaatan: f64,
    pub pengamanan: f64,
    pub tertib: f64,
}

/// Bobot komponen skor kesehatan (jumlah = 1).
pub const BOBOT_SKOR: [(&str, f64, &str); 5] = [
    (
        "Kondisi fisik",
        0.35,
        "Porsi aset fisik berkondisi baik (rusak ringan dihitung setengah).",
    ),
    (
        "Masa manfaat",
        0.20,
        "Porsi aset yang disusutkan dan belum habis masa manfaat.",
    ),
    (
        "Pemanfaatan",
        0.15,
        "Makin kecil porsi aset idle makin tinggi (10% idle = nol).",
    ),
    (
        "Pengamanan",
        0.15,
        "Makin kecil porsi barang hilang makin tinggi (2% hilang = nol).",
    ),
    (
        "Tertib administrasi",
        0.15,
        "Rata-rata cakupan PSP, sertifikat tanah, dan kelengkapan data.",
    ),
];

impl Skor {
    /// Hitung skor dari ringkasan satu kelompok aset.
    pub fn dari(r: &Ringkas) -> Skor {
        let diketahui = r.kondisi_diketahui() as f64;
        let kondisi = if diketahui > 0.0 {
            100.0 * (r.baik as f64 + 0.5 * r.rr as f64) / diketahui
        } else {
            100.0
        };
        let umur = if r.susut_unit > 0 {
            100.0 - r.pct_habis()
        } else {
            100.0
        };
        let pemanfaatan = 100.0 - (r.pct_idle() / 10.0).min(1.0) * 100.0;
        let pengamanan = 100.0 - (r.pct_hilang() / 2.0).min(1.0) * 100.0;
        let mut bagian = Vec::new();
        if r.psp_n > 0 {
            bagian.push(r.pct_psp());
        }
        if r.tanah_n > 0 {
            bagian.push(r.pct_sertifikat());
        }
        if r.lengkap_n > 0 {
            bagian.push(r.pct_lengkap());
        }
        let tertib = if bagian.is_empty() {
            100.0
        } else {
            bagian.iter().sum::<f64>() / bagian.len() as f64
        };
        let total = BOBOT_SKOR[0].1 * kondisi
            + BOBOT_SKOR[1].1 * umur
            + BOBOT_SKOR[2].1 * pemanfaatan
            + BOBOT_SKOR[3].1 * pengamanan
            + BOBOT_SKOR[4].1 * tertib;
        Skor {
            total,
            kondisi,
            umur,
            pemanfaatan,
            pengamanan,
            tertib,
        }
    }

    /// Predikat skor untuk pembaca cepat.
    pub fn predikat(total: f64) -> (&'static str, &'static str) {
        if total >= 80.0 {
            ("Baik", "green")
        } else if total >= 65.0 {
            ("Perlu Perhatian", "amber")
        } else {
            ("Kritis", "red")
        }
    }
}

/// Asumsi yang dipakai mesin analitik; ditampilkan apa adanya kepada pembaca.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Asumsi {
    /// Eskalasi harga per tahun (pecahan, mis. 0,03).
    pub eskalasi: f64,
    /// Porsi nilai bangunan yang dipakai sebagai biaya rehabilitasi berat.
    pub faktor_rehab: f64,
    /// Satuan kolom `umur_sisa` pada sumber: "tahun", "semester", atau "bulan".
    pub satuan_umur_sisa: String,
}

impl Default for Asumsi {
    fn default() -> Self {
        Asumsi {
            eskalasi: 0.03,
            faktor_rehab: 0.65,
            satuan_umur_sisa: "tahun".to_string(),
        }
    }
}

/// Keterangan umum hasil analitik.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Meta {
    pub is_demo: bool,
    /// Belum ada satu pun tabel yang tersinkron.
    pub kosong: bool,
    /// Waktu hasil analitik dibuat.
    pub dibuat: String,
    /// Tanggal posisi data (sinkronisasi terakhir).
    pub tanggal_data: String,
    /// Tahun berjalan (acuan proyeksi).
    pub tahun0: i32,
    pub total_satker: i64,
    pub total_provinsi: i64,
    /// Jumlah kategori yang sudah tersinkron dari 15.
    pub tabel_tersinkron: i64,
    /// Baris berstatus dihapus yang dikecualikan dari analisis.
    pub dikecualikan: i64,
    pub asumsi: Option<Asumsi>,
}

impl Meta {
    /// Label tahun untuk indeks proyeksi `k` (0 = tahun berjalan).
    pub fn tahun(&self, k: usize) -> i32 {
        self.tahun0 + k as i32
    }
    /// Label kolom proyeksi, mis. "2026*" untuk sisa tahun berjalan.
    pub fn label_tahun(&self, k: usize) -> String {
        if k == 0 {
            format!("{}*", self.tahun0)
        } else {
            self.tahun(k).to_string()
        }
    }
    pub fn tahun_akhir(&self) -> i32 {
        self.tahun0 + HORIZON as i32
    }
}

/// Ringkasan satu kategori aset (satu tabel SIMAN).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct KategoriBaris {
    pub tabel: String,
    pub label: String,
    pub icon: String,
    /// Kondisi fisik relevan untuk kategori ini.
    pub fisik: bool,
    pub disusutkan: bool,
    /// "Penggantian", "Rehabilitasi", atau "-".
    pub kebutuhan: String,
    /// Masa manfaat tipikal menurut data (median umur + sisa), tahun.
    pub masa_manfaat: f64,
    pub r: Ringkas,
}

/// Ringkasan satu provinsi.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ProvBaris {
    pub nama: String,
    pub satker: i64,
    pub skor: f64,
    pub r: Ringkas,
}

/// Baris ringkas satu satuan kerja untuk tabel peringkat.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct SatkerBaris {
    pub kode: String,
    pub nama: String,
    pub prov: String,
    pub unit: i64,
    pub nilai: f64,
    pub pct_baik: f64,
    pub rr: i64,
    pub rb: i64,
    pub habis: i64,
    pub pct_habis: f64,
    pub idle: i64,
    pub hilang: i64,
    /// Ekspektasi rusak berat baru selama cakrawala.
    pub rb5: f64,
    /// Habis masa manfaat selama cakrawala.
    pub habis5: f64,
    /// Kebutuhan skenario moderat selama cakrawala termasuk tunggakan.
    pub ganti5: f64,
    pub biaya5: f64,
    pub rku: f64,
    pub skor: f64,
}

/// Rincian satu satuan kerja.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct SatkerRinci {
    pub meta: Meta,
    pub kode: String,
    pub nama: String,
    pub prov: String,
    pub peringkat: i64,
    pub dari: i64,
    pub skor: Skor,
    pub skor_nasional: f64,
    pub r: Ringkas,
    pub nasional_pct_baik: f64,
    pub nasional_pct_habis: f64,
    pub kategori: Vec<KategoriBaris>,
    pub aset_rb: Vec<AsetRisiko>,
    pub aset_risiko: Vec<AsetRisiko>,
    pub aset_habis: Vec<AsetRisiko>,
    pub rekomendasi: Vec<Rekomendasi>,
}

/// Satu aset pada daftar prioritas.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct AsetRisiko {
    /// Id baris pada penyimpanan lokal (untuk melengkapi identitas).
    pub id: i64,
    pub nama: String,
    pub kd_brg: String,
    pub no_aset: String,
    pub kategori: String,
    pub kelompok: String,
    pub satker: String,
    pub kode_satker: String,
    pub tahun: i32,
    pub umur: f64,
    pub kondisi: String,
    pub nilai: f64,
    /// Sisa masa manfaat (tahun); negatif bila tidak relevan.
    pub sisa: f64,
    /// Peluang menjadi rusak berat dalam cakrawala (0–1).
    pub p_rb: f64,
}

/// Jumlah dan nilai perolehan satu tahun.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct TrenTahun {
    pub tahun: i32,
    pub unit: i64,
    pub nilai: f64,
}

/// Sebaran kondisi pada satu pita umur.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct UmurPita {
    pub label: String,
    pub baik: i64,
    pub rr: i64,
    pub rb: i64,
    pub tak: i64,
    pub nilai: f64,
}

/// Satu titik pembanding kurva kerusakan: teramati vs model.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct TitikKurva {
    pub umur: f64,
    pub n: i64,
    /// Porsi tidak baik (RR + RB) yang teramati, 0–1.
    pub obs_tb: f64,
    pub obs_rb: f64,
    pub mod_tb: f64,
    pub mod_rb: f64,
}

/// Keterangan model kerusakan satu kategori.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ModelInfo {
    pub tabel: String,
    pub label: String,
    /// Banyaknya aset yang dipakai untuk menaksir.
    pub n: i64,
    /// Memakai parameter gabungan karena sampel kategori terlalu sedikit.
    pub gabungan: bool,
    /// Median umur saat aset mulai tidak baik (tahun).
    pub median_mulai: f64,
    /// Rata-rata lama di kondisi rusak ringan sebelum rusak berat (tahun).
    pub lama_rr: f64,
    /// Bentuk kurva Weibull (>1 berarti laju kerusakan naik seiring umur).
    pub bentuk: f64,
    /// Rata-rata selisih mutlak model vs data (poin persentase).
    pub mae: f64,
    pub kurva: Vec<TitikKurva>,
}

/// Satu titik prakiraan beserta rentang ketidakpastian 80%.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct RamalTitik {
    pub tahun: i32,
    pub nilai: f64,
    pub bawah: f64,
    pub atas: f64,
}

/// Prakiraan satu deret tahunan.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Deret {
    /// Metode terpilih hasil uji balik.
    pub metode: String,
    /// Galat uji balik (sMAPE, persen); negatif bila tidak dapat dihitung.
    pub galat: f64,
    /// Riwayat tahun penuh (tahun berjalan tidak termasuk).
    pub riwayat: Vec<(i32, f64)>,
    /// Titik pertama = taksiran setahun penuh tahun berjalan, lalu tahun-tahun berikutnya.
    pub ramal: Vec<RamalTitik>,
}

/// Prakiraan perolehan satu kategori (atau total).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Prakiraan {
    pub kunci: String,
    pub label: String,
    pub unit: Deret,
    pub nilai: Deret,
    /// Realisasi tahun berjalan sampai tanggal data (unit, nilai).
    pub berjalan: (f64, f64),
}

/// Kapasitas pengadaan historis kategori yang dapat diganti/direhabilitasi.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Kapasitas {
    /// Rata-rata unit per tahun pada lima tahun penuh terakhir.
    pub unit_per_tahun: f64,
    /// Rata-rata nilai perolehan per tahun pada lima tahun penuh terakhir.
    pub nilai_per_tahun: f64,
    pub tahun_awal: i32,
    pub tahun_akhir: i32,
}

/// Kebutuhan per kelompok barang (apa yang perlu diadakan).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct KelompokButuh {
    pub kategori: String,
    pub kelompok: String,
    pub unit_total: i64,
    /// Kebutuhan unit per skenario selama cakrawala termasuk tunggakan.
    pub unit: [f64; 3],
    pub biaya: [f64; 3],
    /// Harga satuan acuan (Rp).
    pub harga: f64,
    /// Sumber harga acuan: "median 5 th" atau "nilai perolehan".
    pub sumber_harga: String,
    /// Unit idle berkondisi baik yang dapat dialihkan.
    pub idle_baik: i64,
}

/// Proyeksi nilai buku portofolio yang ada (tanpa pengadaan baru).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct BukuTahun {
    pub tahun: i32,
    pub total: f64,
    pub peralatan: f64,
}

/// Kelengkapan satu isian.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct KolomMutu {
    pub label: String,
    pub lingkup: String,
    pub terisi: i64,
    pub total: i64,
}

impl KolomMutu {
    pub fn pct(&self) -> f64 {
        100.0 * bagi(self.terisi as f64, self.total as f64)
    }
}

/// Satu jenis anomali data.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Anomali {
    /// Kunci tetap untuk dirujuk mesin rekomendasi.
    pub kunci: String,
    pub judul: String,
    pub jumlah: i64,
    pub nilai: f64,
    pub uraian: String,
}

/// Mutu data dan kepatuhan administrasi.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Kualitas {
    pub kolom: Vec<KolomMutu>,
    pub anomali: Vec<Anomali>,
    /// (kategori, persen kelengkapan isian kunci, unit).
    pub per_kategori: Vec<(String, f64, i64)>,
    /// Uji konsistensi satuan `umur_sisa`: rasio median terhadap taksiran dari penyusutan.
    pub rasio_umur_sisa: f64,
    pub catatan_umur_sisa: String,
}

/// Butir sorotan untuk pembaca cepat.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Sorotan {
    /// "positif", "waspada", "kritis", atau "info".
    pub nada: String,
    pub judul: String,
    pub isi: String,
}

/// Rekomendasi tindak lanjut.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Rekomendasi {
    /// 1 = tinggi, 2 = sedang, 3 = rendah.
    pub prioritas: u8,
    pub bidang: String,
    pub judul: String,
    pub temuan: String,
    pub tindakan: Vec<String>,
    pub dasar: String,
    /// Satuan kerja yang paling terkait (nama).
    pub satker: Vec<String>,
}

impl Rekomendasi {
    pub fn label_prioritas(&self) -> (&'static str, &'static str) {
        match self.prioritas {
            1 => ("Prioritas tinggi", "red"),
            2 => ("Prioritas sedang", "amber"),
            _ => ("Prioritas rendah", "blue"),
        }
    }
}

/// Perubahan terhadap posisi data sebelumnya.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Delta {
    pub tanggal: String,
    pub unit: i64,
    pub nilai: f64,
    pub rb: i64,
    pub habis: i64,
    pub idle: i64,
    pub hilang: i64,
}

/// Hasil analitik tingkat nasional (Kejaksaan RI).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Analisis {
    pub meta: Meta,
    pub nasional: Ringkas,
    pub skor: Skor,
    pub kategori: Vec<KategoriBaris>,
    pub provinsi: Vec<ProvBaris>,
    pub tren: Vec<TrenTahun>,
    pub umur: Vec<UmurPita>,
    pub model: Vec<ModelInfo>,
    pub prakiraan: Vec<Prakiraan>,
    pub buku_th: Vec<BukuTahun>,
    pub kapasitas: Kapasitas,
    pub kelompok_butuh: Vec<KelompokButuh>,
    /// Satuan kerja dengan skor terendah.
    pub satker_skor: Vec<SatkerBaris>,
    /// Satuan kerja dengan kebutuhan biaya terbesar.
    pub satker_butuh: Vec<SatkerBaris>,
    /// Satuan kerja dengan ekspektasi rusak berat terbesar.
    pub satker_rb: Vec<SatkerBaris>,
    /// Satuan kerja dengan kerusakan paling jauh di atas wajarnya umur.
    pub satker_rku: Vec<SatkerBaris>,
    /// Aset rusak berat bernilai terbesar.
    pub aset_rb: Vec<AsetRisiko>,
    /// Aset dengan peluang rusak berat tertinggi.
    pub aset_risiko: Vec<AsetRisiko>,
    pub kualitas: Kualitas,
    pub sorotan: Vec<Sorotan>,
    pub rekomendasi: Vec<Rekomendasi>,
    pub delta: Option<Delta>,
}

/// Perolehan pada bulan pelaporan dan sepanjang tahun berjalan.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct AktivitasPeriode {
    pub label: String,
    pub unit_bulan: i64,
    pub nilai_bulan: f64,
    pub unit_tahun: i64,
    pub nilai_tahun: f64,
    /// (kategori, unit, nilai) pada bulan pelaporan.
    pub per_kategori: Vec<(String, i64, f64)>,
}

/// Kemajuan sinkronisasi satu tabel.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ProgresTabel {
    pub table: String,
    pub label: String,
    /// "antre", "berjalan", "selesai", "gagal", atau "".
    pub status: String,
    pub terambil: i64,
    pub total: i64,
    pub pesan: String,
}

/// Kemajuan pekerjaan sinkronisasi latar.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Progres {
    pub berjalan: bool,
    pub mulai: String,
    pub selesai: String,
    pub tabel: Vec<ProgresTabel>,
    pub log: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn persen_dan_total_ringkas() {
        let mut r = Ringkas {
            unit: 100,
            baik: 60,
            rr: 30,
            rb: 10,
            susut_unit: 50,
            habis: 20,
            idle: 5,
            hilang: 1,
            ..Default::default()
        };
        r.ganti_kini = [10.0, 15.0, 30.0];
        r.ganti_th[1] = [1.0, 2.0, 3.0, 4.0, 5.0, 6.0];
        assert_eq!(r.pct_baik(), 60.0);
        assert_eq!(r.pct_rb(), 10.0);
        assert_eq!(r.pct_habis(), 40.0);
        assert_eq!(r.ganti_total(1), 36.0);
        assert_eq!(r.ganti_total(0), 10.0);
    }

    #[test]
    fn gabung_menjumlahkan_semua_bagian() {
        let mut a = Ringkas {
            unit: 3,
            nilai: 10.0,
            baik: 2,
            ..Default::default()
        };
        a.rb_th[2] = 1.5;
        let mut b = Ringkas {
            unit: 4,
            nilai: 5.0,
            baik: 1,
            ..Default::default()
        };
        b.rb_th[2] = 0.5;
        b.biaya_th[2][5] = 7.0;
        a.gabung(&b);
        assert_eq!(a.unit, 7);
        assert_eq!(a.nilai, 15.0);
        assert_eq!(a.baik, 3);
        assert_eq!(a.rb_th[2], 2.0);
        assert_eq!(a.biaya_th[2][5], 7.0);
    }

    #[test]
    fn skor_sempurna_dan_buruk() {
        let sempurna = Ringkas {
            unit: 100,
            fisik: 100,
            baik: 100,
            susut_unit: 100,
            psp_ya: 100,
            psp_n: 100,
            lengkap_ya: 100,
            lengkap_n: 100,
            ..Default::default()
        };
        assert!((Skor::dari(&sempurna).total - 100.0).abs() < 1e-9);
        let buruk = Ringkas {
            unit: 100,
            fisik: 100,
            rb: 100,
            susut_unit: 100,
            habis: 100,
            idle: 50,
            hilang: 10,
            psp_n: 100,
            lengkap_n: 100,
            ..Default::default()
        };
        assert!(Skor::dari(&buruk).total < 1.0);
        assert_eq!(Skor::predikat(85.0).0, "Baik");
        assert_eq!(Skor::predikat(70.0).0, "Perlu Perhatian");
        assert_eq!(Skor::predikat(40.0).0, "Kritis");
    }

    #[test]
    fn rku_netral_bila_sesuai_harapan() {
        let r = Ringkas {
            rusak_obs: 40.0,
            rusak_harap: 40.0,
            ..Default::default()
        };
        assert!((r.rku() - 1.0).abs() < 1e-9);
        let buruk = Ringkas {
            rusak_obs: 80.0,
            rusak_harap: 40.0,
            ..Default::default()
        };
        assert!(buruk.rku() > 1.5);
    }
}
