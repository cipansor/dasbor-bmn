//! Penyimpanan lokal (SQLite): baris aset, kolom fakta, status sinkronisasi,
//! dan riwayat posisi.
//!
//! Tiap baris aset disimpan dua kali dalam satu rekaman: JSON utuh (untuk
//! telusur dan ekspor 106 kolom) dan kolom fakta bertipe (untuk analitik,
//! saring, urut, dan paginasi di SQL). Penggantian isi tabel memakai nomor
//! **generasi**: baris baru ditulis bertahap dengan generasi baru, lalu satu
//! transaksi singkat menukar generasi aktif. Pembaca selalu melihat generasi
//! aktif yang utuh, dan sinkronisasi tabel ratusan megabita tidak perlu
//! menampung seluruh baris di memori.

#![cfg(feature = "ssr")]

use crate::config;
use crate::fakta;
use crate::model::{AssetRow, ColumnDef, SyncStatus};
use rusqlite::types::ToSql;
use rusqlite::{params, Connection, OpenFlags};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

/// Versi skema; menaikkannya membuang cache lama agar disinkronkan ulang.
const VERSI_SKEMA: i64 = 2;

static DB: OnceLock<Mutex<Connection>> = OnceLock::new();

fn jalur() -> std::path::PathBuf {
    config::data_dir().join("bmn.sqlite")
}

/// Buka (dan siapkan) basis data lokal.
pub fn conn() -> &'static Mutex<Connection> {
    DB.get_or_init(|| {
        let path = jalur();
        let c = Connection::open(&path).expect("buka basis data lokal");
        c.execute_batch("PRAGMA journal_mode=WAL; PRAGMA synchronous=NORMAL;")
            .expect("atur mode basis data");
        let versi: i64 = c
            .query_row("PRAGMA user_version", [], |r| r.get(0))
            .unwrap_or(0);
        if versi != VERSI_SKEMA {
            // Isi basis data hanyalah cache dari SLDK; skema lama dibuang utuh.
            c.execute_batch(
                "DROP TABLE IF EXISTS assets;
                 DROP TABLE IF EXISTS aset;
                 DROP TABLE IF EXISTS sync_meta;
                 DROP TABLE IF EXISTS riwayat;
                 DROP TABLE IF EXISTS app_meta;",
            )
            .expect("buang skema lama");
        }
        c.execute_batch(&format!(
            "CREATE TABLE IF NOT EXISTS aset (
                 id          INTEGER PRIMARY KEY AUTOINCREMENT,
                 tabel       TEXT NOT NULL,
                 gen         INTEGER NOT NULL,
                 kunci       TEXT NOT NULL,
                 kode_satker TEXT NOT NULL,
                 nama_satker TEXT NOT NULL,
                 prov        TEXT NOT NULL,
                 kab         TEXT NOT NULL,
                 kd_brg      TEXT NOT NULL,
                 no_aset     TEXT NOT NULL,
                 nama        TEXT NOT NULL,
                 kelompok    TEXT NOT NULL,
                 merk        TEXT NOT NULL,
                 kondisi     INTEGER NOT NULL,
                 nilai       REAL NOT NULL,
                 buku        REAL NOT NULL,
                 tahun       INTEGER NOT NULL,
                 bulan       INTEGER NOT NULL,
                 umur_sisa   REAL,
                 flags       INTEGER NOT NULL,
                 data        TEXT NOT NULL
             );
             CREATE INDEX IF NOT EXISTS idx_aset_tabel ON aset(tabel, gen);
             CREATE INDEX IF NOT EXISTS idx_aset_satker ON aset(kode_satker);
             CREATE TABLE IF NOT EXISTS sync_meta (
                 table_name TEXT PRIMARY KEY,
                 gen        INTEGER NOT NULL,
                 row_count  INTEGER NOT NULL DEFAULT 0,
                 synced_at  TEXT
             );
             CREATE TABLE IF NOT EXISTS app_meta (
                 key   TEXT PRIMARY KEY,
                 value TEXT
             );
             CREATE TABLE IF NOT EXISTS riwayat (
                 tanggal TEXT PRIMARY KEY,
                 data    TEXT NOT NULL
             );
             PRAGMA user_version = {VERSI_SKEMA};"
        ))
        .expect("siapkan skema basis data");
        // Baris generasi yatim (sisa sinkronisasi yang terputus) tidak pernah
        // terlihat pembaca; buang agar tidak menumpuk.
        let _ = c.execute(
            "DELETE FROM aset WHERE NOT EXISTS (
                 SELECT 1 FROM sync_meta m WHERE m.table_name = aset.tabel AND m.gen = aset.gen)",
            [],
        );
        // Basis data memuat data BMN berklasifikasi Terbatas.
        crate::fs_aman::rapatkan(&path);
        Mutex::new(c)
    })
}

