//! Model kerusakan aset dua tahap: baik → rusak ringan → rusak berat.
//!
//! Data SIMAN hanya memberi potret satu waktu: umur tiap aset dan kondisinya
//! hari ini. Dari potret itu ditaksir dua hal:
//!
//! 1. **Kapan aset mulai tidak baik.** Peluang aset masih baik pada umur `a`
//!    mengikuti kurva Weibull `S(a) = exp(-(a/η)^β)`. Bila β > 1, laju kerusakan
//!    naik seiring umur (aus), sesuai perilaku peralatan dan bangunan.
//! 2. **Berapa lama bertahan di rusak ringan.** Setelah tidak baik, aset
//!    berpindah ke rusak berat dengan laju tetap μ per tahun (rata-rata 1/μ tahun).
//!
//! Parameter ditaksir dengan kemungkinan maksimum atas hitungan kondisi per umur
//! (data *current status*). Proyeksi ke depan memakai kondisi dan umur tiap aset
//! saat ini, sehingga aset rusak ringan dan aset tua mendapat risiko lebih besar.
//!
//! Keterbatasan yang perlu diingat pembaca: aset rusak berat yang sudah dihapus
//! tidak lagi tercatat, sehingga porsi rusak berat pada umur tua cenderung
//! terlihat lebih kecil dari kenyataan dan proyeksi bersifat konservatif.

#![cfg(feature = "ssr")]

/// Parameter model kerusakan satu kategori.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Param {
    /// Skala Weibull (tahun).
    pub eta: f64,
    /// Bentuk Weibull.
    pub beta: f64,
    /// Laju perpindahan rusak ringan → rusak berat per tahun.
    pub mu: f64,
}

/// Hitungan kondisi pada satu umur (tahun bulat).
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Hitung {
    pub n: f64,
    pub baik: f64,
    pub rb: f64,
}

/// Sampel minimum agar parameter kategori ditaksir sendiri.
pub const SAMPEL_MIN: f64 = 80.0;
/// Kejadian minimum (aset tidak baik) agar taksiran bermakna.
pub const KEJADIAN_MIN: f64 = 8.0;

const EPS: f64 = 1e-9;

fn jepit(p: f64) -> f64 {
    p.clamp(EPS, 1.0 - EPS)
}

impl Param {
    /// Parameter cadangan dari masa manfaat ketika data tidak cukup.
    pub fn bawaan(masa_manfaat: f64) -> Param {
        let masa = if masa_manfaat > 0.0 { masa_manfaat } else { 10.0 };
        Param {
            eta: 1.7 * masa,
            beta: 2.3,
            mu: (1.0 / (0.45 * masa)).clamp(0.05, 0.5),
        }
    }

    /// Peluang aset masih baik pada umur `a`; `phi` adalah faktor kerapuhan.
    pub fn s_baik(&self, a: f64, phi: f64) -> f64 {
        (-phi * (a.max(0.0) / self.eta).powf(self.beta)).exp()
    }

    /// Median umur saat aset mulai tidak baik.
    pub fn median_mulai(&self) -> f64 {
        self.eta * std::f64::consts::LN_2.powf(1.0 / self.beta)
    }

    /// Rata-rata lama di kondisi rusak ringan sebelum rusak berat.
    pub fn lama_rr(&self) -> f64 {
        1.0 / self.mu
    }

    /// Peluang rusak berat dalam `t` tahun bagi aset yang kini rusak ringan.
    pub fn rb_dari_rr(&self, t: f64) -> f64 {
        1.0 - (-self.mu * t.max(0.0)).exp()
    }

