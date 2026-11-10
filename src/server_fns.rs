use leptos::prelude::*;
use serde::{Deserialize, Serialize};

use crate::catalog;
use crate::dto::{Analisis, Asumsi, Progres, SatkerBaris, SatkerRinci};
use crate::model::{AssetPage, ConnInfo, SyncStatus};
#[cfg(feature = "ssr")]
use crate::{analitik, config, demo, sinkron, sldk, store};

/// Metadata satu tabel beserta kolomnya untuk kebutuhan tampilan.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TableMeta {
    pub table: String,
    pub resource: String,
    pub label: String,
    pub nama_data: String,
    pub volume: String,
    pub icon: String,
    pub columns: Vec<crate::model::ColumnDef>,
}

/// Jalankan pekerjaan berat di luar utas async agar permintaan lain tidak tertahan.
#[cfg(feature = "ssr")]
async fn di_latar<T: Send + 'static>(
    f: impl FnOnce() -> T + Send + 'static,
) -> Result<T, ServerFnError> {
    tokio::task::spawn_blocking(f)
        .await
        .map_err(|e| ServerFnError::new(format!("pekerjaan terhenti: {e}")))
}

/// Hasil analitik nasional; otomatis memakai data contoh bila kredensial belum ada.
#[server]
pub async fn get_analisis() -> Result<Analisis, ServerFnError> {
    di_latar(|| analitik::hasil().analisis.clone()).await
}

/// Keterangan umum hasil analitik (mode data, posisi data, tahun acuan).
#[server]
pub async fn get_meta() -> Result<crate::dto::Meta, ServerFnError> {
    di_latar(|| analitik::hasil().analisis.meta.clone()).await
}

/// Daftar ringkas seluruh satuan kerja.
#[server]
pub async fn get_satker() -> Result<Vec<SatkerBaris>, ServerFnError> {
    di_latar(|| analitik::hasil().satker.clone()).await
}

/// Rincian satu satuan kerja.
#[server]
pub async fn get_satker_rinci(kode: String) -> Result<SatkerRinci, ServerFnError> {
    di_latar(move || analitik::rinci_satker(&kode))
        .await?
        .ok_or_else(|| ServerFnError::new("Satuan kerja tidak ditemukan."))
}

/// Daftar tabel BMN beserta definisi kolomnya.
#[server]
pub async fn get_tables() -> Result<Vec<TableMeta>, ServerFnError> {
    Ok(catalog::TABLES
        .iter()
        .map(|t| TableMeta {
            table: t.table.to_string(),
            resource: t.resource.to_string(),
            label: t.label.to_string(),
            nama_data: t.nama_data.to_string(),
            volume: t.volume.to_string(),
            icon: t.icon.to_string(),
            columns: crate::schema::columns_of(t.table),
        })
        .collect())
}

/// Status sinkronisasi tiap tabel.
#[server]
pub async fn get_sync_statuses() -> Result<Vec<SyncStatus>, ServerFnError> {
    di_latar(|| {
        demo::pastikan();
        store::statuses_with_labels()
    })
    .await
}

/// Kemajuan pekerjaan sinkronisasi latar.
#[server]
pub async fn get_progres() -> Result<Progres, ServerFnError> {
    Ok(sinkron::progres())
}

/// Mulai sinkronisasi latar; `table` kosong berarti seluruh tabel.
#[server]
pub async fn mulai_sinkron(table: String) -> Result<Progres, ServerFnError> {
    let diminta: Vec<String> = if table.trim().is_empty() {
        vec![]
    } else {
        vec![table.trim().to_string()]
    };
    sinkron::mulai(&diminta).map_err(ServerFnError::new)?;
    Ok(sinkron::progres())
}

#[cfg(feature = "ssr")]
async fn info_koneksi() -> ConnInfo {
    let cfg = config::get();
    let (token_valid, last_error) = if cfg.client_id.is_empty() || cfg.client_secret.is_empty() {
        (false, None)
    } else {
        match sldk::token().await {
            Ok(_) => (true, None),
            Err(e) => (false, Some(e.user_message())),
        }
    };
    let isi = |v: &str| (!v.is_empty()).then(|| v.to_string());
    ConnInfo {
        base_url_set: !cfg.base_url.is_empty(),
        client_id: isi(&cfg.client_id),
        grant_type: isi(&cfg.grant_type),
        ba_key: (!cfg.ba_key.is_empty()).then(|| mask(&cfg.ba_key)),
        client_secret_set: !cfg.client_secret.is_empty(),
        token_valid,
        last_error,
    }
}

/// Informasi koneksi aktif (tanpa membocorkan rahasia).
#[server]
pub async fn get_conn_info() -> Result<ConnInfo, ServerFnError> {
    Ok(info_koneksi().await)
}

/// Samarkan nilai, menyisakan empat karakter terakhir.
pub fn mask(v: &str) -> String {
    let huruf: Vec<char> = v.chars().collect();
    let n = huruf.len();
    if n <= 4 {
        "•".repeat(n.max(1))
    } else {
        format!(
            "{}{}",
            "•".repeat(n - 4),
            huruf[n - 4..].iter().collect::<String>()
        )
    }
}