/// Koneksi baca-saja tersendiri untuk pemindaian besar.
///
/// Pemindaian (analitik, ekspor) tidak menahan mutex koneksi utama, dan berkat
/// WAL seluruh barisnya berasal dari satu snapshot walau sinkronisasi berjalan.
fn baca() -> rusqlite::Result<Connection> {
    let _ = conn();
    Connection::open_with_flags(
        jalur(),
        OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )
}

/// Baca nilai metadata aplikasi.
pub fn get_meta(key: &str) -> Option<String> {
    let c = conn().lock().unwrap();
    c.query_row(
        "SELECT value FROM app_meta WHERE key = ?1",
        params![key],
        |r| r.get::<_, String>(0),
    )
    .ok()
}

fn tulis_meta(c: &Connection, key: &str, value: &str) -> rusqlite::Result<()> {
    c.execute(
        "INSERT INTO app_meta (key, value) VALUES (?1, ?2)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        params![key, value],
    )?;
    Ok(())
}

/// Tulis nilai metadata aplikasi.
pub fn set_meta(key: &str, value: &str) -> rusqlite::Result<()> {
    let c = conn().lock().unwrap();
    tulis_meta(&c, key, value)
}

fn naikkan(c: &Connection, key: &str) -> rusqlite::Result<i64> {
    let kini: i64 = c
        .query_row(
            "SELECT CAST(value AS INTEGER) FROM app_meta WHERE key = ?1",
            params![key],
            |r| r.get(0),
        )
        .unwrap_or(0);
    tulis_meta(c, key, &(kini + 1).to_string())?;
    Ok(kini + 1)
}

/// Nomor versi data; berubah setiap kali isi suatu tabel diganti.
pub fn versi_data() -> i64 {
    get_meta("versi")
        .and_then(|v| v.parse().ok())
        .unwrap_or(0)
}

/// Apakah basis data saat ini berisi data contoh.
pub fn is_demo() -> bool {
    get_meta("mode").as_deref() == Some("demo")
}

/// Nomor generasi baru untuk satu putaran penulisan tabel.
pub fn gen_baru() -> rusqlite::Result<i64> {
    let c = conn().lock().unwrap();
    naikkan(&c, "gen")
}

/// Sisipkan sekumpulan baris mentah ke generasi yang sedang ditulis.
///
/// `awal` adalah nomor urut baris pertama pada sumber; dipakai sebagai kunci
/// cadangan bila baris tidak punya kode register maupun kode barang.
pub fn sisip(tabel: &str, gen: i64, rows: Vec<Value>, awal: usize) -> rusqlite::Result<usize> {
    let mut c = conn().lock().unwrap();
    let tx = c.transaction()?;
    let mut n = 0usize;
    {
        let mut stmt = tx.prepare_cached(
            "INSERT INTO aset (tabel, gen, kunci, kode_satker, nama_satker, prov, kab, kd_brg,
                 no_aset, nama, kelompok, merk, kondisi, nilai, buku, tahun, bulan, umur_sisa,
                 flags, data)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17,
                 ?18, ?19, ?20)",
        )?;
        for (i, mentah) in rows.into_iter().enumerate() {
            let row = fakta::normalkan(mentah);
            if !row.is_object() {
                continue;
            }
            let k = fakta::ekstrak(&row, awal + i);
            stmt.execute(params![
                tabel,
                gen,
                k.kunci,
                k.kode_satker,
                k.nama_satker,
                k.prov,
                k.kab,
                k.kd_brg,
                k.no_aset,
                k.nama,
                k.kelompok,
                k.merk,
                k.kondisi,
                k.nilai,
                k.buku,
                k.tahun,
                k.bulan,
                k.umur_sisa,
                k.flags,
                row.to_string(),
            ])?;
            n += 1;
        }
    }
    tx.commit()?;
    Ok(n)
}