    /// Lintasan peluang bagi aset yang kini **baik** pada umur `a`.
    ///
    /// Mengembalikan, untuk tiap batas waktu pada `batas` (tahun dari sekarang,
    /// menaik), pasangan (peluang tidak baik, peluang rusak berat).
    pub fn lintasan_baik(&self, a: f64, phi: f64, batas: &[f64]) -> Vec<(f64, f64)> {
        let mut out = Vec::with_capacity(batas.len());
        let s0 = self.s_baik(a, phi).max(1e-300);
        let (mut baik, mut rr, mut rb) = (1.0_f64, 0.0_f64, 0.0_f64);
        let mut t = 0.0_f64;
        for &akhir in batas {
            let rentang = (akhir - t).max(0.0);
            // Langkah ≤ 0,25 tahun menjaga galat integrasi jauh di bawah 1%.
            let langkah = ((rentang / 0.25).ceil() as usize).max(1);
            let d = rentang / langkah as f64;
            for _ in 0..langkah {
                let s_awal = self.s_baik(a + t, phi) / s0;
                let s_akhir = self.s_baik(a + t + d, phi) / s0;
                let keluar = if s_awal > 0.0 {
                    baik * (1.0 - s_akhir / s_awal)
                } else {
                    baik
                };
                let lanjut = 1.0 - (-self.mu * d).exp();
                let lanjut_baru = 1.0 - (-self.mu * d * 0.5).exp();
                rb += rr * lanjut + keluar * lanjut_baru;
                rr = rr * (1.0 - lanjut) + keluar * (1.0 - lanjut_baru);
                baik -= keluar;
                t += d;
            }
            out.push(((rr + rb).clamp(0.0, 1.0), rb.clamp(0.0, 1.0)));
        }
        out
    }

    /// Peluang rusak berat pada umur `a` bagi aset yang semula baik.
    pub fn p_rb(&self, a: f64, phi: f64) -> f64 {
        if a <= 0.0 {
            return 0.0;
        }
        self.lintasan_baik(0.0, phi, &[a])[0].1
    }
}

fn kemungkinan_baik(data: &[Hitung], eta: f64, beta: f64) -> f64 {
    let p = Param {
        eta,
        beta,
        mu: 0.2,
    };
    let mut ll = 0.0;
    for (i, h) in data.iter().enumerate() {
        if h.n <= 0.0 {
            continue;
        }
        let s = jepit(p.s_baik(i as f64 + 0.5, 1.0));
        ll += h.baik * s.ln() + (h.n - h.baik) * (1.0 - s).ln();
    }
    ll
}

fn kemungkinan_rb(data: &[Hitung], p: &Param) -> f64 {
    let mut ll = 0.0;
    for (i, h) in data.iter().enumerate() {
        if h.n <= 0.0 {
            continue;
        }
        let q = jepit(p.p_rb(i as f64 + 0.5, 1.0));
        ll += h.rb * q.ln() + (h.n - h.rb) * (1.0 - q).ln();
    }
    ll
}

