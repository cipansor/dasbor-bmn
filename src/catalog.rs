use crate::model::{JenisKebutuhan, TableDef};

/// Katalog 15 tabel Master Aset BMN dari layanan SLDK SIMAN v2 (DJKN).
///
/// Selain identitas tabel, katalog memuat sifat analitik tiap kategori:
/// apakah kondisi fisiknya relevan, apakah disusutkan, jenis kebutuhan yang
/// timbul ketika aset rusak berat, dan masa manfaat bawaan. Masa manfaat bawaan
/// hanya dipakai sebagai cadangan ketika kolom `umur_sisa` dari SIMAN kosong;
/// angkanya penyederhanaan per kategori dari Tabel Masa Manfaat (KMK
/// 295/KM.6/2019 beserta perubahannya) dan perlu diverifikasi untuk kelompok
/// barang yang masa manfaatnya berbeda.
pub const TABLES: &[TableDef] = &[
    TableDef {
        table: "SIMAN2_M_ASET_TANAH",
        resource: "getAsetTanah",
        label: "Tanah",
        nama_data: "Master Aset Tanah",
        volume: "4.3 MB",
        icon: "🗺️",
        fisik: false,
        disusutkan: false,
        kebutuhan: JenisKebutuhan::TidakAda,
        masa_manfaat: 0.0,
    },
    TableDef {
        table: "SIMAN2_M_ASET_GEDUNG_BANGUNAN",
        resource: "getAsetGedungBangunan",
        label: "Gedung & Bangunan",
        nama_data: "Master Aset Gedung Bangunan",
        volume: "9 MB",
        icon: "🏢",
        fisik: true,
        disusutkan: true,
        kebutuhan: JenisKebutuhan::Rehabilitasi,
        masa_manfaat: 50.0,
    },
    TableDef {
        table: "SIMAN2_M_ASET_RUMAH",
        resource: "getAsetRumah",
        label: "Rumah Negara",
        nama_data: "Master Aset Rumah Negara",
        volume: "7.4 MB",
        icon: "🏠",
        fisik: true,
        disusutkan: true,
        kebutuhan: JenisKebutuhan::Rehabilitasi,
        masa_manfaat: 50.0,
    },
    TableDef {
        table: "SIMAN2_M_ASET_ANGKUTAN_BERMOTOR",
        resource: "getAsetAngkutanBermotor",
        label: "Angkutan Bermotor",
        nama_data: "Master Aset Angkutan Bermotor",
        volume: "18 MB",
        icon: "🚗",
        fisik: true,
        disusutkan: true,
        kebutuhan: JenisKebutuhan::Penggantian,
        masa_manfaat: 7.0,
    },
    TableDef {
        table: "SIMAN2_M_ASET_ALAT_BESAR",
        resource: "getAsetAlatBesar",
        label: "Alat Besar",
        nama_data: "Master Aset Alat Besar",
        volume: "2.2 MB",
        icon: "🚜",
        fisik: true,
        disusutkan: true,
        kebutuhan: JenisKebutuhan::Penggantian,
        masa_manfaat: 10.0,
    },
    TableDef {
        table: "SIMAN2_M_ASET_ALAT_PERSENJATAAN",
        resource: "getAsetAlatPersenjataan",
        label: "Alat Persenjataan",
        nama_data: "Master Aset Alat Persenjataan",
        volume: "1.5 MB",
        icon: "🎯",
        fisik: true,
        disusutkan: true,
        kebutuhan: JenisKebutuhan::Penggantian,
        masa_manfaat: 10.0,
    },
    TableDef {
        table: "SIMAN2_M_ASET_JALAN_DAN_JEMBATAN",
        resource: "getAsetJalandanJembatan",
        label: "Jalan & Jembatan",
        nama_data: "Master Aset Jalan dan Jembatan",
        volume: "260 KB",
        icon: "🛣️",
        fisik: true,
        disusutkan: true,
        kebutuhan: JenisKebutuhan::Rehabilitasi,
        masa_manfaat: 10.0,
    },
    TableDef {
        table: "SIMAN2_M_ASET_BANGUNAN_AIR",
        resource: "getAsetBangunanAir",
        label: "Bangunan Air",
        nama_data: "Master Aset Bangunan Air",
        volume: "300 KB",
        icon: "🌊",
        fisik: true,
        disusutkan: true,
        kebutuhan: JenisKebutuhan::Rehabilitasi,
        masa_manfaat: 30.0,
    },
    TableDef {
        table: "SIMAN2_M_ASET_INSTALASI_JARINGAN",
        resource: "getAsetInstalasiJaringan",
        label: "Instalasi & Jaringan",
        nama_data: "Master Aset Instalasi Jaringan",
        volume: "9.1 MB",
        icon: "🔌",
        fisik: true,
        disusutkan: true,
        kebutuhan: JenisKebutuhan::Penggantian,
        masa_manfaat: 20.0,
    },
    TableDef {
        table: "SIMAN2_M_ASET_KHUSUS_TIK",
        resource: "getAsetKhususTIK",
        label: "Peralatan TIK",
        nama_data: "Master Aset Khusus TIK",
        volume: "3.8 MB",
        icon: "💻",
        fisik: true,
        disusutkan: true,
        kebutuhan: JenisKebutuhan::Penggantian,
        masa_manfaat: 4.0,
    },
    TableDef {
        table: "SIMAN2_M_ASET_NON_TIK",
        resource: "getAsetNonTIK",
        label: "Peralatan Non-TIK",
        nama_data: "Master Aset Non TIK",
        volume: "867 MB",
        icon: "📦",
        fisik: true,
        disusutkan: true,
        kebutuhan: JenisKebutuhan::Penggantian,
        masa_manfaat: 5.0,
    },
    TableDef {
        table: "SIMAN2_M_ASET_ASET_TAK_BERWUJUD",
        resource: "getAsetTakBerwujud",
        label: "Aset Tak Berwujud",
        nama_data: "Master Aset Tak Berwujud",
        volume: "8.2 MB",
        icon: "💾",
        fisik: false,
        disusutkan: true,
        kebutuhan: JenisKebutuhan::TidakAda,
        masa_manfaat: 4.0,
    },
    TableDef {
        table: "SIMAN2_M_ASET_ASET_TETAP_LAINNYA",
        resource: "getAsetTetapLainnya",
        label: "Aset Tetap Lainnya",
        nama_data: "Master Aset Tetap Lainnya",
        volume: "142 MB",
        icon: "🗄️",
        fisik: false,
        disusutkan: false,
        kebutuhan: JenisKebutuhan::TidakAda,
        masa_manfaat: 0.0,
    },
    TableDef {
        table: "SIMAN2_M_ASET_ASET_TETAP_RENOVASI",
        resource: "getAsetTetapRenovasi",
        label: "Aset Tetap Renovasi",
        nama_data: "Master Aset Tetap Renovasi",
        volume: "4.8 MB",
        icon: "🧱",
        fisik: false,
        disusutkan: true,
        kebutuhan: JenisKebutuhan::TidakAda,
        masa_manfaat: 10.0,
    },
    TableDef {
        table: "SIMAN2_M_ASET_KDP",
        resource: "getAsetKDP",
        label: "Konstruksi Dalam Pengerjaan",
        nama_data: "Master Aset KDP",
        volume: "696 KB",
        icon: "🏗️",
        fisik: false,
        disusutkan: false,
        kebutuhan: JenisKebutuhan::TidakAda,
        masa_manfaat: 0.0,
    },
];

pub fn find(table: &str) -> Option<&'static TableDef> {
    TABLES.iter().find(|t| t.table == table)
}

/// Indeks tabel pada katalog.
pub fn indeks(table: &str) -> Option<usize> {
    TABLES.iter().position(|t| t.table == table)
}

pub fn label_of(table: &str) -> String {
    find(table)
        .map(|t| t.label.to_string())
        .unwrap_or_else(|| table.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn katalog_lengkap_dan_unik() {
        assert_eq!(TABLES.len(), 15);
        let mut nama: Vec<&str> = TABLES.iter().map(|t| t.table).collect();
        nama.sort();
        nama.dedup();
        assert_eq!(nama.len(), 15, "nama tabel harus unik");
        // Aset yang disusutkan wajib punya masa manfaat bawaan.
        for t in TABLES {
            assert_eq!(t.disusutkan, t.masa_manfaat > 0.0, "{}", t.table);
            if t.kebutuhan != JenisKebutuhan::TidakAda {
                assert!(t.fisik, "{} butuh kondisi fisik", t.table);
            }
        }
        assert_eq!(indeks("SIMAN2_M_ASET_TANAH"), Some(0));
        assert_eq!(indeks("TIDAK_ADA"), None);
    }
}
