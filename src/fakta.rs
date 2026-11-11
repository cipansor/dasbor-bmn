//! Normalisasi baris mentah SLDK menjadi kolom-kolom fakta yang dianalisis.
//!
//! Baris dari gateway berupa objek JSON 106 kolom. Sebelum disimpan, kunci
//! diseragamkan ke huruf kecil (gateway berbasis Oracle kerap mengirim huruf
//! besar), lalu isian yang dipakai analitik diekstrak ke kolom bertipe: kondisi,
//! nilai, tanggal perolehan, sisa umur, dan penanda status. Dengan begitu
//! analitik tidak perlu mengurai ulang JSON dan tahan terhadap variasi format.

#![cfg(feature = "ssr")]

use crate::model::{cell_to_string, parse_angka, to_f64};
use serde_json::{Map, Value};

/// Kode kondisi fisik.
pub const K_TAK: i64 = 0;
pub const K_BAIK: i64 = 1;
pub const K_RR: i64 = 2;
pub const K_RB: i64 = 3;

/// Penanda status (bitmask pada kolom `flags`).
pub const F_IDLE: i64 = 1;
pub const F_HILANG: i64 = 1 << 1;
pub const F_RUSAK_YN: i64 = 1 << 2;
pub const F_DIHENTIKAN: i64 = 1 << 3;
pub const F_DIHAPUS: i64 = 1 << 4;
pub const F_PSP: i64 = 1 << 5;
pub const F_BUKTI: i64 = 1 << 6;
pub const F_SENGKETA: i64 = 1 << 7;
pub const F_SBSN: i64 = 1 << 8;
pub const F_EKSTRA: i64 = 1 << 9;
pub const F_PENGHUNI: i64 = 1 << 10;
pub const F_FOTO: i64 = 1 << 11;
pub const F_GPS: i64 = 1 << 12;

/// Kolom hasil ekstraksi satu baris aset.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Kolom {
    pub kunci: String,
    pub kode_satker: String,
    pub nama_satker: String,
    pub prov: String,
    pub kab: String,
    pub kd_brg: String,
    pub no_aset: String,
    pub nama: String,
    pub kelompok: String,
    pub merk: String,
    pub kondisi: i64,
    pub nilai: f64,
    pub buku: f64,
    /// Tahun dan bulan perolehan; 0 bila tidak sah.
    pub tahun: i64,
    pub bulan: i64,
    pub umur_sisa: Option<f64>,
    pub flags: i64,
}

/// Seragamkan kunci objek ke huruf kecil dan rapikan spasi pada teks.
pub fn normalkan(v: Value) -> Value {
    match v {
        Value::Object(o) => {
            let mut baru = Map::with_capacity(o.len());
            for (k, isi) in o {
                let isi = match isi {
                    Value::String(s) => {
                        let t = s.trim();
                        if t.len() == s.len() {
                            Value::String(s)
                        } else {
                            Value::String(t.to_string())
                        }
                    }
                    lain => lain,
                };
                baru.insert(k.trim().to_ascii_lowercase(), isi);
            }
            Value::Object(baru)
        }
        lain => lain,
    }
}

fn teks(row: &Value, kunci: &[&str]) -> String {
    for k in kunci {
        if let Some(v) = row.get(*k) {
            let s = cell_to_string(v);
            let s = s.trim();
            if !s.is_empty() {
                return s.to_string();
            }
        }
    }
    String::new()
}

fn angka(row: &Value, kunci: &str) -> f64 {
    row.get(kunci).map(to_f64).unwrap_or(0.0)
}

/// Apakah isian benar-benar memuat nilai (bukan tanda kosong seperti "-").
pub fn terisi(s: &str) -> bool {
    let t = s.trim().to_lowercase();
    if t.is_empty() {
        return false;
    }
    if matches!(
        t.as_str(),
        "-" | "--" | "0" | "null" | "n/a" | "na" | "nihil" | "tidak ada" | "tidak" | "tdk"
    ) {
        return false;
    }
    !t.starts_with("belum")
}

