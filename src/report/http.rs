//! Handler HTTP untuk unduhan: laporan PDF dan lampiran CSV.

#![cfg(feature = "ssr")]

use std::collections::HashMap;
use std::pin::Pin;
use std::task::{Context, Poll};

use axum::body::Body;
use axum::extract::Query;
use axum::http::{header, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};

use crate::dto::{AktivitasPeriode, NH, SKENARIO};
use crate::report::Periode;
use crate::{analitik, catalog, store};

fn unduhan(isi: impl Into<Body>, jenis: &'static str, nama: &str) -> Response {
    // Nama berkas hanya memuat karakter aman agar tidak merusak kepala HTTP.
    let aman: String = nama
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.') { c } else { '-' })
        .collect();
    let mut res = (StatusCode::OK, isi.into()).into_response();
    let h = res.headers_mut();
    h.insert(header::CONTENT_TYPE, HeaderValue::from_static(jenis));
    if let Ok(v) = HeaderValue::from_str(&format!("attachment; filename=\"{aman}\"")) {
        h.insert(header::CONTENT_DISPOSITION, v);
    }
    h.insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
    res
}

fn galat(status: StatusCode, pesan: &str) -> Response {
    (
        status,
        [(header::CONTENT_TYPE, HeaderValue::from_static("text/plain; charset=utf-8"))],
        pesan.to_string(),
    )
        .into_response()
}

/// `GET /api/laporan.pdf?periode=YYYY-MM[&satker=KODE]`.
///
/// Tanpa `satker` menghasilkan laporan nasional; dengan `satker` menghasilkan
/// profil satuan kerja itu. `periode` kosong berarti bulan berjalan (WIB).
pub async fn laporan_pdf(Query(params): Query<HashMap<String, String>>) -> Response {
    let periode = params
        .get("periode")
        .and_then(|s| Periode::dari_str(s))
        .unwrap_or_else(Periode::sekarang);
    let satker = params
        .get("satker")
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty());
    eprintln!(
        "[audit] unduh laporan PDF periode {} lingkup {}",
        periode.kode(),
        satker.as_deref().unwrap_or("nasional")
    );
    let hasil = tokio::task::spawn_blocking(move || match satker {
        Some(kode) => analitik::rinci_satker(&kode).map(|s| {
            (
                crate::report::bangun_satker(&s, &periode),
                format!("laporan-bmn-{}-{}.pdf", s.kode, periode.kode()),
            )
        }),
        None => {
            let h = analitik::hasil();
            let aktivitas: AktivitasPeriode =
                analitik::aktivitas(periode.tahun, periode.bulan, &periode.label());
            Some((
                crate::report::bangun_nasional(&h.analisis, &h.satker, &periode, &aktivitas),
                format!("laporan-bmn-nasional-{}.pdf", periode.kode()),
            ))
        }
    })
    .await;
    match hasil {
        Ok(Some((bytes, nama))) => unduhan(bytes, "application/pdf", &nama),
        Ok(None) => galat(StatusCode::NOT_FOUND, "Satuan kerja tidak ditemukan."),
        Err(_) => galat(StatusCode::INTERNAL_SERVER_ERROR, "Laporan gagal disusun."),
    }
}

/// Netralkan nilai sel agar tidak dieksekusi sebagai formula spreadsheet.
///
/// Sel yang dimulai dengan `= + - @` (atau tab/CR) diawali apostrof sehingga
/// Excel/LibreOffice memperlakukannya sebagai teks (CWE-1236).
pub fn amankan_sel(v: &str) -> String {
    let berbahaya = v
        .chars()
        .next()
        .is_some_and(|c| matches!(c, '=' | '+' | '-' | '@' | '\t' | '\r'));
    if berbahaya {
        format!("'{v}")
    } else {
        v.to_string()
    }
}

/// Satu sel CSV: dinetralkan lalu dikutip.
fn sel(v: &str) -> String {
    format!("\"{}\"", amankan_sel(v).replace('"', "\"\""))
}

/// Angka untuk CSV: tanpa pemisah ribuan agar terbaca sebagai angka.
fn angka(v: f64) -> String {
    if (v - v.round()).abs() < 1e-9 {
        format!("{v:.0}")
    } else {
        format!("{v:.2}")
    }
}

fn baris_csv(out: &mut Vec<u8>, sel_baris: &[String]) {
    out.extend_from_slice(sel_baris.join(",").as_bytes());
    out.push(b'\n');
}

/// Tanda urutan bita agar Excel membaca berkas sebagai UTF-8.
const BOM: &[u8] = &[0xEF, 0xBB, 0xBF];

/// Aliran potongan bita dari tugas latar ke badan respons.
struct Aliran(tokio::sync::mpsc::Receiver<Result<Vec<u8>, std::io::Error>>);