/// Simpan kredensial yang diisi pengguna ke berkas konfigurasi lokal.
#[server]
pub async fn save_conn_config(
    client_id: String,
    client_secret: String,
    grant_type: String,
    ba_key: String,
) -> Result<ConnInfo, ServerFnError> {
    let patch = config::FileConfig {
        client_id: client_id.trim().to_string(),
        client_secret: client_secret.trim().to_string(),
        grant_type: if grant_type.trim().is_empty() {
            "client_credentials".to_string()
        } else {
            grant_type.trim().to_string()
        },
        ba_key: ba_key.trim().to_string(),
        ..Default::default()
    };
    config::save_file_config(patch)
        .map_err(|e| ServerFnError::new(format!("gagal menyimpan: {e}")))?;
    eprintln!("[audit] kredensial SLDK diperbarui lewat halaman Pengaturan");
    Ok(info_koneksi().await)
}

/// Asumsi analitik yang sedang dipakai.
#[server]
pub async fn get_asumsi() -> Result<Asumsi, ServerFnError> {
    Ok(config::asumsi())
}

/// Simpan asumsi analitik; hasil analitik dihitung ulang pada permintaan berikutnya.
#[server]
pub async fn save_asumsi(
    eskalasi_persen: f64,
    rehab_persen: f64,
    satuan_umur_sisa: String,
) -> Result<Asumsi, ServerFnError> {
    if !eskalasi_persen.is_finite() || !rehab_persen.is_finite() {
        return Err(ServerFnError::new("Angka asumsi tidak sah."));
    }
    let a = Asumsi {
        eskalasi: (eskalasi_persen / 100.0).clamp(0.0, 0.25),
        faktor_rehab: (rehab_persen / 100.0).clamp(0.05, 1.5),
        satuan_umur_sisa,
    };
    config::simpan_asumsi(&a).map_err(|e| ServerFnError::new(format!("gagal menyimpan: {e}")))?;
    Ok(config::asumsi())
}

/// Hapus cache lokal (kembali ke mode contoh bila kredensial belum diisi).
#[server]
pub async fn reset_cache() -> Result<Vec<SyncStatus>, ServerFnError> {
    if sinkron::progres().berjalan {
        return Err(ServerFnError::new(
            "Sinkronisasi masih berjalan; tunggu sampai selesai.",
        ));
    }
    eprintln!("[audit] cache lokal dihapus");
    di_latar(|| {
        store::clear_all().map_err(|e| e.to_string())?;
        demo::pastikan();
        Ok::<_, String>(store::statuses_with_labels())
    })
    .await?
    .map_err(ServerFnError::new)
}

/// Satu halaman data aset dengan pencarian, saringan, dan pengurutan.
#[server]
#[allow(clippy::too_many_arguments)]
pub async fn get_asset_page(
    table: String,
    page: i64,
    per_page: i64,
    q: String,
    sort: Option<String>,
    dir: String,
    satker: String,
    kondisi: i64,
    status: String,
) -> Result<AssetPage, ServerFnError> {
    let Some(def) = catalog::find(&table) else {
        return Err(ServerFnError::new("tabel tidak dikenal"));
    };
    let label = def.label.to_string();
    di_latar(move || {
        demo::pastikan();
        let columns = crate::schema::columns_of(&table);
        let saring = store::Saring {
            tabel: table.clone(),
            q,
            satker,
            kondisi,
            status,
            sort: sort.unwrap_or_default(),
            desc: dir.eq_ignore_ascii_case("desc"),
            page,
            per_page,
        };
        let (rows, total) = store::halaman(&saring, &columns).map_err(|e| e.to_string())?;
        Ok::<_, String>(AssetPage {
            table,
            label,
            columns,
            rows,
            page: page.max(1),
            per_page: per_page.clamp(1, 200),
            total,
            is_demo: store::is_demo(),
        })
    })
    .await?
    .map_err(ServerFnError::new)
}

/// Jumlah baris pada sumber SLDK untuk satu tabel (untuk pratinjau sinkronisasi).
#[server]
pub async fn preview_row_count(table: String) -> Result<i64, ServerFnError> {
    if catalog::find(&table).is_none() {
        return Err(ServerFnError::new("tabel tidak dikenal"));
    }
    if !config::get().is_configured() {
        return Err(ServerFnError::new("Kredensial SLDK belum diisi."));
    }
    sldk::row_count(&table)
        .await
        .map_err(|e| ServerFnError::new(e.user_message()))
}

/// Pilihan tabel untuk kotak pilih di antarmuka.
pub fn table_options() -> Vec<(String, String)> {
    catalog::TABLES
        .iter()
        .map(|t| (t.table.to_string(), t.label.to_string()))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::mask;

    #[test]
    fn penyamaran_aman_untuk_karakter_non_ascii() {
        assert_eq!(mask("ABCDEFGH"), "••••EFGH");
        assert_eq!(mask("AB"), "••");
        assert_eq!(mask(""), "•");
        // Regresi: pemotongan berbasis bita dulu panik pada karakter multibita.
        assert_eq!(mask("kunci-éèêë"), "••••••éèêë");
    }
}
