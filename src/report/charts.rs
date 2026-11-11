//! Penggambar grafik vektor untuk laporan PDF.
//!
//! Bahasa visual mengikuti dasbor: warna kondisi tetap (hijau/kuning/merah),
//! sumbu berangka bulat, dan panjang batang selalu sebanding dengan nilai yang
//! tertulis di sampingnya.

#![cfg(feature = "ssr")]

use super::pdf::{lebar_teks, Page};
use crate::components::chart::batas_bulat;
use crate::dto::RamalTitik;

/// Warna garis bantu dan latar netral.
pub const GARIS: &str = "#d7dced";
pub const TRACK: &str = "#e8ebf5";
pub const TINTA: &str = "#0f172a";
pub const TEKS: &str = "#334155";
pub const REDUP: &str = "#64748b";
pub const UTAMA: &str = "#1f47f5";
pub const C_BAIK: &str = "#10b981";
pub const C_RR: &str = "#f59e0b";
pub const C_RB: &str = "#ef4444";
pub const C_TAK: &str = "#94a3b8";
pub const C_UNGU: &str = "#8b5cf6";
pub const C_BIRU2: &str = "#0ea5e9";

pub fn warna_tone(tone: &str) -> &'static str {
    match tone {
        "emerald" | "green" => C_BAIK,
        "amber" => C_RR,
        "rose" | "red" => C_RB,
        "violet" => C_UNGU,
        "slate" => C_TAK,
        _ => UTAMA,
    }
}

/// Kartu KPI: label kecil, nilai besar, keterangan, dan pita warna di tepi.
#[allow(clippy::too_many_arguments)]
pub fn kartu_kpi(
    p: &mut Page,
    x: f64,
    y: f64,
    w: f64,
    h: f64,
    label: &str,
    nilai: &str,
    hint: &str,
    tone: &str,
) {
    p.kotak(x, y, w, h, "#f6f8fd");
    p.kotak_tepi(x, y, w, h, GARIS, 0.7);
    p.kotak(x, y, 3.0, h, warna_tone(tone));
    p.teks(x + 10.0, y + 13.0, 6.8, true, REDUP, &label.to_uppercase());
    let uk = if lebar_teks(nilai, 15.0, true) > w - 18.0 {
        11.5
    } else {
        15.0
    };
    p.teks(x + 10.0, y + 31.0, uk, true, TINTA, nilai);
    p.teks(
        x + 10.0,
        y + 44.0,
        6.8,
        false,
        REDUP,
        &Page::potong(hint, w - 16.0, 6.8, false),
    );
}

/// Satu seri diagram kolom.
pub struct SeriPdf<'a> {
    pub nama: &'a str,
    pub warna: &'static str,
    pub nilai: Vec<f64>,
}

