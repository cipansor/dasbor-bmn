//! Prakiraan deret tahunan (jumlah dan nilai perolehan).
//!
//! Tiga metode sederhana diadu lewat uji balik berjalan (meramal satu tahun ke
//! depan dari data sebelum tahun itu, diulang untuk beberapa tahun terakhir).
//! Metode dengan galat terkecil dipakai, dan galatnya ikut dilaporkan supaya
//! pembaca tahu seberapa jauh prakiraan dapat dipegang.

#![cfg(feature = "ssr")]

use crate::dto::{Deret, RamalTitik};

/// Banyaknya tahun riwayat paling akhir yang dipakai.
const JENDELA: usize = 15;
/// Kuantil normal untuk rentang 80%.
const Z80: f64 = 1.2816;

#[derive(Debug, Clone, Copy, PartialEq)]
enum Metode {
    Rata3,
    Linear,
    Holt,
}

impl Metode {
    fn label(&self) -> &'static str {
        match self {
            Metode::Rata3 => "Rata-rata 3 tahun terakhir",
            Metode::Linear => "Tren linear",
            Metode::Holt => "Pemulusan eksponensial (Holt teredam)",
        }
    }
}

fn rata3(y: &[f64], langkah: usize) -> Vec<f64> {
    let k = y.len().min(3).max(1);
    let m = y[y.len() - k..].iter().sum::<f64>() / k as f64;
    vec![m; langkah]
}

fn linear(y: &[f64], langkah: usize) -> Vec<f64> {
    let k = y.len().min(10);
    let y = &y[y.len() - k..];
    let n = k as f64;
    let mx = (n - 1.0) / 2.0;
    let my = y.iter().sum::<f64>() / n;
    let (mut sxy, mut sxx) = (0.0, 0.0);
    for (i, v) in y.iter().enumerate() {
        sxy += (i as f64 - mx) * (v - my);
        sxx += (i as f64 - mx).powi(2);
    }
    let b = if sxx > 0.0 { sxy / sxx } else { 0.0 };
    let a = my - b * mx;
    (0..langkah)
        .map(|h| (a + b * (n + h as f64)).max(0.0))
        .collect()
}

fn holt_dengan(y: &[f64], alpha: f64, beta: f64, phi: f64) -> (f64, f64, f64) {
    let mut l = y[0];
    let mut b = if y.len() > 1 { y[1] - y[0] } else { 0.0 };
    let mut sse = 0.0;
    for v in &y[1..] {
        let ramal = l + phi * b;
        sse += (v - ramal).powi(2);
        let l_baru = alpha * v + (1.0 - alpha) * ramal;
        b = beta * (l_baru - l) + (1.0 - beta) * phi * b;
        l = l_baru;
    }
    (l, b, sse)
}

fn holt(y: &[f64], langkah: usize) -> Vec<f64> {
    const PHI: f64 = 0.85;
    let mut terbaik = (f64::INFINITY, y[y.len() - 1], 0.0);
    for alpha in [0.2, 0.4, 0.6, 0.8] {
        for beta in [0.1, 0.3, 0.5] {
            let (l, b, sse) = holt_dengan(y, alpha, beta, PHI);
            if sse < terbaik.0 {
                terbaik = (sse, l, b);
            }
        }
    }
    let (_, l, b) = terbaik;
    let mut redam = 0.0;
    (0..langkah)
        .map(|h| {
            redam += PHI.powi(h as i32 + 1);
            (l + redam * b).max(0.0)
        })
        .collect()
}

fn jalankan(m: Metode, y: &[f64], langkah: usize) -> Vec<f64> {
    match m {
        Metode::Rata3 => rata3(y, langkah),
        Metode::Linear => linear(y, langkah),
        Metode::Holt => holt(y, langkah),
    }
}

/// Uji balik berjalan satu langkah: (sMAPE persen, akar rata-rata kuadrat galat).
fn uji_balik(m: Metode, y: &[f64]) -> Option<(f64, f64)> {
    let n = y.len();
    if n < 5 {
        return None;
    }
    let putaran = (n - 4).min(4);
    let (mut smape, mut kuadrat) = (0.0, 0.0);
    for o in (n - putaran)..n {
        let f = jalankan(m, &y[..o], 1)[0];
        let a = y[o];
        let penyebut = f.abs() + a.abs();
        smape += if penyebut > 0.0 {
            200.0 * (f - a).abs() / penyebut
        } else {
            0.0
        };
        kuadrat += (f - a).powi(2);
    }
    Some((
        smape / putaran as f64,
        (kuadrat / putaran as f64).sqrt(),
    ))
}