/// Jadikan generasi `gen` isi aktif tabel dan buang isi lamanya.
///
/// Bila `buang_demo`, seluruh data contoh ikut dibuang dan mode berpindah ke
/// data langsung dalam transaksi yang sama, sehingga kegagalan tidak pernah
/// meninggalkan dasbor kosong atau bercampur antara contoh dan langsung.
pub fn komit(tabel: &str, gen: i64, synced_at: &str, buang_demo: bool) -> rusqlite::Result<i64> {
    let mut c = conn().lock().unwrap();
    let tx = c.transaction()?;
    if buang_demo {
        tx.execute(
            "DELETE FROM aset WHERE NOT (tabel = ?1 AND gen = ?2)",
            params![tabel, gen],
        )?;
        tx.execute("DELETE FROM sync_meta", [])?;
        tx.execute("DELETE FROM riwayat", [])?;
        tulis_meta(&tx, "mode", "live")?;
    } else {
        tx.execute(
            "DELETE FROM aset WHERE tabel = ?1 AND gen <> ?2",
            params![tabel, gen],
        )?;
    }
    let n: i64 = tx.query_row(
        "SELECT COUNT(*) FROM aset WHERE tabel = ?1 AND gen = ?2",
        params![tabel, gen],
        |r| r.get(0),
    )?;
    tx.execute(
        "INSERT INTO sync_meta (table_name, gen, row_count, synced_at)
         VALUES (?1, ?2, ?3, ?4)
         ON CONFLICT(table_name) DO UPDATE SET
            gen = excluded.gen,
            row_count = excluded.row_count,
            synced_at = excluded.synced_at",
        params![tabel, gen, n, synced_at],
    )?;
    naikkan(&tx, "versi")?;
    tx.commit()?;
    Ok(n)
}

/// Buang generasi yang gagal diselesaikan; isi aktif tidak tersentuh.
pub fn batal(tabel: &str, gen: i64) {
    let c = conn().lock().unwrap();
    let _ = c.execute(
        "DELETE FROM aset WHERE tabel = ?1 AND gen = ?2
           AND NOT EXISTS (SELECT 1 FROM sync_meta m WHERE m.table_name = ?1 AND m.gen = ?2)",
        params![tabel, gen],
    );
}

/// Ganti isi satu tabel sekaligus (dipakai data contoh dan pengujian).
pub fn ganti_tabel(tabel: &str, rows: Vec<Value>, synced_at: &str) -> rusqlite::Result<i64> {
    let gen = gen_baru()?;
    if let Err(e) = sisip(tabel, gen, rows, 0) {
        batal(tabel, gen);
        return Err(e);
    }
    komit(tabel, gen, synced_at, false)
}

/// Hapus seluruh cache.
pub fn clear_all() -> rusqlite::Result<()> {
    let mut c = conn().lock().unwrap();
    let tx = c.transaction()?;
    tx.execute("DELETE FROM aset", [])?;
    tx.execute("DELETE FROM sync_meta", [])?;
    tx.execute("DELETE FROM riwayat", [])?;
    tx.execute("DELETE FROM app_meta WHERE key = 'mode'", [])?;
    naikkan(&tx, "versi")?;
    tx.commit()
}

/// Jumlah baris aktif satu tabel.
pub fn stored_count(tabel: &str) -> i64 {
    let c = conn().lock().unwrap();
    c.query_row(
        "SELECT COUNT(*) FROM aset a JOIN sync_meta m
           ON m.table_name = a.tabel AND m.gen = a.gen WHERE a.tabel = ?1",
        params![tabel],
        |r| r.get(0),
    )
    .unwrap_or(0)
}

/// Status sinkronisasi mentah: (tabel, jumlah baris, waktu sinkron).
pub fn sync_statuses() -> Vec<(String, i64, Option<String>)> {
    let c = conn().lock().unwrap();
    let Ok(mut stmt) =
        c.prepare("SELECT table_name, row_count, synced_at FROM sync_meta ORDER BY table_name")
    else {
        return vec![];
    };
    let rows = stmt.query_map([], |r| {
        Ok((
            r.get::<_, String>(0)?,
            r.get::<_, i64>(1)?,
            r.get::<_, Option<String>>(2)?,
        ))
    });
    match rows {
        Ok(it) => it.filter_map(|r| r.ok()).collect(),
        Err(_) => vec![],
    }
}