/// Taksir parameter dari hitungan kondisi per umur (indeks = umur tahun bulat).
///
/// Mengembalikan `None` bila sampel atau kejadian terlalu sedikit.
pub fn taksir(data: &[Hitung]) -> Option<Param> {
    let n: f64 = data.iter().map(|h| h.n).sum();
    let tidak_baik: f64 = data.iter().map(|h| h.n - h.baik).sum();
    if n < SAMPEL_MIN || tidak_baik < KEJADIAN_MIN {
        return None;
    }

    // Tahap 1: (η, β) dari porsi baik per umur — telusur kisi lalu perhalus.
    let (mut ln_eta_lo, mut ln_eta_hi) = (1.0_f64.ln(), 600.0_f64.ln());
    let (mut beta_lo, mut beta_hi) = (0.5_f64, 6.0_f64);
    let mut terbaik = (f64::NEG_INFINITY, 20.0_f64, 2.0_f64);
    for _ in 0..5 {
        const N_ETA: usize = 36;
        const N_BETA: usize = 24;
        for i in 0..=N_ETA {
            let ln_eta = ln_eta_lo + (ln_eta_hi - ln_eta_lo) * i as f64 / N_ETA as f64;
            for j in 0..=N_BETA {
                let beta = beta_lo + (beta_hi - beta_lo) * j as f64 / N_BETA as f64;
                let ll = kemungkinan_baik(data, ln_eta.exp(), beta);
                if ll > terbaik.0 {
                    terbaik = (ll, ln_eta.exp(), beta);
                }
            }
        }
        let lebar_eta = (ln_eta_hi - ln_eta_lo) / 6.0;
        let lebar_beta = (beta_hi - beta_lo) / 6.0;
        ln_eta_lo = (terbaik.1.ln() - lebar_eta).max(0.0);
        ln_eta_hi = terbaik.1.ln() + lebar_eta;
        beta_lo = (terbaik.2 - lebar_beta).max(0.3);
        beta_hi = terbaik.2 + lebar_beta;
    }
    let (_, eta, beta) = terbaik;

    // Tahap 2: μ dari porsi rusak berat per umur.
    let (mut lo, mut hi) = (0.003_f64.ln(), 4.0_f64.ln());
    let mut mu_terbaik = (f64::NEG_INFINITY, 0.2_f64);
    for _ in 0..4 {
        const N: usize = 28;
        for i in 0..=N {
            let mu = (lo + (hi - lo) * i as f64 / N as f64).exp();
            let ll = kemungkinan_rb(data, &Param { eta, beta, mu });
            if ll > mu_terbaik.0 {
                mu_terbaik = (ll, mu);
            }
        }
        let lebar = (hi - lo) / 6.0;
        lo = mu_terbaik.1.ln() - lebar;
        hi = mu_terbaik.1.ln() + lebar;
    }
    Some(Param {
        eta,
        beta,
        mu: mu_terbaik.1,
    })
}