/// Ramal `langkah` tahun ke depan dari riwayat tahunan (tahun menaik, tahun penuh).
pub fn ramal(riwayat: &[(i32, f64)], langkah: usize) -> Deret {
    let riwayat: Vec<(i32, f64)> = if riwayat.len() > JENDELA {
        riwayat[riwayat.len() - JENDELA..].to_vec()
    } else {
        riwayat.to_vec()
    };
    if riwayat.is_empty() {
        return Deret {
            metode: "Belum ada riwayat".into(),
            galat: -1.0,
            ..Default::default()
        };
    }
    let y: Vec<f64> = riwayat.iter().map(|p| p.1).collect();
    let tahun_akhir = riwayat[riwayat.len() - 1].0;

    let mut pilihan = (Metode::Rata3, None::<(f64, f64)>);
    for m in [Metode::Rata3, Metode::Linear, Metode::Holt] {
        if let Some(hasil) = uji_balik(m, &y) {
            if pilihan.1.is_none_or(|p| hasil.0 < p.0 - 1e-9) {
                pilihan = (m, Some(hasil));
            }
        }
    }
    let (metode, uji) = pilihan;
    let titik = jalankan(metode, &y, langkah);
    // Tanpa uji balik, simpangan diambil dari perubahan antartahun.
    let sigma = match uji {
        Some((_, rmse)) => rmse,
        None => {
            if y.len() > 1 {
                let d: Vec<f64> = y.windows(2).map(|w| w[1] - w[0]).collect();
                (d.iter().map(|x| x * x).sum::<f64>() / d.len() as f64).sqrt()
            } else {
                0.5 * y[0].abs()
            }
        }
    };
    let ramal = titik
        .iter()
        .enumerate()
        .map(|(h, v)| {
            let lebar = Z80 * sigma * ((h + 1) as f64).sqrt();
            RamalTitik {
                tahun: tahun_akhir + h as i32 + 1,
                nilai: *v,
                bawah: (v - lebar).max(0.0),
                atas: v + lebar,
            }
        })
        .collect();
    Deret {
        metode: metode.label().to_string(),
        galat: uji.map(|u| u.0).unwrap_or(-1.0),
        riwayat,
        ramal,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn deret(v: &[f64]) -> Vec<(i32, f64)> {
        v.iter()
            .enumerate()
            .map(|(i, x)| (2010 + i as i32, *x))
            .collect()
    }

    #[test]
    fn tren_naik_dilanjutkan() {
        let d = ramal(&deret(&[10., 12., 14., 16., 18., 20., 22., 24., 26., 28.]), 5);
        assert_eq!(d.ramal.len(), 5);
        assert_eq!(d.ramal[0].tahun, 2020);
        assert!((d.ramal[0].nilai - 30.0).abs() < 1.5, "{:?}", d.ramal[0]);
        assert!(d.ramal[4].nilai > d.ramal[0].nilai);
        assert!(d.galat >= 0.0 && d.galat < 5.0, "galat {}", d.galat);
        assert_ne!(d.metode, "Rata-rata 3 tahun terakhir");
    }

    #[test]
    fn deret_datar_tetap_datar() {
        let d = ramal(&deret(&[50.0; 8]), 5);
        for t in &d.ramal {
            assert!((t.nilai - 50.0).abs() < 1e-6);
            assert!((t.atas - t.bawah).abs() < 1e-6, "tanpa variasi, rentang nol");
        }
        assert!(d.galat.abs() < 1e-9);
    }

    #[test]
    fn rentang_melebar_dan_tidak_negatif() {
        let d = ramal(&deret(&[5., 40., 3., 30., 8., 45., 2., 38., 6., 1.]), 5);
        for w in d.ramal.windows(2) {
            assert!(
                w[1].atas - w[1].bawah >= w[0].atas - w[0].bawah - 1e-9,
                "ketidakpastian membesar makin jauh"
            );
        }
        assert!(d.ramal.iter().all(|t| t.bawah >= 0.0 && t.nilai >= 0.0));
        assert!(d.ramal.iter().all(|t| t.bawah <= t.nilai && t.nilai <= t.atas));
    }

    #[test]
    fn riwayat_pendek_dan_kosong() {
        let d = ramal(&deret(&[10., 20.]), 3);
        assert_eq!(d.ramal.len(), 3);
        assert!(d.galat < 0.0, "uji balik tidak mungkin dengan dua titik");
        assert!((d.ramal[0].nilai - 15.0).abs() < 1e-9);
        let kosong = ramal(&[], 5);
        assert!(kosong.ramal.is_empty());
        assert!(kosong.riwayat.is_empty());
    }

    #[test]
    fn hanya_jendela_terakhir_dipakai() {
        let mut v = vec![1000.0; 10];
        v.extend([10.0; 15]);
        let d = ramal(&deret(&v), 2);
        assert_eq!(d.riwayat.len(), 15);
        assert!((d.ramal[0].nilai - 10.0).abs() < 1e-6);
    }
}