impl futures_core::Stream for Aliran {
    type Item = Result<Vec<u8>, std::io::Error>;
    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        self.0.poll_recv(cx)
    }
}

/// `GET /api/ekspor.csv?tabel=…` — seluruh baris satu kategori, 106 kolom.
///
/// Berkas dialirkan potong demi potong, sehingga kategori berukuran ratusan
/// megabita tidak perlu ditampung utuh di memori server maupun peramban.
pub async fn ekspor_csv(Query(params): Query<HashMap<String, String>>) -> Response {
    let tabel = params.get("tabel").cloned().unwrap_or_default();
    if catalog::find(&tabel).is_none() {
        return galat(StatusCode::BAD_REQUEST, "Tabel tidak dikenal.");
    }
    eprintln!("[audit] ekspor CSV tabel {tabel}");
    let kolom = crate::schema::columns_of(&tabel);
    let (tx, rx) = tokio::sync::mpsc::channel::<Result<Vec<u8>, std::io::Error>>(8);
    let nama = format!("{tabel}.csv");
    tokio::task::spawn_blocking(move || {
        const POTONG: usize = 64 * 1024;
        let mut buf: Vec<u8> = Vec::with_capacity(POTONG + 4096);
        buf.extend_from_slice(BOM);
        baris_csv(&mut buf, &kolom.iter().map(|c| sel(&c.label)).collect::<Vec<_>>());
        let mut putus = false;
        let hasil = store::alir_baris(&tabel, |v| {
            if putus {
                return;
            }
            let baris: Vec<String> = kolom
                .iter()
                .map(|c| {
                    sel(&v
                        .get(&c.name)
                        .map(crate::model::cell_to_string)
                        .unwrap_or_default())
                })
                .collect();
            baris_csv(&mut buf, &baris);
            if buf.len() >= POTONG {
                // Penerima hilang berarti unduhan dibatalkan; hentikan pekerjaan.
                putus = tx.blocking_send(Ok(std::mem::take(&mut buf))).is_err();
            }
        });
        if putus {
            return;
        }
        match hasil {
            Ok(()) => {
                let _ = tx.blocking_send(Ok(buf));
            }
            Err(e) => {
                let _ = tx.blocking_send(Err(std::io::Error::other(e.to_string())));
            }
        }
    });
    unduhan(Body::from_stream(Aliran(rx)), "text/csv; charset=utf-8", &nama)
}

/// `GET /api/satker.csv` — ringkasan seluruh satuan kerja.
pub async fn satker_csv() -> Response {
    eprintln!("[audit] unduh ringkasan satuan kerja (CSV)");
    let hasil = tokio::task::spawn_blocking(|| {
        let h = analitik::hasil();
        let m = &h.analisis.meta;
        let mut out = BOM.to_vec();
        baris_csv(
            &mut out,
            &[
                "Kode satker",
                "Satuan kerja",
                "Provinsi",
                "Unit",
                "Nilai aset (Rp)",
                "Kondisi baik (%)",
                "Rusak ringan",
                "Rusak berat",
                "Habis masa manfaat",
                "Habis masa manfaat (%)",
                "Idle",
                "Hilang",
                &format!("Rusak berat baru s.d. {}", m.tahun_akhir()),
                &format!("Habis masa manfaat s.d. {}", m.tahun_akhir()),
                "Kebutuhan moderat (unit)",
                "Kebutuhan moderat (Rp)",
                "RKU",
                "Skor kesehatan",
            ]
            .map(sel),
        );
        for s in &h.satker {
            baris_csv(
                &mut out,
                &[
                    sel(&s.kode),
                    sel(&s.nama),
                    sel(&s.prov),
                    angka(s.unit as f64),
                    angka(s.nilai),
                    angka(s.pct_baik),
                    angka(s.rr as f64),
                    angka(s.rb as f64),
                    angka(s.habis as f64),
                    angka(s.pct_habis),
                    angka(s.idle as f64),
                    angka(s.hilang as f64),
                    angka(s.rb5),
                    angka(s.habis5),
                    angka(s.ganti5),
                    angka(s.biaya5),
                    angka(s.rku),
                    angka(s.skor),
                ],
            );
        }
        out
    })
    .await;
    match hasil {
        Ok(b) => unduhan(b, "text/csv; charset=utf-8", "bmn-satuan-kerja.csv"),
        Err(_) => galat(StatusCode::INTERNAL_SERVER_ERROR, "Berkas gagal disusun."),
    }
}