/// Diagram kolom bertumpuk dengan sumbu, total di atas kolom, garis acuan, dan legenda.
#[allow(clippy::too_many_arguments)]
pub fn kolom(
    p: &mut Page,
    x: f64,
    y: f64,
    w: f64,
    h: f64,
    label: &[String],
    seri: &[SeriPdf<'_>],
    fmt: &dyn Fn(f64) -> String,
    acuan: Option<(f64, &str)>,
) {
    let kiri = 46.0;
    let atas = 14.0;
    let bawah = if seri.len() > 1 { 30.0 } else { 16.0 };
    let n = label.len().max(1);
    let total: Vec<f64> = (0..n)
        .map(|i| {
            seri.iter()
                .map(|s| s.nilai.get(i).copied().unwrap_or(0.0).max(0.0))
                .sum()
        })
        .collect();
    let maks = total
        .iter()
        .copied()
        .fold(acuan.map(|a| a.0).unwrap_or(0.0), f64::max);
    let puncak = batas_bulat(maks);
    let pw = w - kiri - 6.0;
    let ph = h - atas - bawah;
    let x0 = x + kiri;
    let y0 = y + atas;
    let py = |v: f64| y0 + ph - (v / puncak).clamp(0.0, 1.0) * ph;
    for i in 0..=4 {
        let v = puncak * i as f64 / 4.0;
        p.garis(x0, py(v), x0 + pw, py(v), GARIS, 0.5);
        p.teks_kanan(x0 - 5.0, py(v) + 2.5, 6.5, false, REDUP, &fmt(v));
    }
    let lebar = pw / n as f64;
    let batang = (lebar * 0.6).min(46.0);
    for i in 0..label.len() {
        let bx = x0 + lebar * i as f64 + (lebar - batang) / 2.0;
        let mut dasar = 0.0;
        for s in seri {
            let v = s.nilai.get(i).copied().unwrap_or(0.0).max(0.0);
            if v <= 0.0 {
                continue;
            }
            let y1 = py(dasar + v);
            p.kotak(bx, y1, batang, (py(dasar) - y1).max(0.0), s.warna);
            dasar += v;
        }
        p.teks_tengah(
            bx + batang / 2.0,
            py(total[i]) - 3.0,
            6.8,
            true,
            TINTA,
            &fmt(total[i]),
        );
        p.teks_tengah(
            bx + batang / 2.0,
            y0 + ph + 10.0,
            6.8,
            false,
            REDUP,
            &Page::potong(&label[i], lebar - 2.0, 6.8, false),
        );
    }
    if let Some((v, teks)) = acuan {
        p.putus(true);
        p.garis(x0, py(v), x0 + pw, py(v), REDUP, 0.9);
        p.putus(false);
        p.teks_kanan(x0 + pw, py(v) - 3.0, 6.5, false, REDUP, teks);
    }
    if seri.len() > 1 {
        let mut lx = x0;
        for s in seri {
            p.kotak(lx, y + h - 9.0, 6.0, 6.0, s.warna);
            p.teks(lx + 9.0, y + h - 3.5, 6.8, false, TEKS, s.nama);
            lx += 16.0 + lebar_teks(s.nama, 6.8, false);
        }
    }
}

/// Riwayat tahunan dan prakiraan (garis putus) dengan pita rentang 80%.
#[allow(clippy::too_many_arguments)]
pub fn garis_ramal(
    p: &mut Page,
    x: f64,
    y: f64,
    w: f64,
    h: f64,
    riwayat: &[(i32, f64)],
    ramal: &[RamalTitik],
    fmt: &dyn Fn(f64) -> String,
) {
    if riwayat.len() < 2 {
        p.teks(
            x,
            y + h / 2.0,
            8.5,
            false,
            REDUP,
            "Riwayat belum cukup untuk digambar.",
        );
        return;
    }
    let kiri = 46.0;
    let atas = 8.0;
    let bawah = 16.0;
    let x_min = riwayat[0].0 as f64;
    let x_maks = ramal
        .last()
        .map(|r| r.tahun as f64)
        .unwrap_or(riwayat[riwayat.len() - 1].0 as f64);
    let maks = riwayat
        .iter()
        .map(|r| r.1)
        .chain(ramal.iter().map(|r| r.atas))
        .fold(0.0_f64, f64::max);
    let puncak = batas_bulat(maks);
    let pw = w - kiri - 8.0;
    let ph = h - atas - bawah;
    let x0 = x + kiri;
    let y0 = y + atas;
    let px = |t: f64| x0 + (t - x_min) / (x_maks - x_min).max(1.0) * pw;
    let py = |v: f64| y0 + ph - (v / puncak).clamp(0.0, 1.0) * ph;
    for i in 0..=4 {
        let v = puncak * i as f64 / 4.0;
        p.garis(x0, py(v), x0 + pw, py(v), GARIS, 0.5);
        p.teks_kanan(x0 - 5.0, py(v) + 2.5, 6.5, false, REDUP, &fmt(v));
    }
    let rentang = (x_maks - x_min).max(1.0);
    let langkah = [1.0, 2.0, 5.0, 10.0]
        .into_iter()
        .find(|l| rentang / l <= 10.0)
        .unwrap_or(20.0);
    let mut t = (x_min / langkah).ceil() * langkah;
    while t <= x_maks + 1e-9 {
        p.teks_tengah(px(t), y + h - 3.0, 6.5, false, REDUP, &format!("{t:.0}"));
        t += langkah;
    }
    let akhir = riwayat[riwayat.len() - 1];
    if !ramal.is_empty() {
        let mut pita: Vec<(f64, f64)> = vec![(px(akhir.0 as f64), py(akhir.1))];
        pita.extend(ramal.iter().map(|r| (px(r.tahun as f64), py(r.atas))));
        pita.extend(
            ramal
                .iter()
                .rev()
                .map(|r| (px(r.tahun as f64), py(r.bawah))),
        );
        p.poligon(&pita, "#e3e9fd");
    }
    let nyata: Vec<(f64, f64)> = riwayat.iter().map(|r| (px(r.0 as f64), py(r.1))).collect();
    p.garis_poli(&nyata, UTAMA, 1.6);
    for (cx, cy) in &nyata {
        p.kotak(cx - 1.6, cy - 1.6, 3.2, 3.2, UTAMA);
    }
    if !ramal.is_empty() {
        let mut depan: Vec<(f64, f64)> = vec![(px(akhir.0 as f64), py(akhir.1))];
        depan.extend(ramal.iter().map(|r| (px(r.tahun as f64), py(r.nilai))));
        p.putus(true);
        p.garis_poli(&depan, UTAMA, 1.6);
        p.putus(false);
        for (cx, cy) in depan.iter().skip(1) {
            p.kotak(cx - 1.6, cy - 1.6, 3.2, 3.2, "#ffffff");
            p.kotak_tepi(cx - 1.6, cy - 1.6, 3.2, 3.2, UTAMA, 0.9);
        }
    }
}

/// Batang 100% sebaran kondisi.
#[allow(clippy::too_many_arguments)]
pub fn bilah_kondisi(
    p: &mut Page,
    x: f64,
    y: f64,
    w: f64,
    h: f64,
    baik: i64,
    rr: i64,
    rb: i64,
    tak: i64,
) {
    let total = (baik + rr + rb + tak).max(1) as f64;
    p.kotak(x, y, w, h, TRACK);
    let mut cx = x;
    for (v, warna) in [(baik, C_BAIK), (rr, C_RR), (rb, C_RB), (tak, C_TAK)] {
        let lw = w * v as f64 / total;
        if lw > 0.0 {
            p.kotak(cx, y, lw, h, warna);
            cx += lw;
        }
    }
}

/// Batang nilai mendatar (0–100%) untuk komponen skor.
pub fn bilah_nilai(p: &mut Page, x: f64, y: f64, w: f64, h: f64, persen: f64, warna: &str) {
    p.kotak(x, y, w, h, TRACK);
    p.kotak(x, y, w * (persen / 100.0).clamp(0.0, 1.0), h, warna);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::report::pdf::{Dokumen, A4_H, A4_W};

    #[test]
    fn grafik_menggambar_tanpa_panik() {
        let mut d = Dokumen::new("Uji Grafik");
        let mut p = Page::new(A4_W, A4_H);
        kartu_kpi(
            &mut p, 40.0, 70.0, 120.0, 52.0, "Total", "1.234", "unit", "blue",
        );
        let label: Vec<String> = (2026..2032).map(|y| y.to_string()).collect();
        kolom(
            &mut p,
            40.0,
            140.0,
            500.0,
            150.0,
            &label,
            &[
                SeriPdf {
                    nama: "A",
                    warna: C_BAIK,
                    nilai: vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0],
                },
                SeriPdf {
                    nama: "B",
                    warna: C_RB,
                    nilai: vec![1.0; 6],
                },
            ],
            &|v| format!("{v:.0}"),
            Some((4.0, "acuan")),
        );
        // Deret kosong, satu titik, dan nilai nol tidak boleh panik.
        kolom(
            &mut p,
            40.0,
            300.0,
            500.0,
            100.0,
            &[],
            &[],
            &|v| format!("{v:.0}"),
            None,
        );
        garis_ramal(
            &mut p,
            40.0,
            420.0,
            500.0,
            120.0,
            &[(2020, 1.0)],
            &[],
            &|v| format!("{v:.0}"),
        );
        garis_ramal(
            &mut p,
            40.0,
            560.0,
            500.0,
            120.0,
            &[(2020, 0.0), (2021, 0.0), (2022, 0.0)],
            &[RamalTitik {
                tahun: 2023,
                nilai: 0.0,
                bawah: 0.0,
                atas: 0.0,
            }],
            &|v| format!("{v:.0}"),
        );
        bilah_kondisi(&mut p, 40.0, 700.0, 300.0, 8.0, 0, 0, 0, 0);
        bilah_nilai(&mut p, 40.0, 720.0, 300.0, 6.0, 150.0, C_BAIK);
        d.tambah(p);
        let b = d.bangun("D:20260101120000Z");
        assert!(String::from_utf8_lossy(&b).starts_with("%PDF-1.4"));
    }
}
