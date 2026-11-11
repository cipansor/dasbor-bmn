//! Pekerjaan sinkronisasi latar dengan laporan kemajuan.
//!
//! Menarik satu tabel besar dapat memakan waktu jauh melebihi batas waktu satu
//! permintaan HTTP. Karena itu sinkronisasi berjalan sebagai tugas latar:
//! antarmuka memulainya lalu menanyakan kemajuan secara berkala. Baris ditulis
//! per potongan ke generasi baru (lihat `store`), sehingga memori tetap kecil
//! dan isi lama tetap tersaji sampai unduhan lengkap.

#![cfg(feature = "ssr")]

use crate::catalog::{self, TABLES};
use crate::dto::{Progres, ProgresTabel};
use crate::model::TableDef;
use crate::{config, demo, sldk, store};
use std::sync::{Mutex, OnceLock};

/// Banyaknya baris per permintaan, mengikuti contoh pada panduan SLDK.
const POTONGAN: i64 = 1000;
/// Percobaan ulang per potongan saat jaringan/gateway bermasalah.
const ULANG_MAKS: u32 = 3;
/// Baris log yang disimpan.
const LOG_MAKS: usize = 300;

static KEADAAN: OnceLock<Mutex<Progres>> = OnceLock::new();

fn keadaan() -> &'static Mutex<Progres> {
    KEADAAN.get_or_init(|| Mutex::new(Progres::default()))
}

fn ubah(f: impl FnOnce(&mut Progres)) {
    let mut p = keadaan().lock().unwrap_or_else(|e| e.into_inner());
    f(&mut p);
}

fn catat(pesan: String) {
    eprintln!("[sinkron] {pesan}");
    ubah(|p| {
        p.log.push(format!("{} — {pesan}", crate::now_string()));
        if p.log.len() > LOG_MAKS {
            let buang = p.log.len() - LOG_MAKS;
            p.log.drain(0..buang);
        }
    });
}

fn ubah_tabel(table: &str, f: impl FnOnce(&mut ProgresTabel)) {
    ubah(|p| {
        if let Some(t) = p.tabel.iter_mut().find(|t| t.table == table) {
            f(t);
        }
    });
}

/// Kemajuan pekerjaan terakhir.
pub fn progres() -> Progres {
    keadaan().lock().unwrap_or_else(|e| e.into_inner()).clone()
}

/// Mulai sinkronisasi tabel yang diminta (kosong = seluruh katalog).
///
/// Mengembalikan galat bila pekerjaan lain masih berjalan atau nama tabel tidak dikenal.
pub fn mulai(diminta: &[String]) -> Result<(), String> {
    let daftar: Vec<&'static TableDef> = if diminta.is_empty() {
        TABLES.iter().collect()
    } else {
        let mut v = Vec::new();
        for nama in diminta {
            v.push(catalog::find(nama).ok_or_else(|| format!("tabel tidak dikenal: {nama}"))?);
        }
        v
    };
    {
        let mut p = keadaan().lock().unwrap_or_else(|e| e.into_inner());
        if p.berjalan {
            return Err("Sinkronisasi lain masih berjalan.".into());
        }
        *p = Progres {
            berjalan: true,
            mulai: crate::now_string(),
            selesai: String::new(),
            tabel: daftar
                .iter()
                .map(|d| ProgresTabel {
                    table: d.table.into(),
                    label: d.label.into(),
                    status: "antre".into(),
                    ..Default::default()
                })
                .collect(),
            log: std::mem::take(&mut p.log),
        };
    }
    tokio::spawn(async move {
        jalankan(daftar).await;
        ubah(|p| {
            p.berjalan = false;
            p.selesai = crate::now_string();
        });
    });
    Ok(())
}

