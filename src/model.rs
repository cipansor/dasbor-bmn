use serde::{Deserialize, Serialize};

/// Jenis kebutuhan yang timbul ketika aset suatu kategori rusak berat.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum JenisKebutuhan {
    /// Peralatan dan mesin: diganti unit baru.
    Penggantian,
    /// Bangunan dan prasarana: direhabilitasi berat.
    Rehabilitasi,
    /// Tidak menimbulkan kebutuhan pengadaan (tanah, KDP, aset tak berwujud, dst.).
    TidakAda,
}

impl JenisKebutuhan {
    pub fn label(&self) -> &'static str {
        match self {
            JenisKebutuhan::Penggantian => "Penggantian",
            JenisKebutuhan::Rehabilitasi => "Rehabilitasi",
            JenisKebutuhan::TidakAda => "-",
        }
    }
}

/// Definisi satu tabel BMN pada layanan SLDK SIMAN v2.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TableDef {
    /// Nama tabel pada SIMAN, mis. `SIMAN2_M_ASET_TANAH`.
    pub table: &'static str,
    /// Nama resource pada gateway, mis. `getAsetTanah`.
    pub resource: &'static str,
    /// Label singkat yang ramah pengguna, mis. "Tanah".
    pub label: &'static str,
    /// Nama data resmi dari dokumen SLDK.
    pub nama_data: &'static str,
    /// Ukuran volume data menurut dokumen SLDK.
    pub volume: &'static str,
    /// Ikon (emoji) untuk kartu kategori.
    pub icon: &'static str,
    /// Kondisi fisik (baik/rusak ringan/rusak berat) relevan untuk kategori ini.
    pub fisik: bool,
    /// Kategori disusutkan/diamortisasi sehingga punya masa manfaat.
    pub disusutkan: bool,
    /// Kebutuhan yang timbul saat aset rusak berat.
    pub kebutuhan: JenisKebutuhan,
    /// Masa manfaat bawaan (tahun); 0 bila tidak disusutkan.
    pub masa_manfaat: f64,
}

/// Metadata satu kolom pada tabel BMN.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ColumnDef {
    pub name: String,
    pub label: String,
    pub r#type: String,
    pub len: String,
}

/// Status sinkronisasi satu tabel terhadap sumber SLDK.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SyncStatus {
    pub table: String,
    pub label: String,
    pub row_count: i64,
    pub last_sync: Option<String>,
    pub status: String,
    pub message: Option<String>,
}

/// Satu baris data aset yang sudah dinormalisasi untuk tampilan tabel.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AssetRow {
    pub values: Vec<String>,
}

/// Halaman data aset beserta metadata paginasi.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AssetPage {
    pub table: String,
    pub label: String,
    pub columns: Vec<ColumnDef>,
    pub rows: Vec<AssetRow>,
    pub page: i64,
    pub per_page: i64,
    pub total: i64,
    pub is_demo: bool,
}

/// Konfigurasi koneksi yang aktif (nilai rahasia tidak pernah dikirim ke klien).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ConnInfo {
    pub base_url_set: bool,
    pub client_id: Option<String>,
    pub grant_type: Option<String>,
    pub ba_key: Option<String>,
    pub client_secret_set: bool,
    pub token_valid: bool,
    pub last_error: Option<String>,
}

/// Bentuk angka desimal gaya Indonesia (koma sebagai pemisah desimal).
pub fn format_desimal(v: f64, digit: usize) -> String {
    let s = format!("{v:.digit$}");
    let (bulat, pecahan) = match s.split_once('.') {
        Some((b, p)) => (b.to_string(), Some(p.to_string())),
        None => (s, None),
    };
    let neg = bulat.starts_with('-');
    let angka: i64 = bulat.trim_start_matches('-').parse().unwrap_or(0);
    let mut out = format_number(angka);
    if neg && (angka != 0 || pecahan.as_deref().is_some_and(|p| p.chars().any(|c| c != '0'))) {
        out = format!("-{out}");
    }
    match pecahan {
        Some(p) => format!("{out},{p}"),
        None => out,
    }
}

/// Persentase satu desimal, mis. `12,3%`.
pub fn format_persen(v: f64) -> String {
    format!("{}%", format_desimal(v, 1))
}

/// Jumlah unit yang boleh pecahan (hasil ekspektasi) menjadi bilangan bulat terbaca.
pub fn format_unit(v: f64) -> String {
    format_number(v.round() as i64)
}

fn skala_rupiah(v: f64) -> (f64, &'static str, &'static str) {
    // Ambang 999,5 mencegah tampilan seperti "Rp 1000 M" akibat pembulatan.
    let a = v.abs() / 0.9995;
    if a >= 1e12 {
        (1e12, " T", " triliun")
    } else if a >= 1e9 {
        (1e9, " M", " miliar")
    } else if a >= 1e6 {
        (1e6, " jt", " juta")
    } else if a >= 1e3 {
        (1e3, " rb", " ribu")
    } else {
        (1.0, "", "")
    }
}

fn angka_ringkas(n: f64) -> String {
    // Tiga angka penting sudah cukup untuk pembaca pimpinan.
    let a = n.abs();
    let s = if a >= 100.0 {
        format!("{n:.0}")
    } else if a >= 10.0 {
        format!("{n:.1}")
    } else {
        format!("{n:.2}")
    };
    let s = if s.contains('.') {
        s.trim_end_matches('0').trim_end_matches('.').to_string()
    } else {
        s
    };
    s.replace('.', ",")
}

