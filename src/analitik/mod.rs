//! Analitik BMN: model kerusakan, proyeksi, analisis kebutuhan, prakiraan, dan
//! rekomendasi. Modul ini menjembatani mesin hitung murni dengan penyimpanan
//! lokal dan menyimpan hasil terakhir agar halaman tidak menghitung ulang.

#![cfg(feature = "ssr")]

pub mod kerusakan;
pub mod mesin;
pub mod prakiraan;
pub mod rekomendasi;

use crate::catalog::{self, TABLES};
use crate::dto::*;
use crate::fakta::{F_DIHAPUS, K_RB};
use crate::model::JenisKebutuhan;
use crate::store;
pub use mesin::{Fakta, Hasil, Kamus, Konteks};
use std::sync::{Arc, Mutex};

static TEMBOLOK: Mutex<Option<Arc<Hasil>>> = Mutex::new(None);
static KUNCI_HITUNG: Mutex<()> = Mutex::new(());

fn tersimpan(versi: i64, asumsi: &Asumsi) -> Option<Arc<Hasil>> {
    TEMBOLOK
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .as_ref()
        .filter(|h| h.versi == versi && &h.konteks.asumsi == asumsi)
        .cloned()
}

/// Hasil analitik terkini; dihitung ulang hanya bila data atau asumsi berubah.
pub fn hasil() -> Arc<Hasil> {
    crate::demo::pastikan();
    let asumsi = crate::config::asumsi();
    if let Some(h) = tersimpan(store::versi_data(), &asumsi) {
        return h;
    }
    // Hanya satu penghitung; permintaan lain menunggu lalu memakai hasilnya.
    let _k = KUNCI_HITUNG.lock().unwrap_or_else(|e| e.into_inner());
    let versi = store::versi_data();
    if let Some(h) = tersimpan(versi, &asumsi) {
        return h;
    }
    let h = Arc::new(segarkan(versi, asumsi));
    *TEMBOLOK.lock().unwrap_or_else(|e| e.into_inner()) = Some(h.clone());
    h
}

/// Tahun dan bulan dari teks waktu `YYYY-MM-DD …`.
fn tahun_bulan(s: &str) -> Option<(i32, u32)> {
    let y: i32 = s.get(0..4)?.parse().ok()?;
    let m: u32 = s.get(5..7)?.parse().ok()?;
    ((2000..=2100).contains(&y) && (1..=12).contains(&m)).then_some((y, m))
}

fn lengkapi_identitas(daftar: &mut [&mut Vec<AsetRisiko>]) {
    let ids: Vec<i64> = daftar.iter().flat_map(|d| d.iter().map(|a| a.id)).collect();
    let idn = store::identitas(&ids);
    for d in daftar.iter_mut() {
        for a in d.iter_mut() {
            if let Some(i) = idn.get(&a.id) {
                a.nama = if i.merk.trim().is_empty() || i.merk.trim() == "-" {
                    i.nama.clone()
                } else {
                    format!("{} ({})", i.nama, i.merk)
                };
                a.kd_brg = i.kd_brg.clone();
                a.no_aset = i.no_aset.clone();
            }
        }
    }
}

fn segarkan(versi: i64, asumsi: Asumsi) -> Hasil {
    let status = store::sync_statuses();
    let terakhir = status.iter().filter_map(|s| s.2.clone()).max();
    let (tahun0, bulan0) = terakhir
        .as_deref()
        .and_then(tahun_bulan)
        .unwrap_or_else(crate::tanggal_kini);
    let konteks = Konteks {
        tahun0,
        bulan0,
        asumsi,
        is_demo: store::is_demo(),
        tanggal_data: terakhir.unwrap_or_default(),
        dibuat: crate::now_string(),
        tabel_tersinkron: status.len() as i64,
        kunci_ganda: store::kunci_ganda(),
    };

    let mut kamus = Kamus::default();
    let mut fakta: Vec<Fakta> = Vec::new();
    let _ = store::alir_fakta(None, |f| {
        let Some(tabel) = catalog::indeks(f.tabel) else {
            return;
        };
        fakta.push(Fakta {
            id: f.id,
            tabel,
            satker: kamus.satker(f.kode_satker, f.nama_satker),
            prov: kamus.prov(f.prov),
            kelompok: kamus.kelompok(f.kelompok),
            kondisi: f.kondisi,
            nilai: f.nilai,
            buku: f.buku,
            tahun: f.tahun as i32,
            bulan: f.bulan as u32,
            umur_sisa: f.umur_sisa,
            flags: f.flags,
        });
    });

    let mut h = mesin::hitung(&fakta, &kamus, &konteks);
    h.versi = versi;
    {
        let a = &mut h.analisis;
        lengkapi_identitas(&mut [&mut a.aset_rb, &mut a.aset_risiko]);
    }

    // Catat posisi harian dan bandingkan dengan posisi sebelumnya.
    let r = &h.analisis.nasional;
    if r.unit > 0 {
        let tanggal: String = h.konteks.tanggal_data.chars().take(10).collect();
        let kini = serde_json::json!({
            "unit": r.unit, "nilai": r.nilai, "rb": r.rb,
            "habis": r.habis, "idle": r.idle, "hilang": r.hilang,
        });
        if let Some((tgl, data)) = store::riwayat_sebelum(&tanggal) {
            if let Ok(lama) = serde_json::from_str::<serde_json::Value>(&data) {
                let i = |k: &str| lama.get(k).and_then(|v| v.as_i64()).unwrap_or(0);
                h.analisis.delta = Some(Delta {
                    tanggal: tgl,
                    unit: r.unit - i("unit"),
                    nilai: r.nilai - lama.get("nilai").and_then(|v| v.as_f64()).unwrap_or(0.0),
                    rb: r.rb - i("rb"),
                    habis: r.habis - i("habis"),
                    idle: r.idle - i("idle"),
                    hilang: r.hilang - i("hilang"),
                });
            }
        }
        if tanggal.len() == 10 {
            let _ = store::simpan_riwayat(&tanggal, &kini.to_string());
        }
    }
    h
}