/// Status sinkronisasi seluruh tabel katalog untuk tampilan.
pub fn statuses_with_labels() -> Vec<SyncStatus> {
    let semua = sync_statuses();
    crate::catalog::TABLES
        .iter()
        .map(|t| {
            let found = semua.iter().find(|(n, _, _)| n == t.table);
            let (count, at) = found
                .map(|(_, c, a)| (*c, a.clone()))
                .unwrap_or((0, None));
            SyncStatus {
                table: t.table.to_string(),
                label: t.label.to_string(),
                row_count: count,
                status: if at.is_some() {
                    "tersinkron".into()
                } else {
                    "belum".into()
                },
                last_sync: at,
                message: None,
            }
        })
        .collect()
}

/// Satu baris fakta yang dipinjamkan selama pemindaian.
pub struct FaktaRef<'a> {
    pub id: i64,
    pub tabel: &'a str,
    pub kode_satker: &'a str,
    pub nama_satker: &'a str,
    pub prov: &'a str,
    pub kelompok: &'a str,
    pub kondisi: i64,
    pub nilai: f64,
    pub buku: f64,
    pub tahun: i64,
    pub bulan: i64,
    pub umur_sisa: Option<f64>,
    pub flags: i64,
}

/// Alirkan kolom fakta seluruh aset aktif (atau satu satuan kerja saja).
pub fn alir_fakta(satker: Option<&str>, mut f: impl FnMut(FaktaRef<'_>)) -> rusqlite::Result<()> {
    let c = baca()?;
    let mut sql = String::from(
        "SELECT a.id, a.tabel, a.kode_satker, a.nama_satker, a.prov, a.kelompok, a.kondisi,
                a.nilai, a.buku, a.tahun, a.bulan, a.umur_sisa, a.flags
         FROM aset a JOIN sync_meta m ON m.table_name = a.tabel AND m.gen = a.gen",
    );
    if satker.is_some() {
        sql.push_str(" WHERE a.kode_satker = ?1");
    }
    let mut stmt = c.prepare(&sql)?;
    let mut rows = match satker {
        Some(s) => stmt.query(params![s])?,
        None => stmt.query([])?,
    };
    while let Some(r) = rows.next()? {
        f(FaktaRef {
            id: r.get(0)?,
            tabel: r.get_ref(1)?.as_str().unwrap_or(""),
            kode_satker: r.get_ref(2)?.as_str().unwrap_or(""),
            nama_satker: r.get_ref(3)?.as_str().unwrap_or(""),
            prov: r.get_ref(4)?.as_str().unwrap_or(""),
            kelompok: r.get_ref(5)?.as_str().unwrap_or(""),
            kondisi: r.get(6)?,
            nilai: r.get(7)?,
            buku: r.get(8)?,
            tahun: r.get(9)?,
            bulan: r.get(10)?,
            umur_sisa: r.get(11)?,
            flags: r.get(12)?,
        });
    }
    Ok(())
}

/// Identitas tampilan satu aset.
#[derive(Debug, Clone, Default)]
pub struct Identitas {
    pub nama: String,
    pub kd_brg: String,
    pub no_aset: String,
    pub merk: String,
}

/// Ambil identitas sekumpulan aset berdasarkan id baris.
pub fn identitas(ids: &[i64]) -> HashMap<i64, Identitas> {
    let mut out = HashMap::new();
    if ids.is_empty() {
        return out;
    }
    let Ok(c) = baca() else {
        return out;
    };
    for potong in ids.chunks(400) {
        let tanda = vec!["?"; potong.len()].join(",");
        let sql = format!("SELECT id, nama, kd_brg, no_aset, merk FROM aset WHERE id IN ({tanda})");
        let Ok(mut stmt) = c.prepare(&sql) else {
            continue;
        };
        let par: Vec<&dyn ToSql> = potong.iter().map(|i| i as &dyn ToSql).collect();
        let hasil = stmt.query_map(par.as_slice(), |r| {
            Ok((
                r.get::<_, i64>(0)?,
                Identitas {
                    nama: r.get(1)?,
                    kd_brg: r.get(2)?,
                    no_aset: r.get(3)?,
                    merk: r.get(4)?,
                },
            ))
        });
        if let Ok(it) = hasil {
            for (id, idn) in it.flatten() {
                out.insert(id, idn);
            }
        }
    }
    out
}

/// Perolehan aktif pada satu tahun (dan bulan, bila diberikan): (tabel, unit, nilai).
pub fn perolehan_periode(tahun: i64, bulan: Option<i64>) -> Vec<(String, i64, f64)> {
    let Ok(c) = baca() else {
        return vec![];
    };
    let mut sql = String::from(
        "SELECT a.tabel, COUNT(*), COALESCE(SUM(a.nilai), 0)
         FROM aset a JOIN sync_meta m ON m.table_name = a.tabel AND m.gen = a.gen
         WHERE a.tahun = ?1 AND (a.flags & ?2) = 0",
    );
    if bulan.is_some() {
        sql.push_str(" AND a.bulan = ?3");
    }
    sql.push_str(" GROUP BY a.tabel");
    let Ok(mut stmt) = c.prepare(&sql) else {
        return vec![];
    };
    let petakan = |r: &rusqlite::Row<'_>| {
        Ok((
            r.get::<_, String>(0)?,
            r.get::<_, i64>(1)?,
            r.get::<_, f64>(2)?,
        ))
    };
    let hasil = match bulan {
        Some(b) => stmt.query_map(params![tahun, fakta::F_DIHAPUS, b], petakan),
        None => stmt.query_map(params![tahun, fakta::F_DIHAPUS], petakan),
    };
    match hasil {
        Ok(it) => it.filter_map(|r| r.ok()).collect(),
        Err(_) => vec![],
    }
}