/// Format rupiah ringkas untuk kartu dan tabel, mis. `Rp 1,25 T`.
pub fn format_rupiah_short(v: f64) -> String {
    let (div, suf, _) = skala_rupiah(v);
    format!("Rp {}{}", angka_ringkas(v / div), suf)
}

/// Format rupiah untuk kalimat narasi, mis. `Rp 1,25 triliun`.
pub fn format_rupiah_kata(v: f64) -> String {
    let (div, _, kata) = skala_rupiah(v);
    format!("Rp {}{}", angka_ringkas(v / div), kata)
}

/// Format angka dengan pemisah ribuan gaya Indonesia.
pub fn format_number(v: i64) -> String {
    let neg = v < 0;
    let s = v.unsigned_abs().to_string();
    let mut out = String::new();
    for (i, c) in s.chars().rev().enumerate() {
        if i > 0 && i % 3 == 0 {
            out.push('.');
        }
        out.push(c);
    }
    let s: String = out.chars().rev().collect();
    if neg {
        format!("-{s}")
    } else {
        s
    }
}

/// Format rupiah penuh gaya Indonesia.
pub fn format_rupiah(v: f64) -> String {
    format!("Rp {}", format_number(v.round() as i64))
}

/// Ubah nilai sel menjadi teks tampilan yang aman.
pub fn cell_to_string(v: &serde_json::Value) -> String {
    match v {
        serde_json::Value::Null => String::new(),
        serde_json::Value::String(s) => s.clone(),
        serde_json::Value::Bool(b) => (if *b { "Ya" } else { "Tidak" }).to_string(),
        serde_json::Value::Number(n) => n.to_string(),
        other => other.to_string(),
    }
}

/// Ambil angka dari nilai JSON dengan toleransi string berformat.
///
/// Mengenali bentuk Indonesia (`1.234.567,89`) maupun bentuk mesin (`1234567.89`).
pub fn to_f64(v: &serde_json::Value) -> f64 {
    match v {
        serde_json::Value::Number(n) => n.as_f64().unwrap_or(0.0),
        serde_json::Value::String(s) => parse_angka(s).unwrap_or(0.0),
        serde_json::Value::Bool(b) => {
            if *b {
                1.0
            } else {
                0.0
            }
        }
        _ => 0.0,
    }
}

/// Baca angka dari teks; `None` bila teks tidak memuat angka.
pub fn parse_angka(s: &str) -> Option<f64> {
    let bersih: String = s
        .trim()
        .chars()
        .filter(|c| c.is_ascii_digit() || matches!(c, '.' | ',' | '-'))
        .collect();
    if !bersih.chars().any(|c| c.is_ascii_digit()) {
        return None;
    }
    let titik = bersih.matches('.').count();
    let koma = bersih.matches(',').count();
    let normal = if koma == 1 && titik != 1 {
        // Koma tunggal = desimal gaya Indonesia; titik = pemisah ribuan.
        bersih.replace('.', "").replace(',', ".")
    } else if koma == 1 && titik == 1 {
        // Tentukan pemisah desimal dari yang muncul paling akhir.
        if bersih.rfind(',') > bersih.rfind('.') {
            bersih.replace('.', "").replace(',', ".")
        } else {
            bersih.replace(',', "")
        }
    } else if koma > 1 {
        bersih.replace(',', "")
    } else if titik > 1 {
        bersih.replace('.', "")
    } else {
        bersih
    };
    normal.parse::<f64>().ok().filter(|x| x.is_finite())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn rupiah_ringkas_dan_kata() {
        assert_eq!(format_rupiah_short(1_500_000_000_000.0), "Rp 1,5 T");
        assert_eq!(format_rupiah_short(2_000_000_000.0), "Rp 2 M");
        assert_eq!(format_rupiah_short(48_812_000_000_000.0), "Rp 48,8 T");
        assert_eq!(format_rupiah_short(1_254_000_000.0), "Rp 1,25 M");
        assert_eq!(format_rupiah_short(325_400_000.0), "Rp 325 jt");
        assert_eq!(format_rupiah_short(0.0), "Rp 0");
        assert_eq!(format_rupiah_kata(1_250_000_000_000.0), "Rp 1,25 triliun");
        assert_eq!(format_rupiah_kata(87_300_000.0), "Rp 87,3 juta");
        assert_eq!(format_rupiah(1234567.4), "Rp 1.234.567");
    }

    #[test]
    fn angka_dan_persen() {
        assert_eq!(format_number(1234567), "1.234.567");
        assert_eq!(format_number(-1200), "-1.200");
        assert_eq!(format_number(i64::MIN + 1).chars().next(), Some('-'));
        assert_eq!(format_desimal(1234.56, 1), "1.234,6");
        assert_eq!(format_desimal(-0.04, 1), "0,0");
        assert_eq!(format_desimal(-2.5, 1), "-2,5");
        assert_eq!(format_persen(12.34), "12,3%");
        assert_eq!(format_unit(12.6), "13");
    }

    #[test]
    fn angka_dari_teks() {
        assert_eq!(to_f64(&json!("250")), 250.0);
        assert_eq!(to_f64(&json!(100)), 100.0);
        assert_eq!(to_f64(&json!("1.234.567,89")), 1234567.89);
        assert_eq!(to_f64(&json!("1,234,567.89")), 1234567.89);
        assert_eq!(to_f64(&json!("1234567.89")), 1234567.89);
        assert_eq!(to_f64(&json!("2,5")), 2.5);
        assert_eq!(to_f64(&json!("Rp 1.500.000")), 1500000.0);
        assert_eq!(to_f64(&json!("")), 0.0);
        assert_eq!(to_f64(&json!(null)), 0.0);
        assert_eq!(parse_angka("tidak ada"), None);
    }
}