/// Penanda Y/N.
pub fn ya(s: &str) -> bool {
    matches!(
        s.trim().to_ascii_uppercase().as_str(),
        "Y" | "YA" | "1" | "TRUE" | "T" | "YES"
    )
}

/// Baca kondisi fisik dari uraian bebas.
pub fn parse_kondisi(s: &str) -> i64 {
    let t = s.trim().to_lowercase();
    if t.is_empty() {
        return K_TAK;
    }
    if t.contains("berat") || t == "rb" {
        K_RB
    } else if t.contains("ringan") || t == "rr" || t.contains("kurang baik") {
        K_RR
    } else if t.contains("baik") || t == "b" {
        K_BAIK
    } else if t.contains("rusak") {
        // "Rusak" tanpa keterangan diperlakukan sebagai rusak ringan (konservatif).
        K_RR
    } else {
        K_TAK
    }
}

pub fn label_kondisi(k: i64) -> &'static str {
    match k {
        K_BAIK => "Baik",
        K_RR => "Rusak Ringan",
        K_RB => "Rusak Berat",
        _ => "Tidak diisi",
    }
}

fn tanggal_sah(y: i32, m: u32, d: u32) -> Option<(i32, u32, u32)> {
    if (1900..=2100).contains(&y) && (1..=12).contains(&m) && (1..=31).contains(&d) {
        Some((y, m, d))
    } else {
        None
    }
}

/// Baca tanggal dari berbagai bentuk: `YYYY-MM-DD…`, `DD-MM-YYYY`, `DD/MM/YYYY`,
/// `YYYYMMDD`, atau epoch (detik/milidetik).
pub fn parse_tanggal(s: &str) -> Option<(i32, u32, u32)> {
    let t = s.trim();
    if t.is_empty() {
        return None;
    }
    let kepala: &str = t.split(['T', ' ']).next().unwrap_or(t);
    let bagian: Vec<&str> = kepala.split(['-', '/', '.']).collect();
    if bagian.len() == 3 {
        let n: Vec<Option<i64>> = bagian.iter().map(|b| b.parse::<i64>().ok()).collect();
        if let (Some(a), Some(b), Some(c)) = (n[0], n[1], n[2]) {
            if bagian[0].len() == 4 {
                return tanggal_sah(a as i32, b as u32, c as u32);
            }
            if bagian[2].len() == 4 {
                return tanggal_sah(c as i32, b as u32, a as u32);
            }
        }
        return None;
    }
    if kepala.len() == 8 && kepala.chars().all(|c| c.is_ascii_digit()) {
        let y: i32 = kepala[..4].parse().ok()?;
        let m: u32 = kepala[4..6].parse().ok()?;
        let d: u32 = kepala[6..8].parse().ok()?;
        return tanggal_sah(y, m, d);
    }
    // Epoch: milidetik (13 digit) atau detik (10 digit).
    let digit: String = t.chars().filter(|c| c.is_ascii_digit()).collect();
    if digit.len() >= 10 && digit.len() == t.trim_start_matches('-').len() {
        let mut detik: i64 = digit.parse().ok()?;
        if digit.len() >= 12 {
            detik /= 1000;
        }
        let dt = chrono::DateTime::from_timestamp(detik, 0)?;
        use chrono::Datelike;
        return tanggal_sah(dt.year(), dt.month(), dt.day());
    }
    None
}

/// Kunci satuan kerja yang aman dipakai di URL.
fn kunci_satker(kode: &str, nama: &str) -> String {
    let k = kode.trim();
    if !k.is_empty() {
        return k
            .chars()
            .map(|c| {
                if c.is_ascii_alphanumeric() || c == '.' || c == '-' || c == '_' {
                    c
                } else {
                    '-'
                }
            })
            .collect();
    }
    if nama.trim().is_empty() {
        return "TANPA-SATKER".to_string();
    }
    let s: String = nama
        .trim()
        .to_uppercase()
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect();
    format!("N-{s}")
}