/// Rincian satu satuan kerja beserta daftar aset prioritasnya.
pub fn rinci_satker(kode: &str) -> Option<SatkerRinci> {
    let h = hasil();
    let s = h.simpan.get(kode)?;
    let (mut rb, mut risiko, mut habis) = (Vec::new(), Vec::new(), Vec::new());
    let _ = store::alir_fakta(Some(kode), |f| {
        let Some(tabel) = catalog::indeks(f.tabel) else {
            return;
        };
        if f.flags & F_DIHAPUS != 0 {
            return;
        }
        let fk = Fakta {
            id: f.id,
            tabel,
            satker: 0,
            prov: 0,
            kelompok: 0,
            kondisi: f.kondisi,
            nilai: f.nilai,
            buku: f.buku,
            tahun: f.tahun as i32,
            bulan: f.bulan as u32,
            umur_sisa: f.umur_sisa,
            flags: f.flags,
        };
        let t = mesin::turunkan(&fk, f.kelompok, kode, &h.model, &h.konteks);
        let a = mesin::aset_dari(&fk, &t, f.kelompok, &s.nama, kode);
        let def = &TABLES[tabel];
        if def.fisik && f.kondisi == K_RB {
            rb.push(a);
        } else {
            if t.habis && def.kebutuhan != JenisKebutuhan::TidakAda {
                habis.push(a.clone());
            }
            if t.diproyeksikan && a.p_rb >= 0.2 {
                risiko.push(a);
            }
        }
    });
    rb.sort_by(|a, b| b.nilai.total_cmp(&a.nilai).then(a.id.cmp(&b.id)));
    rb.truncate(15);
    risiko.sort_by(|a, b| {
        b.p_rb
            .total_cmp(&a.p_rb)
            .then(b.nilai.total_cmp(&a.nilai))
            .then(a.id.cmp(&b.id))
    });
    risiko.truncate(15);
    habis.sort_by(|a, b| b.nilai.total_cmp(&a.nilai).then(a.id.cmp(&b.id)));
    habis.truncate(15);
    lengkapi_identitas(&mut [&mut rb, &mut risiko, &mut habis]);

    let kategori: Vec<KategoriBaris> = s
        .per_tabel
        .iter()
        .map(|(t, r)| {
            let def = &TABLES[*t];
            KategoriBaris {
                tabel: def.table.into(),
                label: def.label.into(),
                icon: def.icon.into(),
                fisik: def.fisik,
                disusutkan: def.disusutkan,
                kebutuhan: def.kebutuhan.label().into(),
                masa_manfaat: (h.model.masa[*t] * 10.0).round() / 10.0,
                r: r.clone(),
            }
        })
        .collect();
    let meta = h.analisis.meta.clone();
    let rekomendasi =
        rekomendasi::untuk_satker(&s.r, &meta, &kategori, s.r.unit >= 20 && s.r.rku() >= 1.4);
    Some(SatkerRinci {
        meta,
        kode: s.kode.clone(),
        nama: s.nama.clone(),
        prov: s.prov.clone(),
        peringkat: s.peringkat,
        dari: h.simpan.len() as i64,
        skor: s.skor.clone(),
        skor_nasional: h.analisis.skor.total,
        r: s.r.clone(),
        nasional_pct_baik: h.analisis.nasional.pct_baik(),
        nasional_pct_habis: h.analisis.nasional.pct_habis(),
        kategori,
        aset_rb: rb,
        aset_risiko: risiko,
        aset_habis: habis,
        rekomendasi,
    })
}

/// Perolehan pada bulan pelaporan dan sepanjang tahun yang sama.
pub fn aktivitas(tahun: i32, bulan: u32, label: &str) -> AktivitasPeriode {
    let sebulan = store::perolehan_periode(tahun as i64, Some(bulan as i64));
    let setahun = store::perolehan_periode(tahun as i64, None);
    let mut per_kategori: Vec<(String, i64, f64)> = sebulan
        .iter()
        .map(|(t, n, v)| (catalog::label_of(t), *n, *v))
        .collect();
    per_kategori.sort_by(|a, b| b.2.total_cmp(&a.2));
    AktivitasPeriode {
        label: label.to_string(),
        unit_bulan: sebulan.iter().map(|x| x.1).sum(),
        nilai_bulan: sebulan.iter().map(|x| x.2).sum(),
        unit_tahun: setahun.iter().map(|x| x.1).sum(),
        nilai_tahun: setahun.iter().map(|x| x.2).sum(),
        per_kategori,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tahun_bulan_dari_waktu_sinkron() {
        assert_eq!(tahun_bulan("2026-11-10 13:05 WIB"), Some((2026, 11)));
        assert_eq!(tahun_bulan("2026-13-10"), None);
        assert_eq!(tahun_bulan("t0"), None);
        assert_eq!(tahun_bulan(""), None);
    }
}