/// Banyaknya kunci aset yang muncul lebih dari sekali pada data aktif.
pub fn kunci_ganda() -> i64 {
    let Ok(c) = baca() else {
        return 0;
    };
    c.query_row(
        "SELECT COALESCE(SUM(n - 1), 0) FROM (
             SELECT COUNT(*) AS n FROM aset a JOIN sync_meta m
               ON m.table_name = a.tabel AND m.gen = a.gen
             WHERE a.kunci NOT LIKE 'i:%'
             GROUP BY a.tabel, a.kunci HAVING COUNT(*) > 1)",
        [],
        |r| r.get(0),
    )
    .unwrap_or(0)
}

/// Saringan halaman telusur data.
#[derive(Debug, Clone, Default)]
pub struct Saring {
    pub tabel: String,
    pub q: String,
    /// Kode satuan kerja; kosong berarti semua.
    pub satker: String,
    /// Kode kondisi 0..=3; negatif berarti semua.
    pub kondisi: i64,
    /// "", "idle", "hilang", atau "habis".
    pub status: String,
    pub sort: String,
    pub desc: bool,
    pub page: i64,
    pub per_page: i64,
}

/// Ungkapan SQL untuk pengurutan; hanya kolom fakta yang diizinkan.
fn kolom_urut(nama: &str) -> Option<&'static str> {
    Some(match nama {
        "kd_brg" => "a.kd_brg",
        "nama" => "a.nama COLLATE NOCASE",
        "ur_sskel" => "a.kelompok COLLATE NOCASE",
        "merk" => "a.merk COLLATE NOCASE",
        "rph_aset" => "a.nilai",
        "rph_buku" => "a.buku",
        "ur_kondisi" => "a.kondisi",
        "ur_prov" => "a.prov COLLATE NOCASE",
        "nama_satker" => "a.nama_satker COLLATE NOCASE",
        "tgl_perlh" => "a.tahun * 100 + a.bulan",
        "umur_sisa" => "a.umur_sisa",
        _ => return None,
    })
}

/// Kolom yang dapat dipakai untuk mengurutkan halaman telusur.
pub fn dapat_diurut(nama: &str) -> bool {
    kolom_urut(nama).is_some()
}

fn lolos_like(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('%');
    for c in s.chars() {
        if matches!(c, '%' | '_' | '\\') {
            out.push('\\');
        }
        out.push(c);
    }
    out.push('%');
    out
}