/// `GET /api/kebutuhan.csv` — kebutuhan per satuan kerja × kategori, tiga skenario.
pub async fn kebutuhan_csv() -> Response {
    eprintln!("[audit] unduh kebutuhan per satuan kerja (CSV)");
    let hasil = tokio::task::spawn_blocking(|| {
        let h = analitik::hasil();
        let m = &h.analisis.meta;
        let mut kepala: Vec<String> = [
            "Kode satker",
            "Satuan kerja",
            "Provinsi",
            "Kategori",
            "Jenis kebutuhan",
            "Unit",
            "Rusak berat",
            "Habis masa manfaat",
        ]
        .map(sel)
        .to_vec();
        for (nama, _) in SKENARIO {
            kepala.push(sel(&format!("{nama}: tunggakan (unit)")));
            for k in 0..NH {
                kepala.push(sel(&format!("{nama}: {} (unit)", m.label_tahun(k))));
            }
            kepala.push(sel(&format!("{nama}: total unit")));
            kepala.push(sel(&format!("{nama}: perkiraan biaya (Rp)")));
        }
        let mut out = BOM.to_vec();
        baris_csv(&mut out, &kepala);
        let mut satker: Vec<_> = h.simpan.values().collect();
        satker.sort_by(|a, b| a.prov.cmp(&b.prov).then(a.nama.cmp(&b.nama)));
        for s in satker {
            for (t, r) in &s.per_tabel {
                let def = &catalog::TABLES[*t];
                if def.kebutuhan == crate::model::JenisKebutuhan::TidakAda {
                    continue;
                }
                let mut baris = vec![
                    sel(&s.kode),
                    sel(&s.nama),
                    sel(&s.prov),
                    sel(def.label),
                    sel(def.kebutuhan.label()),
                    angka(r.unit as f64),
                    angka(r.rb as f64),
                    angka(r.habis as f64),
                ];
                for sk in 0..3 {
                    baris.push(angka(r.ganti_kini[sk]));
                    baris.extend(r.ganti_th[sk].iter().map(|v| angka(*v)));
                    baris.push(angka(r.ganti_total(sk)));
                    baris.push(angka(r.biaya_total(sk)));
                }
                baris_csv(&mut out, &baris);
            }
        }
        out
    })
    .await;
    match hasil {
        Ok(b) => unduhan(b, "text/csv; charset=utf-8", "bmn-kebutuhan-satker.csv"),
        Err(_) => galat(StatusCode::INTERNAL_SERVER_ERROR, "Berkas gagal disusun."),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sel_formula_dinetralkan() {
        assert_eq!(amankan_sel("=1+1"), "'=1+1");
        assert_eq!(amankan_sel("+SUM(A1)"), "'+SUM(A1)");
        assert_eq!(amankan_sel("-2+3"), "'-2+3");
        assert_eq!(amankan_sel("@cmd"), "'@cmd");
        assert_eq!(amankan_sel("\t=1"), "'\t=1");
        assert_eq!(amankan_sel("Kantor Kejaksaan"), "Kantor Kejaksaan");
        assert_eq!(amankan_sel(""), "");
        assert_eq!(amankan_sel("A2"), "A2");
    }

    #[test]
    fn sel_dikutip_dan_angka_polos() {
        assert_eq!(sel("a\"b"), "\"a\"\"b\"");
        assert_eq!(sel("=HYPERLINK(\"x\")"), "\"'=HYPERLINK(\"\"x\"\")\"");
        assert_eq!(angka(1234567.0), "1234567");
        assert_eq!(angka(12.345), "12.35");
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn ekspor_mengalirkan_seluruh_baris() {
        let t = "SIMAN2_M_ASET_BANGUNAN_AIR";
        {
            let _k = store::uji_kunci();
            let rows: Vec<serde_json::Value> = (0..2500)
                .map(|i| serde_json::json!({"kd_brg": format!("K{i}"), "nama": "=JAHAT()", "rph_aset": i}))
                .collect();
            store::ganti_tabel(t, rows, "t0").unwrap();
        }
        let mut q = HashMap::new();
        q.insert("tabel".to_string(), t.to_string());
        let res = ekspor_csv(Query(q)).await;
        assert_eq!(res.status(), StatusCode::OK);
        let bytes = axum::body::to_bytes(res.into_body(), usize::MAX).await.unwrap();
        assert_eq!(&bytes[..3], BOM);
        let teks = String::from_utf8_lossy(&bytes[3..]);
        assert_eq!(teks.lines().count(), 2501, "kepala + 2.500 baris");
        assert!(teks.contains("\"'=JAHAT()\""), "sel formula dinetralkan");
        assert!(!teks.contains(",\"=JAHAT()\""));

        let mut q = HashMap::new();
        q.insert("tabel".to_string(), "BUKAN_TABEL".to_string());
        assert_eq!(ekspor_csv(Query(q)).await.status(), StatusCode::BAD_REQUEST);
    }

    #[test]
    fn nama_berkas_unduhan_disaring() {
        let res = unduhan(Vec::<u8>::new(), "text/plain", "a\"b\r\nc.pdf");
        let cd = res.headers()[header::CONTENT_DISPOSITION].to_str().unwrap();
        assert_eq!(cd, "attachment; filename=\"a-b--c.pdf\"");
    }
}