/// Rata-rata selisih mutlak model terhadap data (poin persentase), berbobot sampel.
pub fn mae(data: &[Hitung], p: &Param) -> f64 {
    let (mut jumlah, mut bobot) = (0.0, 0.0);
    for (i, h) in data.iter().enumerate() {
        if h.n <= 0.0 {
            continue;
        }
        let a = i as f64 + 0.5;
        let obs_tb = (h.n - h.baik) / h.n;
        let obs_rb = h.rb / h.n;
        jumlah += h.n * ((obs_tb - (1.0 - p.s_baik(a, 1.0))).abs() + (obs_rb - p.p_rb(a, 1.0)).abs())
            / 2.0;
        bobot += h.n;
    }
    if bobot > 0.0 {
        100.0 * jumlah / bobot
    } else {
        0.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::demo::Rng;

    fn simulasi(p: &Param, per_umur: usize, umur_maks: usize, benih: u64) -> Vec<Hitung> {
        let mut rng = Rng(benih);
        let mut data = vec![Hitung::default(); umur_maks];
        for (a, h) in data.iter_mut().enumerate() {
            for _ in 0..per_umur {
                let umur = a as f64 + rng.f();
                let t1 = p.eta * (-(rng.f().ln())).powf(1.0 / p.beta);
                h.n += 1.0;
                if t1 >= umur {
                    h.baik += 1.0;
                } else if t1 + (-(rng.f().ln())) / p.mu < umur {
                    h.rb += 1.0;
                }
            }
        }
        data
    }

    #[test]
    fn peluang_dasar_masuk_akal() {
        let p = Param {
            eta: 10.0,
            beta: 2.5,
            mu: 0.3,
        };
        assert!((p.s_baik(0.0, 1.0) - 1.0).abs() < 1e-12);
        assert!(p.s_baik(5.0, 1.0) > p.s_baik(10.0, 1.0));
        // Kerapuhan lebih tinggi berarti lebih cepat tidak baik.
        assert!(p.s_baik(8.0, 2.0) < p.s_baik(8.0, 1.0));
        assert!((p.s_baik(p.median_mulai(), 1.0) - 0.5).abs() < 1e-9);
        assert!((p.rb_dari_rr(0.0)).abs() < 1e-12);
        assert!((p.rb_dari_rr(1.0 / 0.3) - (1.0 - (-1.0_f64).exp())).abs() < 1e-9);
    }

    #[test]
    fn lintasan_monoton_dan_konsisten() {
        let p = Param {
            eta: 10.0,
            beta: 2.5,
            mu: 0.3,
        };
        let batas = [0.2, 1.2, 2.2, 3.2, 4.2, 5.2];
        let l = p.lintasan_baik(6.0, 1.0, &batas);
        for w in l.windows(2) {
            assert!(w[1].0 >= w[0].0 && w[1].1 >= w[0].1, "peluang kumulatif naik");
        }
        for (i, (tb, rb)) in l.iter().enumerate() {
            assert!(rb <= tb, "rusak berat bagian dari tidak baik");
            // Peluang tidak baik sama dengan rumus tertutup.
            let tutup = 1.0 - p.s_baik(6.0 + batas[i], 1.0) / p.s_baik(6.0, 1.0);
            assert!((tb - tutup).abs() < 1e-9);
        }
        // Aset tua lebih berisiko daripada aset muda pada cakrawala yang sama.
        let muda = p.lintasan_baik(1.0, 1.0, &[5.0])[0].1;
        let tua = p.lintasan_baik(12.0, 1.0, &[5.0])[0].1;
        assert!(tua > muda * 3.0, "muda {muda}, tua {tua}");
    }

    #[test]
    fn integrasi_cocok_dengan_rumus_tertutup() {
        // Untuk β = 1 (laju tetap) peluang rusak berat punya rumus tertutup.
        let p = Param {
            eta: 8.0,
            beta: 1.0,
            mu: 0.25,
        };
        let l1: f64 = 1.0 / 8.0;
        let l2: f64 = 0.25;
        for a in [2.0_f64, 6.0, 15.0] {
            let tutup = 1.0 - (l2 * (-l1 * a).exp() - l1 * (-l2 * a).exp()) / (l2 - l1);
            assert!(
                (p.p_rb(a, 1.0) - tutup).abs() < 0.004,
                "umur {a}: {} vs {tutup}",
                p.p_rb(a, 1.0)
            );
        }
    }

    #[test]
    fn taksiran_memulihkan_parameter_simulasi() {
        let asli = Param {
            eta: 9.0,
            beta: 2.4,
            mu: 0.35,
        };
        let data = simulasi(&asli, 400, 22, 42);
        let p = taksir(&data).expect("sampel cukup");
        assert!((p.eta - asli.eta).abs() / asli.eta < 0.08, "eta {}", p.eta);
        assert!((p.beta - asli.beta).abs() / asli.beta < 0.12, "beta {}", p.beta);
        assert!((p.mu - asli.mu).abs() / asli.mu < 0.15, "mu {}", p.mu);
        assert!(mae(&data, &p) < 2.0, "mae {}", mae(&data, &p));
    }

    #[test]
    fn taksiran_ditolak_bila_sampel_kurang() {
        let asli = Param {
            eta: 9.0,
            beta: 2.4,
            mu: 0.35,
        };
        assert!(
            taksir(&simulasi(&asli, 4, 10, 7)).is_none(),
            "40 aset tidak cukup"
        );
        // Semua baik: tidak ada kejadian untuk ditaksir.
        let semua_baik = vec![
            Hitung {
                n: 100.0,
                baik: 100.0,
                rb: 0.0
            };
            5
        ];
        assert!(taksir(&semua_baik).is_none());
    }

    #[test]
    fn parameter_bawaan_mengikuti_masa_manfaat() {
        let tik = Param::bawaan(4.0);
        let gedung = Param::bawaan(50.0);
        assert!(tik.median_mulai() < gedung.median_mulai());
        assert!(tik.mu > gedung.mu);
        assert!(Param::bawaan(0.0).eta > 0.0);
    }
}