/// Ambil satu halaman baris aset: saring, urut, dan paginasi dikerjakan SQL.
pub fn halaman(s: &Saring, columns: &[ColumnDef]) -> rusqlite::Result<(Vec<AssetRow>, i64)> {
    let c = baca()?;
    let mut di = String::from(
        " FROM aset a JOIN sync_meta m ON m.table_name = a.tabel AND m.gen = a.gen
          WHERE a.tabel = ?",
    );
    let mut par: Vec<Box<dyn ToSql>> = vec![Box::new(s.tabel.clone())];
    let q = s.q.trim();
    if !q.is_empty() {
        di.push_str(
            " AND (a.nama LIKE ? ESCAPE '\\' OR a.kd_brg LIKE ? ESCAPE '\\'
               OR a.merk LIKE ? ESCAPE '\\' OR a.nama_satker LIKE ? ESCAPE '\\'
               OR a.kelompok LIKE ? ESCAPE '\\' OR a.prov LIKE ? ESCAPE '\\'
               OR a.kab LIKE ? ESCAPE '\\' OR a.no_aset LIKE ? ESCAPE '\\')",
        );
        let pola = lolos_like(q);
        for _ in 0..8 {
            par.push(Box::new(pola.clone()));
        }
    }
    if !s.satker.trim().is_empty() {
        di.push_str(" AND a.kode_satker = ?");
        par.push(Box::new(s.satker.trim().to_string()));
    }
    if (0..=3).contains(&s.kondisi) {
        di.push_str(" AND a.kondisi = ?");
        par.push(Box::new(s.kondisi));
    }
    match s.status.as_str() {
        "idle" => di.push_str(&format!(" AND (a.flags & {}) <> 0", fakta::F_IDLE)),
        "hilang" => di.push_str(&format!(" AND (a.flags & {}) <> 0", fakta::F_HILANG)),
        "habis" => di.push_str(" AND a.umur_sisa IS NOT NULL AND a.umur_sisa <= 0"),
        _ => {}
    }
    let acuan: Vec<&dyn ToSql> = par.iter().map(|b| b.as_ref()).collect();
    let total: i64 = c.query_row(&format!("SELECT COUNT(*){di}"), acuan.as_slice(), |r| {
        r.get(0)
    })?;

    let urut = match kolom_urut(s.sort.trim()) {
        Some(k) => format!(
            " ORDER BY {k} {}, a.id",
            if s.desc { "DESC" } else { "ASC" }
        ),
        None => " ORDER BY a.id".to_string(),
    };
    let per = s.per_page.clamp(1, 200);
    let lompat = (s.page.max(1) - 1) * per;
    let sql = format!("SELECT a.data{di}{urut} LIMIT {per} OFFSET {lompat}");
    let mut stmt = c.prepare(&sql)?;
    let rows = stmt
        .query_map(acuan.as_slice(), |r| r.get::<_, String>(0))?
        .filter_map(|r| r.ok())
        .filter_map(|t| serde_json::from_str::<Value>(&t).ok())
        .map(|v| AssetRow {
            values: columns
                .iter()
                .map(|k| {
                    v.get(&k.name)
                        .map(crate::model::cell_to_string)
                        .unwrap_or_default()
                })
                .collect(),
        })
        .collect();
    Ok((rows, total))
}

/// Alirkan JSON utuh seluruh baris aktif satu tabel dalam urutan stabil.
///
/// Dipakai ekspor. Kueri berjalan satu kali pada koneksi baca-saja, jadi seluruh
/// baris berasal dari snapshot yang sama meski sinkronisasi berjalan di sampingnya.
pub fn alir_baris(tabel: &str, mut f: impl FnMut(&Value)) -> rusqlite::Result<()> {
    let c = baca()?;
    let mut stmt = c.prepare(
        "SELECT a.data FROM aset a JOIN sync_meta m ON m.table_name = a.tabel AND m.gen = a.gen
         WHERE a.tabel = ?1 ORDER BY a.id",
    )?;
    let mut rows = stmt.query(params![tabel])?;
    while let Some(r) = rows.next()? {
        if let Ok(v) = serde_json::from_str::<Value>(r.get_ref(0)?.as_str().unwrap_or("")) {
            f(&v);
        }
    }
    Ok(())
}

/// Simpan posisi ringkas harian (satu rekaman per tanggal).
pub fn simpan_riwayat(tanggal: &str, data: &str) -> rusqlite::Result<()> {
    let c = conn().lock().unwrap();
    c.execute(
        "INSERT INTO riwayat (tanggal, data) VALUES (?1, ?2)
         ON CONFLICT(tanggal) DO UPDATE SET data = excluded.data",
        params![tanggal, data],
    )?;
    Ok(())
}

