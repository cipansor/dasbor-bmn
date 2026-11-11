//! Metadata kolom Master Aset BMN (106 kolom) sesuai lampiran Panduan Web
//! Service SLDK. Nama kolom yang terpenggal antarbaris pada dokumen sumber
//! (mis. `no_dok_bukti_k` + `epemilikan`) sudah disatukan kembali.

use crate::model::ColumnDef;

/// 106 kolom Master Aset BMN: (nama, uraian, tipe, panjang).
pub const COLUMN_SPECS: &[(&str, &str, &str, &str)] = &[
    ("kd_jns_bmn", "Kode Jenis BMN", "int", "-"),
    ("kd_brg", "Kode Barang", "text", "-"),
    ("no_aset", "No. Aset", "int", "-"),
    ("tercatat", "Tercatat", "varchar", "3"),
    ("merk", "Merk", "varchar", "500"),
    ("tipe", "Tipe", "varchar", "500"),
    ("rph_aset", "Nilai Aset (Rp)", "int", "-"),
    ("rph_susut", "Nilai Susut (Rp)", "int", "-"),
    ("rph_mutasi", "Nilai Mutasi (Rp)", "int", "-"),
    ("status_bmn_yn", "Status BMN", "nvarchar", "-"),
    ("bpybds_yn", "BPYBDS (Y/N)", "nvarchar", "-"),
    ("flag_sap", "Flag SAP", "nvarchar", "-"),
    ("jml_photo", "Jml. Foto", "int", "-"),
    ("ur_sskel", "Kelompok Aset", "varchar", "100"),
    ("alamat", "Alamat", "varchar", "200"),
    ("komplek", "Komplek", "varchar", "200"),
    ("kd_rtrw", "Kode RT/RW", "varchar", "15"),
    ("ur_kab", "Kabupaten", "varchar", "100"),
    ("ur_prov", "Provinsi", "varchar", "100"),
    ("kd_pos", "Kode Pos", "varchar", "10"),
    ("status_sbsn", "Status SBSN", "varchar", "50"),
    ("no_dana", "Nomor Dana", "varchar", "200"),
    ("tgl_dana", "Tanggal Dana", "datetime", "-"),
    ("asl_perlh", "Asal Perolehan", "varchar", "255"),
    ("tgl_perlh", "Tgl. Perolehan", "datetime", "-"),
    ("ur_sumber_dana", "Sumber Dana", "varchar", "30"),
    ("tgl_akhir_sbsn", "Tgl. Akhir SBSN", "datetime", "-"),
    ("status_bmn_idle", "BMN Idle", "varchar", "1"),
    ("rencana_hibah_yn", "Rencana Hibah (Y/N)", "varchar", "1"),
    ("kmk_sbsn", "KMK SBSN", "varchar", "100"),
    ("no_psp", "No. PSP", "varchar", "100"),
    ("tgl_psp", "Tgl. PSP", "datetime", "-"),
    ("luas", "Luas", "number", "-"),
    ("luas_tapak", "Luas Tapak Bangunan", "number", "-"),
    ("luas_tnhl", "Luas Tanah Lahan", "number", "-"),
    ("luas_tnhk", "Luas Tanah Kosong", "number", "-"),
    ("luas_tnhb", "Luas Tanah Bangunan", "number", "-"),
    ("jml_lantai", "Jml. Lantai", "number", "-"),
    ("jml_bdg", "Jml. Bangunan", "number", "-"),
    ("kd_jns_idle", "Kode Jenis Idle", "varchar", "2"),
    ("no_perkara_hukum", "No. Perkara Hukum", "varchar", "100"),
    (
        "no_dok_bukti_kepemilikan",
        "No. Dokumen Bukti Kepemilikan",
        "varchar",
        "200",
    ),
    (
        "jns_dok_bukti_kepemilikan",
        "Jenis Dokumen Bukti Kepemilikan",
        "varchar",
        "300",
    ),
    (
        "dihentikan_yn",
        "Penggunaan Dihentikan (Y/N)",
        "nvarchar",
        "1",
    ),
    ("sbsk", "SBSK", "number", "-"),
    ("tgl_rekam_pertama", "Tgl. Rekam Pertama", "datetime", "-"),
    ("tgl_rekam", "Tgl. Rekam", "datetime", "-"),
    ("tgl_hapus", "Tgl. Hapus", "datetime", "-"),
    ("bts_utara", "Batas Utara", "varchar", "100"),
    ("bts_selatan", "Batas Selatan", "varchar", "100"),
    ("bts_barat", "Batas Barat", "varchar", "100"),
    ("bts_timur", "Batas Timur", "varchar", "100"),
    ("bentuk", "Bentuk", "varchar", "30"),
    ("peruntukan_tnh", "Peruntukan Tanah", "varchar", "100"),
    ("topografi_kontur", "Kontur Topografi", "varchar", "30"),
    ("topografi_elevasi", "Elevasi", "varchar", "30"),
    ("aksesibilitas", "Aksesibilitas", "varchar", "30"),
    ("gps_latitude", "Latitude", "number", "-"),
    ("gps_longitude", "Longitude", "number", "-"),
    ("kemitraan_yn", "Kemitraan", "varchar", "1"),
    ("brg_hilang_yn", "Barang Hilang", "varchar", "1"),
    ("dktp_yn", "DKTP", "varchar", "1"),
    ("brg_rusak_yn", "Barang Rusak", "varchar", "1"),
    ("umur_sisa", "Umur Sisa", "float", "-"),
    (
        "no_dok_perolehan",
        "No. Dokumen Perolehan",
        "varchar",
        "200",
    ),
    ("jns_aset", "Jenis Aset", "varchar", "70"),
    ("negara", "Negara", "varchar", "128"),
    ("rph_perolehan", "Nilai Perolehan (Rp)", "int", "-"),
    ("rph_buku", "Nilai Buku (Rp)", "int", "-"),
    ("cara_perlh", "Cara Perolehan", "varchar", "128"),
    ("jns_pengguna", "Jenis Pengguna", "varchar", "128"),
    ("kd_unit_pengguna", "Kode Unit Pengguna", "varchar", "128"),
    ("nm_unit_pengguna", "Unit Pengguna", "varchar", "128"),
    ("ket_pengguna", "Keterangan Pengguna", "varchar", "128"),
    (
        "stat_dok_bukti_kepemilikan",
        "Status Dokumen Bukti Kepemilikan",
        "varchar",
        "128",
    ),
    (
        "tgl_dok_bukti_kepemilikan",
        "Tgl. Dokumen Bukti Kepemilikan",
        "varchar",
        "-",
    ),
    ("no_kib", "No. KIB", "varchar", "-"),
    ("kode_satker", "Kode Satker", "varchar", "20"),
    ("nama_satker", "Satker", "varchar", "100"),
    ("kode_sub_satker", "Kode Sub Satker", "varchar", "20"),
    ("nama_sub_satker", "Sub Satker", "varchar", "100"),
    ("ur_kondisi", "Kondisi", "varchar", "50"),
    ("nm_penghuni", "Nama Penghuni", "varchar", "50"),
    ("tgl_mulai_huni", "Tgl. Mulai Huni", "datetime", "-"),
    ("tgl_selesai_huni", "Tgl. Selesai Huni", "datetime", "-"),
    ("no_polisi", "No. Polisi", "varchar", "20"),
    ("urkpknl", "KPKNL", "varchar", "100"),
    ("ur_kanwil", "Kanwil", "varchar", "100"),
    ("ur_kl", "K/L", "varchar", "60"),
    ("ur_eselon1", "Eselon I", "varchar", "100"),
    ("ur_korwil", "Korwil", "varchar", "100"),
    ("kode_register", "Kode Register", "varchar", "100"),
    ("optimalisasi", "Optimalisasi", "number", "-"),
    ("tgl_buku_pertama", "Tgl. Buku Pertama", "datetime", "-"),
    ("jns_sertifikat", "Jenis Sertifikat", "varchar", "30"),
    ("intra_extra", "Intra/Extra", "text", "7"),
    ("ur_status", "Status Aset", "varchar", "100"),
    ("nama", "Nama Aset", "text", "500"),
    ("ur_kel", "Kelurahan", "varchar", "100"),
    ("ur_kec", "Kecamatan", "varchar", "100"),
    (
        "hapus_lainnya_yn",
        "Penghapusan Lainnya (Y/N)",
        "varchar",
        "1",
    ),
    ("konjas_yn", "Koordinasi dan Jasa (Y/N)", "varchar", "1"),
    (
        "properti_investasi_yn",
        "Properti Investasi (Y/N)",
        "varchar",
        "1",
    ),
    ("lokasi_ruang", "Lokasi Ruang", "varchar", "500"),
    ("luas_pemanfaatan", "Luas Pemanfaatan", "number", "-"),
    ("last_update", "Update Terakhir", "datetime", "-"),
];

/// Kolom untuk satu tabel; seluruh tabel memakai struktur yang sama.
pub fn columns_of(_table: &str) -> Vec<ColumnDef> {
    COLUMN_SPECS
        .iter()
        .map(|(name, label, tipe, len)| ColumnDef {
            name: name.to_string(),
            label: label.to_string(),
            r#type: tipe.to_string(),
            len: len.to_string(),
        })
        .collect()
}

/// Kolom ringkas untuk tampilan tabel data.
pub fn key_columns() -> Vec<ColumnDef> {
    let names = [
        "kd_brg",
        "nama",
        "ur_sskel",
        "merk",
        "rph_aset",
        "ur_kondisi",
        "ur_prov",
        "nama_satker",
        "tgl_perlh",
    ];
    columns_of("")
        .into_iter()
        .filter(|c| names.contains(&c.name.as_str()))
        .collect()
}

/// Jumlah kolom yang tersedia.
pub const COLUMN_COUNT: usize = COLUMN_SPECS.len();