async fn jalankan(daftar: Vec<&'static TableDef>) {
    if !config::get().is_configured() {
        // Tanpa kredensial tidak ada sumber untuk ditarik: muat ulang data contoh.
        catat("Kredensial SLDK belum diisi — memuat ulang data contoh.".into());
        let hasil = tokio::task::spawn_blocking(|| {
            let _ = store::clear_all();
            demo::seed();
        })
        .await;
        let status = store::sync_statuses();
        ubah(|p| {
            for t in p.tabel.iter_mut() {
                let n = status
                    .iter()
                    .find(|s| s.0 == t.table)
                    .map(|s| s.1)
                    .unwrap_or(0);
                t.status = if hasil.is_ok() { "selesai" } else { "gagal" }.into();
                t.terambil = n;
                t.total = n;
                t.pesan = "data contoh".into();
            }
        });
        catat("Data contoh selesai dimuat.".into());
        return;
    }

    catat(format!("Mulai sinkronisasi {} tabel.", daftar.len()));
    let (mut sukses, mut gagal) = (0, 0);
    for def in daftar {
        ubah_tabel(def.table, |t| t.status = "berjalan".into());
        match sinkron_tabel(def).await {
            Ok(n) => {
                sukses += 1;
                ubah_tabel(def.table, |t| {
                    t.status = "selesai".into();
                    t.terambil = n;
                    t.total = n;
                    t.pesan = if n == 0 {
                        "Sumber tidak memuat baris untuk tabel ini.".into()
                    } else {
                        String::new()
                    };
                });
                catat(format!("{}: {} baris tersimpan.", def.label, n));
            }
            Err(e) => {
                gagal += 1;
                ubah_tabel(def.table, |t| {
                    t.status = "gagal".into();
                    t.pesan = e.clone();
                });
                catat(format!("{}: GAGAL — {e}", def.label));
            }
        }
    }
    catat(format!("Selesai: {sukses} berhasil, {gagal} gagal."));
}

/// Ambil satu potongan dengan percobaan ulang berjeda.
async fn ambil(
    resource: &str,
    id1: i64,
    id2: i64,
) -> Result<Option<Vec<serde_json::Value>>, sldk::SldkError> {
    let mut percobaan = 0;
    loop {
        match sldk::fetch_rows(resource, id1, id2).await {
            Ok(v) => return Ok(v),
            // Kredensial yang belum lengkap tidak akan pulih dengan mengulang.
            Err(e @ sldk::SldkError::NotConfigured(_)) => return Err(e),
            Err(e) => {
                percobaan += 1;
                if percobaan >= ULANG_MAKS {
                    return Err(e);
                }
                // Token kedaluwarsa di tengah unduhan panjang: minta yang baru.
                sldk::lupakan_token();
                tokio::time::sleep(std::time::Duration::from_secs(2u64.pow(percobaan))).await;
            }
        }
    }
}

/// Tarik satu tabel utuh; isi lama dipertahankan bila unduhan gagal atau kurang.
async fn sinkron_tabel(def: &'static TableDef) -> Result<i64, String> {
    let total = sldk::row_count(def.table)
        .await
        .map_err(|e| e.user_message())?;
    ubah_tabel(def.table, |t| t.total = total.max(0));

    let gen = store::gen_baru().map_err(|e| e.to_string())?;
    let mut terambil: i64 = 0;
    let mut id1: i64 = 1;
    while id1 <= total {
        let potongan = match ambil(def.resource, id1, id1 + POTONGAN - 1).await {
            Ok(Some(rows)) if !rows.is_empty() => rows,
            Ok(_) => break,
            Err(e) => {
                store::batal(def.table, gen);
                return Err(e.user_message());
            }
        };
        let tabel = def.table;
        let awal = terambil as usize;
        let n = tokio::task::spawn_blocking(move || store::sisip(tabel, gen, potongan, awal))
            .await
            .map_err(|e| e.to_string())
            .and_then(|r| r.map_err(|e| e.to_string()));
        match n {
            Ok(n) => terambil += n as i64,
            Err(e) => {
                store::batal(def.table, gen);
                return Err(format!("gagal menyimpan: {e}"));
            }
        }
        ubah_tabel(def.table, |t| t.terambil = terambil);
        id1 += POTONGAN;
    }

    // Unduhan yang tidak lengkap tidak boleh menggantikan isi lama.
    if terambil < total {
        store::batal(def.table, gen);
        return Err(format!(
            "Unduhan tidak lengkap: terkumpul {terambil} dari {total} baris. Data lama dipertahankan; coba sinkronkan lagi."
        ));
    }
    let tabel = def.table;
    let waktu = crate::now_string();
    tokio::task::spawn_blocking(move || store::komit(tabel, gen, &waktu, store::is_demo()))
        .await
        .map_err(|e| e.to_string())?
        .map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tabel_tak_dikenal_ditolak_sebelum_mulai() {
        let e = mulai(&["BUKAN_TABEL".to_string()]).unwrap_err();
        assert!(e.contains("tidak dikenal"));
        assert!(
            !progres().berjalan,
            "pekerjaan tidak boleh tercatat berjalan"
        );
    }
}