/// Ekstrak kolom fakta dari satu baris yang sudah dinormalkan.
pub fn ekstrak(row: &Value, idx: usize) -> Kolom {
    let kode = teks(row, &["kode_satker"]);
    let nama_satker = teks(row, &["nama_satker"]);
    let kd_brg = teks(row, &["kd_brg"]);
    let no_aset = teks(row, &["no_aset"]);
    let register = teks(row, &["kode_register"]);
    let kode_satker = kunci_satker(&kode, &nama_satker);

    let kunci = if !register.is_empty() {
        format!("r:{register}")
    } else if !kd_brg.is_empty() || !no_aset.is_empty() {
        format!("k:{kode_satker}|{kd_brg}|{no_aset}")
    } else {
        format!("i:{idx}")
    };

    let susut = angka(row, "rph_susut").max(0.0);
    let mut nilai = angka(row, "rph_aset");
    if nilai <= 0.0 {
        nilai = angka(row, "rph_perolehan").max(0.0);
    }
    let buku_sumber = row.get("rph_buku").filter(|v| !v.is_null()).map(to_f64);
    let buku = match buku_sumber {
        Some(b) if b > 0.0 || susut >= nilai => b.max(0.0),
        _ => (nilai - susut).max(0.0),
    };

    let tgl = parse_tanggal(&teks(row, &["tgl_perlh"]))
        .or_else(|| parse_tanggal(&teks(row, &["tgl_buku_pertama"])));
    // Tanggal bawaan basis data (mis. 1900-01-01) bukan tanggal perolehan sungguhan.
    let (tahun, bulan) = match tgl {
        Some((y, m, _)) if y >= 1945 => (y as i64, m as i64),
        _ => (0, 0),
    };

    let umur_sisa = match row.get("umur_sisa") {
        None | Some(Value::Null) => None,
        Some(Value::Number(n)) => n.as_f64(),
        Some(Value::String(s)) => parse_angka(s),
        _ => None,
    }
    .filter(|x| x.is_finite())
    .map(|x| x.max(0.0));

    let mut flags = 0i64;
    let mut set = |syarat: bool, f: i64| {
        if syarat {
            flags |= f;
        }
    };
    set(ya(&teks(row, &["status_bmn_idle"])), F_IDLE);
    set(ya(&teks(row, &["brg_hilang_yn"])), F_HILANG);
    set(ya(&teks(row, &["brg_rusak_yn"])), F_RUSAK_YN);
    let status = teks(row, &["ur_status"]).to_lowercase();
    set(
        ya(&teks(row, &["dihentikan_yn"])) || status.contains("henti"),
        F_DIHENTIKAN,
    );
    set(
        parse_tanggal(&teks(row, &["tgl_hapus"])).is_some_and(|(y, _, _)| y >= 1945),
        F_DIHAPUS,
    );
    set(terisi(&teks(row, &["no_psp"])), F_PSP);
    set(
        terisi(&teks(row, &["no_dok_bukti_kepemilikan"]))
            || terisi(&teks(row, &["jns_sertifikat"])),
        F_BUKTI,
    );
    set(terisi(&teks(row, &["no_perkara_hukum"])), F_SENGKETA);
    let sbsn = teks(row, &["status_sbsn"]).to_lowercase();
    set(
        terisi(&sbsn) && !sbsn.starts_with("non") && sbsn != "n",
        F_SBSN,
    );
    let intra = teks(row, &["intra_extra"]).to_lowercase();
    set(
        intra.contains("ekstra") || intra.contains("extra"),
        F_EKSTRA,
    );
    set(terisi(&teks(row, &["nm_penghuni"])), F_PENGHUNI);
    set(angka(row, "jml_photo") > 0.0, F_FOTO);
    set(
        angka(row, "gps_latitude") != 0.0 && angka(row, "gps_longitude") != 0.0,
        F_GPS,
    );

    Kolom {
        kunci,
        kode_satker,
        nama_satker,
        prov: teks(row, &["ur_prov"]),
        kab: teks(row, &["ur_kab"]),
        kd_brg,
        no_aset,
        nama: teks(row, &["nama"]),
        kelompok: teks(row, &["ur_sskel"]),
        merk: teks(row, &["merk"]),
        kondisi: parse_kondisi(&teks(row, &["ur_kondisi"])),
        nilai,
        buku,
        tahun,
        bulan,
        umur_sisa,
        flags,
    }
}