/// Posisi ringkas terakhir sebelum `tanggal`.
pub fn riwayat_sebelum(tanggal: &str) -> Option<(String, String)> {
    let c = conn().lock().unwrap();
    c.query_row(
        "SELECT tanggal, data FROM riwayat WHERE tanggal < ?1 ORDER BY tanggal DESC LIMIT 1",
        params![tanggal],
        |r| Ok((r.get(0)?, r.get(1)?)),
    )
    .ok()
}

/// Arahkan uji ke basis data sementara agar tidak menyentuh cache pengembangan.
#[cfg(test)]
pub fn uji_isolasi() {
    static SEKALI: std::sync::OnceLock<()> = std::sync::OnceLock::new();
    SEKALI.get_or_init(|| {
        let unik = format!(
            "dasbor-bmn-uji-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        );
        let dir = std::env::temp_dir().join(unik);
        let _ = std::fs::remove_dir_all(&dir);
        let _ = std::fs::create_dir_all(&dir);
        std::env::set_var("SIMAN_DATA_DIR", &dir);
    });
}

/// Kunci bersama untuk uji yang menyentuh basis data.
///
/// Seluruh uji dalam satu proses berbagi satu berkas SQLite. Tanpa serialisasi,
/// uji paralel saling menimpa data sehingga hasilnya bergantung waktu.
#[cfg(test)]
pub fn uji_kunci() -> std::sync::MutexGuard<'static, ()> {
    static KUNCI: OnceLock<Mutex<()>> = OnceLock::new();
    let k = KUNCI
        .get_or_init(|| Mutex::new(()))
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    uji_isolasi();
    conn();
    k
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn baris(n: usize, awalan: &str) -> Vec<Value> {
        (0..n)
            .map(|i| {
                json!({
                    "kd_brg": format!("{awalan}{i:04}"), "no_aset": i,
                    "nama": format!("Barang {awalan} {i}"), "rph_aset": (i as i64 + 1) * 1000,
                    "ur_kondisi": if i % 3 == 0 { "Rusak Berat" } else { "Baik" },
                    "kode_satker": if i % 2 == 0 { "S1" } else { "S2" },
                    "nama_satker": if i % 2 == 0 { "SATKER SATU" } else { "SATKER DUA" },
                    "status_bmn_idle": if i == 1 { "Y" } else { "N" },
                    "tgl_perlh": "2020-06-01"
                })
            })
            .collect()
    }

    fn kolom(nama: &[&str]) -> Vec<ColumnDef> {
        nama.iter()
            .map(|n| ColumnDef {
                name: n.to_string(),
                label: n.to_string(),
                r#type: "text".into(),
                len: "-".into(),
            })
            .collect()
    }

    #[test]
    fn generasi_baru_tidak_terlihat_sampai_dikomit() {
        let _k = uji_kunci();
        let t = "UJI_GENERASI";
        ganti_tabel(t, baris(5, "L"), "t0").unwrap();
        assert_eq!(stored_count(t), 5);

        // Tulis generasi baru sebagian: pembaca tetap melihat isi lama.
        let gen = gen_baru().unwrap();
        sisip(t, gen, baris(3, "B"), 0).unwrap();
        assert_eq!(stored_count(t), 5, "generasi setengah jadi tidak boleh terlihat");
        let mut lama = 0;
        alir_baris(t, |v| {
            assert!(v["kd_brg"].as_str().unwrap().starts_with('L'));
            lama += 1;
        })
        .unwrap();
        assert_eq!(lama, 5);

        // Sinkronisasi gagal: generasi dibatalkan, isi lama utuh.
        batal(t, gen);
        assert_eq!(stored_count(t), 5);

        // Sinkronisasi berhasil: isi berganti utuh dan versi data naik.
        let v0 = versi_data();
        let gen = gen_baru().unwrap();
        sisip(t, gen, baris(3, "B"), 0).unwrap();
        assert_eq!(komit(t, gen, "t1", false).unwrap(), 3);
        assert_eq!(stored_count(t), 3);
        assert!(versi_data() > v0);
    }

    #[test]
    fn batal_tidak_menghapus_generasi_aktif() {
        let _k = uji_kunci();
        let t = "UJI_BATAL_AKTIF";
        ganti_tabel(t, baris(4, "A"), "t0").unwrap();
        let aktif: i64 = {
            let c = conn().lock().unwrap();
            c.query_row(
                "SELECT gen FROM sync_meta WHERE table_name = ?1",
                params![t],
                |r| r.get(0),
            )
            .unwrap()
        };
        batal(t, aktif);
        assert_eq!(stored_count(t), 4, "generasi aktif tidak boleh ikut terhapus");
    }

    #[test]
    fn komit_pertama_membuang_data_contoh() {
        let _k = uji_kunci();
        ganti_tabel("UJI_CONTOH_A", baris(2, "D"), "t0").unwrap();
        set_meta("mode", "demo").unwrap();
        assert!(is_demo());
        let gen = gen_baru().unwrap();
        sisip("UJI_LANGSUNG", gen, baris(6, "N"), 0).unwrap();
        komit("UJI_LANGSUNG", gen, "t1", true).unwrap();
        assert_eq!(stored_count("UJI_CONTOH_A"), 0, "data contoh ikut terbuang");
        assert_eq!(stored_count("UJI_LANGSUNG"), 6);
        assert!(!is_demo());
    }

    #[test]
    fn halaman_menyaring_mengurut_dan_membagi() {
        let _k = uji_kunci();
        let t = "UJI_HALAMAN";
        ganti_tabel(t, baris(30, "H"), "t0").unwrap();
        let kol = kolom(&["kd_brg", "nama", "rph_aset"]);

        let mut s = Saring {
            tabel: t.into(),
            kondisi: -1,
            page: 1,
            per_page: 10,
            ..Default::default()
        };
        let (rows, total) = halaman(&s, &kol).unwrap();
        assert_eq!((rows.len(), total), (10, 30));

        // Urut angka menurun: nilai terbesar di depan (bukan urutan teks).
        s.sort = "rph_aset".into();
        s.desc = true;
        let (rows, _) = halaman(&s, &kol).unwrap();
        assert_eq!(rows[0].values[2], "30000");

        // Saring kondisi rusak berat (i % 3 == 0 → 10 baris).
        s.kondisi = fakta::K_RB;
        assert_eq!(halaman(&s, &kol).unwrap().1, 10);
        s.kondisi = -1;

        // Saring satuan kerja dan status idle.
        s.satker = "S2".into();
        assert_eq!(halaman(&s, &kol).unwrap().1, 15);
        s.status = "idle".into();
        assert_eq!(halaman(&s, &kol).unwrap().1, 1);
        s.satker.clear();
        s.status.clear();

        // Pencarian teks; karakter pola diperlakukan harfiah.
        s.q = "barang h 7".into();
        assert_eq!(halaman(&s, &kol).unwrap().1, 1);
        s.q = "%".into();
        assert_eq!(halaman(&s, &kol).unwrap().1, 0);

        // Kolom urut tak dikenal tidak menjadi SQL.
        s.q.clear();
        s.sort = "data; DROP TABLE aset".into();
        assert_eq!(halaman(&s, &kol).unwrap().1, 30);
    }

    #[test]
    fn fakta_dan_identitas_terbaca() {
        let _k = uji_kunci();
        let t = "UJI_FAKTA";
        ganti_tabel(t, baris(6, "F"), "t0").unwrap();
        let mut ids = Vec::new();
        let mut nilai = 0.0;
        alir_fakta(Some("S1"), |f| {
            if f.tabel == t {
                ids.push(f.id);
                nilai += f.nilai;
                assert_eq!(f.tahun, 2020);
            }
        })
        .unwrap();
        assert_eq!(ids.len(), 3);
        assert_eq!(nilai, 1000.0 + 3000.0 + 5000.0);
        let idn = identitas(&ids);
        assert_eq!(idn.len(), 3);
        assert!(idn.values().all(|i| i.nama.starts_with("Barang F")));
        let per = perolehan_periode(2020, Some(6));
        assert!(per.iter().any(|(tb, n, _)| tb == t && *n == 6));
    }

    #[test]
    fn riwayat_mengambil_posisi_sebelumnya() {
        let _k = uji_kunci();
        simpan_riwayat("2031-01-05", "{\"a\":1}").unwrap();
        simpan_riwayat("2031-01-12", "{\"a\":2}").unwrap();
        simpan_riwayat("2031-01-12", "{\"a\":3}").unwrap();
        let (tgl, data) = riwayat_sebelum("2031-01-12").unwrap();
        assert_eq!(tgl, "2031-01-05");
        assert_eq!(data, "{\"a\":1}");
        assert_eq!(riwayat_sebelum("2031-01-13").unwrap().1, "{\"a\":3}");
    }
}