/// Akronim yang tetap huruf besar saat nama dirapikan.
const AKRONIM: &[&str] = &[
    "RI", "DKI", "DI", "NTB", "NTT", "II", "III", "IV", "TIK", "BMN", "KDP", "TU", "SDM", "PPA",
    "UPT", "RSU", "RS",
];

/// Rapikan nama huruf besar menjadi huruf kapital di awal kata.
pub fn judul(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for (i, kata) in s.split_whitespace().enumerate() {
        if i > 0 {
            out.push(' ');
        }
        let atas = kata.to_uppercase();
        let inti: String = atas.chars().filter(|c| c.is_alphanumeric()).collect();
        if AKRONIM.contains(&inti.as_str()) {
            out.push_str(&atas);
            continue;
        }
        let bawah = kata.to_lowercase();
        if i > 0
            && matches!(
                bawah.as_str(),
                "dan" | "di" | "ke" | "dari" | "pada" | "yang"
            )
        {
            out.push_str(&bawah);
            continue;
        }
        let mut awal = true;
        for c in bawah.chars() {
            if awal && c.is_alphabetic() {
                out.extend(c.to_uppercase());
                awal = false;
            } else {
                out.push(c);
                if !c.is_alphanumeric() && c != '\'' {
                    awal = true;
                }
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn kunci_huruf_besar_dinormalkan() {
        let v = normalkan(json!({"KD_BRG": " 3020101001 ", "Rph_Aset": 5, "NAMA_SATKER": "X"}));
        assert_eq!(v["kd_brg"], "3020101001");
        assert_eq!(v["rph_aset"], 5);
        assert_eq!(v["nama_satker"], "X");
    }

    #[test]
    fn tanggal_berbagai_bentuk() {
        assert_eq!(parse_tanggal("2019-05-01 00:00:00"), Some((2019, 5, 1)));
        assert_eq!(
            parse_tanggal("2019-05-01T00:00:00.000Z"),
            Some((2019, 5, 1))
        );
        assert_eq!(parse_tanggal("01-05-2019"), Some((2019, 5, 1)));
        assert_eq!(parse_tanggal("31/12/2020"), Some((2020, 12, 31)));
        assert_eq!(parse_tanggal("20210817"), Some((2021, 8, 17)));
        assert_eq!(parse_tanggal("1556668800000"), Some((2019, 5, 1)));
        assert_eq!(parse_tanggal("1556668800"), Some((2019, 5, 1)));
        assert_eq!(parse_tanggal(""), None);
        assert_eq!(parse_tanggal("tidak ada"), None);
        assert_eq!(parse_tanggal("2019-13-01"), None);
    }

    #[test]
    fn kondisi_dari_uraian() {
        assert_eq!(parse_kondisi("Baik"), K_BAIK);
        assert_eq!(parse_kondisi("RUSAK RINGAN"), K_RR);
        assert_eq!(parse_kondisi("Rusak Berat"), K_RB);
        assert_eq!(parse_kondisi("rb"), K_RB);
        assert_eq!(parse_kondisi(""), K_TAK);
        assert_eq!(parse_kondisi("???"), K_TAK);
        // "Kurang baik" tidak boleh terbaca sebagai baik.
        assert_eq!(parse_kondisi("Kurang Baik"), K_RR);
    }

    #[test]
    fn ekstraksi_lengkap() {
        let row = normalkan(json!({
            "KODE_SATKER": "005123", "NAMA_SATKER": "KEJAKSAAN NEGERI BANDUNG",
            "KD_BRG": "3020101003", "NO_ASET": 7, "NAMA": "Mini Bus",
            "RPH_ASET": "350.000.000", "RPH_SUSUT": 150000000, "RPH_BUKU": 200000000,
            "UR_KONDISI": "Rusak Ringan", "TGL_PERLH": "2018-03-15 00:00:00",
            "UMUR_SISA": "1,5", "STATUS_BMN_IDLE": "Y", "BRG_HILANG_YN": "N",
            "NO_PSP": "KEP-12/2019", "TGL_HAPUS": "1900-01-01 00:00:00",
            "UR_PROV": "JAWA BARAT"
        }));
        let k = ekstrak(&row, 0);
        assert_eq!(k.kode_satker, "005123");
        assert_eq!(k.kunci, "k:005123|3020101003|7");
        assert_eq!(k.nilai, 350_000_000.0);
        assert_eq!(k.buku, 200_000_000.0);
        assert_eq!(k.kondisi, K_RR);
        assert_eq!((k.tahun, k.bulan), (2018, 3));
        assert_eq!(k.umur_sisa, Some(1.5));
        assert!(k.flags & F_IDLE != 0);
        assert!(k.flags & F_HILANG == 0);
        assert!(k.flags & F_PSP != 0);
        // Tanggal hapus bawaan 1900 bukan penghapusan sungguhan.
        assert!(k.flags & F_DIHAPUS == 0);
    }

    #[test]
    fn nilai_buku_dihitung_bila_kosong() {
        let k = ekstrak(&json!({"rph_aset": 100, "rph_susut": 30}), 3);
        assert_eq!(k.buku, 70.0);
        assert_eq!(k.kunci, "i:3");
        assert_eq!(k.kode_satker, "TANPA-SATKER");
        // Nilai buku nol sah bila aset memang sudah tersusut penuh.
        let k = ekstrak(
            &json!({"rph_aset": 100, "rph_susut": 100, "rph_buku": 0}),
            0,
        );
        assert_eq!(k.buku, 0.0);
        // Nilai buku nol tanpa penyusutan dianggap tidak diisi.
        let k = ekstrak(&json!({"rph_aset": 100, "rph_susut": 0, "rph_buku": 0}), 0);
        assert_eq!(k.buku, 100.0);
    }

    #[test]
    fn penanda_kosong_tidak_dianggap_terisi() {
        assert!(!terisi("-"));
        assert!(!terisi("  "));
        assert!(!terisi("Belum bersertifikat"));
        assert!(!terisi("Tidak Ada"));
        assert!(terisi("SHP No. 12"));
    }

    #[test]
    fn satker_tanpa_kode_memakai_nama() {
        let k = ekstrak(&json!({"nama_satker": "Kejari Kota X/Y"}), 0);
        assert_eq!(k.kode_satker, "N-KEJARI-KOTA-X-Y");
    }

    #[test]
    fn nama_dirapikan() {
        assert_eq!(
            judul("KEJAKSAAN NEGERI BANDUNG"),
            "Kejaksaan Negeri Bandung"
        );
        assert_eq!(
            judul("KEJAKSAAN TINGGI DKI JAKARTA"),
            "Kejaksaan Tinggi DKI Jakarta"
        );
        assert_eq!(judul("DI YOGYAKARTA"), "DI Yogyakarta");
        assert_eq!(judul("jalan dan jembatan"), "Jalan dan Jembatan");
        assert_eq!(
            judul("KEPULAUAN BANGKA-BELITUNG"),
            "Kepulauan Bangka-Belitung"
        );
    }
}
