//! Sorotan dan rekomendasi tindak lanjut berbasis aturan.
//!
//! Setiap rekomendasi memuat temuan berangka, langkah yang dapat dikerjakan,
//! dan dasar ketentuannya. Dasar ketentuan dicantumkan sebagai rujukan awal;
//! pembaca tetap perlu memastikan ketentuan yang berlaku saat tindak lanjut.

#![cfg(feature = "ssr")]

use crate::dto::*;
use crate::model::{format_number, format_persen, format_rupiah_kata, format_unit};

const D_PENGHAPUSAN: &str =
    "PP 27/2014 jo. PP 28/2020; PMK 83/PMK.06/2016 (pemusnahan dan penghapusan BMN)";
const D_PERENCANAAN: &str = "PMK 153/PMK.06/2021 (perencanaan kebutuhan BMN)";
const D_IDLE: &str =
    "PMK 71/PMK.06/2016 (BMN yang tidak digunakan untuk tugas dan fungsi); PMK 153/PMK.06/2021";
const D_PENGGUNAAN: &str = "PMK 246/PMK.06/2014 beserta perubahannya (penggunaan BMN)";
const D_PENGAMANAN: &str = "PP 27/2014 jo. PP 28/2020 (pengamanan dan pemeliharaan BMN)";
const D_PENATAUSAHAAN: &str = "PMK 181/PMK.06/2016 (penatausahaan BMN)";
const D_WASDAL: &str = "PMK 207/PMK.06/2021 (pengawasan dan pengendalian BMN)";
const D_PENYUSUTAN: &str =
    "PMK 65/PMK.06/2017 (penyusutan BMN); KMK 295/KM.6/2019 beserta perubahannya (tabel masa manfaat)";

fn n(v: i64) -> String {
    format_number(v)
}

fn nama_satker(daftar: &[&SatkerBaris]) -> Vec<String> {
    daftar.iter().take(5).map(|s| s.nama.clone()).collect()
}

fn teratas<'a>(
    satker: &'a [SatkerBaris],
    nilai: impl Fn(&SatkerBaris) -> f64,
) -> Vec<&'a SatkerBaris> {
    let mut v: Vec<&SatkerBaris> = satker.iter().filter(|s| nilai(s) > 0.0).collect();
    v.sort_by(|a, b| nilai(b).total_cmp(&nilai(a)).then(a.nama.cmp(&b.nama)));
    v.truncate(5);
    v
}

/// Kategori dengan nilai terbesar menurut suatu ukuran.
fn kategori_teratas(a: &Analisis, ukuran: impl Fn(&KategoriBaris) -> f64) -> Option<&KategoriBaris> {
    a.kategori
        .iter()
        .filter(|c| ukuran(c) > 0.0)
        .max_by(|x, y| ukuran(x).total_cmp(&ukuran(y)))
}

fn anomali<'a>(a: &'a Analisis, kunci: &str) -> Option<&'a Anomali> {
    a.kualitas
        .anomali
        .iter()
        .find(|x| x.kunci == kunci && x.jumlah > 0)
}

/// Rata-rata kebutuhan biaya per tahun anggaran pada cakrawala (skenario `s`).
pub fn biaya_per_tahun(r: &Ringkas, s: usize) -> f64 {
    r.biaya_total(s) / HORIZON as f64
}

/// Rekomendasi yang berlaku untuk satu kelompok aset (nasional atau satuan kerja).
fn aturan_umum(
    r: &Ringkas,
    meta: &Meta,
    lingkup: &str,
    kategori: &[KategoriBaris],
    satker: &[SatkerBaris],
) -> Vec<Rekomendasi> {
    let mut out = Vec::new();
    let akhir = meta.tahun_akhir();

    if r.rb > 0 {
        let top = teratas(satker, |s| s.rb as f64);
        out.push(Rekomendasi {
            prioritas: if r.pct_rb() >= 5.0 || r.rb >= 500 { 1 } else { 2 },
            bidang: "Penghapusan".into(),
            judul: "Tuntaskan aset rusak berat yang masih tercatat".into(),
            temuan: format!(
                "{} unit aset {lingkup} berkondisi rusak berat ({} dari aset fisik) senilai {} masih tercatat.",
                n(r.rb),
                format_persen(r.pct_rb()),
                format_rupiah_kata(r.nilai_rb)
            ),
            tindakan: vec![
                "Tetapkan penghentian penggunaan, lalu ajukan usulan penghapusan atau pemindahtanganan (penjualan melalui lelang) bagi aset rusak berat yang tidak ekonomis diperbaiki.".into(),
                "Keluarkan aset rusak berat dari usulan pemeliharaan agar anggaran pemeliharaan tidak terserap aset yang tidak layak pakai.".into(),
                "Jadwalkan penyelesaian bertahap dan pantau tunggakan per satuan kerja setiap triwulan.".into(),
            ],
            dasar: D_PENGHAPUSAN.into(),
            satker: nama_satker(&top),
        });
    }

    if r.hilang > 0 {
        let top = teratas(satker, |s| s.hilang as f64);
        out.push(Rekomendasi {
            prioritas: 1,
            bidang: "Pengamanan".into(),
            judul: "Selesaikan barang hilang dan perkuat pengamanan".into(),
            temuan: format!(
                "{} unit aset {lingkup} tercatat hilang senilai {}.",
                n(r.hilang),
                format_rupiah_kata(r.nilai_hilang)
            ),
            tindakan: vec![
                "Proses tuntutan ganti kerugian terhadap pihak yang bertanggung jawab dan lengkapi berita acara serta laporan kehilangan.".into(),
                "Ajukan penghapusan dari daftar barang setelah proses pertanggungjawaban selesai.".into(),
                "Perketat pengamanan fisik dan administrasi untuk barang mudah berpindah (laptop, sepeda motor, senjata).".into(),
            ],
            dasar: format!("{D_PENGAMANAN}; PMK 83/PMK.06/2016; ketentuan tuntutan ganti kerugian negara"),
            satker: nama_satker(&top),
        });
    }

    if r.susut_unit > 0 && r.pct_habis() >= 25.0 {
        let kat = kategori
            .iter()
            .filter(|c| c.kebutuhan != "-" && c.r.susut_unit >= 20)
            .max_by(|x, y| x.r.pct_habis().total_cmp(&y.r.pct_habis()));
        let top = teratas(satker, |s| s.habis as f64);
        out.push(Rekomendasi {
            prioritas: if r.pct_habis() >= 50.0 { 1 } else { 2 },
            bidang: "Perencanaan Kebutuhan".into(),
            judul: "Susun rencana peremajaan aset yang habis masa manfaat".into(),
            temuan: format!(
                "{} unit ({}) aset {lingkup} yang disusutkan sudah habis masa manfaat, dan {} unit lagi menyusul sampai {akhir}.{}",
                n(r.habis),
                format_persen(r.pct_habis()),
                format_unit(r.habis_5()),
                kat.map(|c| format!(
                    " Porsi tertinggi pada {} ({}).",
                    c.label,
                    format_persen(c.r.pct_habis())
                ))
                .unwrap_or_default()
            ),
            tindakan: vec![
                "Dahulukan penggantian aset habis masa manfaat yang sudah rusak ringan; aset tua yang masih baik dievaluasi kelayakannya, tidak otomatis diganti.".into(),
                format!("Tuangkan jadwal peremajaan {}–{akhir} ke dalam RKBMN pengadaan sesuai standar barang dan standar kebutuhan.", meta.tahun0 + 1),
                "Tinjau ulang aset yang nilai bukunya nol tetapi masih dipakai untuk memastikan keselamatan dan kelayakan operasional.".into(),
            ],
            dasar: format!("{D_PERENCANAAN}; {D_PENYUSUTAN}"),
            satker: nama_satker(&top),
        });
    }

    let rb5 = r.rb_5();
    if rb5 >= 1.0 && (rb5 >= 0.4 * r.rb as f64 || rb5 >= 0.02 * r.fisik as f64) {
        let kat = kategori
            .iter()
            .max_by(|x, y| x.r.rb_5().total_cmp(&y.r.rb_5()));
        let top = teratas(satker, |s| s.rb5);
        out.push(Rekomendasi {
            prioritas: if rb5 >= r.rb as f64 { 1 } else { 2 },
            bidang: "Pemeliharaan".into(),
            judul: "Cegah lonjakan rusak berat dengan pemeliharaan terarah".into(),
            temuan: format!(
                "Model kerusakan memperkirakan sekitar {} unit aset {lingkup} lagi menjadi rusak berat sampai {akhir} (kini {} unit).{} Saat ini {} unit rusak ringan paling berisiko.",
                format_unit(rb5),
                n(r.rb),
                kat.filter(|c| c.r.rb_5() >= 1.0)
                    .map(|c| format!(" Terbanyak pada {} (±{} unit).", c.label, format_unit(c.r.rb_5())))
                    .unwrap_or_default(),
                n(r.rr)
            ),
            tindakan: vec![
                "Arahkan anggaran pemeliharaan ke aset rusak ringan dan aset mendekati akhir masa manfaat; perbaikan dini lebih murah daripada penggantian.".into(),
                "Gunakan daftar aset berisiko tinggi pada dasbor sebagai dasar pemeriksaan fisik berkala.".into(),
                "Catat hasil pemeliharaan agar kondisi di SIMAN mutakhir dan proyeksi makin tepat.".into(),
            ],
            dasar: format!("{D_PENGAMANAN}; {D_PERENCANAAN}"),
            satker: nama_satker(&top),
        });
    }

    if r.idle > 0 {
        let top = teratas(satker, |s| s.idle as f64);
        out.push(Rekomendasi {
            prioritas: if r.pct_idle() >= 3.0 { 1 } else { 2 },
            bidang: "Pemanfaatan".into(),
            judul: "Optimalkan aset idle sebelum mengadakan yang baru".into(),
            temuan: format!(
                "{} unit aset {lingkup} berstatus idle ({} dari seluruh aset) senilai {}.",
                n(r.idle),
                format_persen(r.pct_idle()),
                format_rupiah_kata(r.nilai_idle)
            ),
            tindakan: vec![
                "Tawarkan aset idle berkondisi baik kepada satuan kerja yang membutuhkan (alih status penggunaan) sebelum menyetujui pengadaan sejenis.".into(),
                "Serahkan kepada Pengelola Barang aset idle yang tidak lagi diperlukan untuk tugas dan fungsi.".into(),
                "Kaji pemanfaatan (sewa/pinjam pakai) untuk tanah dan bangunan idle agar menghasilkan PNBP.".into(),
            ],
            dasar: D_IDLE.into(),
            satker: nama_satker(&top),
        });
    }

    if r.psp_n > 0 && r.pct_psp() < 90.0 {
        out.push(Rekomendasi {
            prioritas: if r.pct_psp() < 60.0 { 1 } else { 2 },
            bidang: "Penatausahaan".into(),
            judul: "Percepat Penetapan Status Penggunaan".into(),
            temuan: format!(
                "Baru {} aset {lingkup} yang memiliki nomor Penetapan Status Penggunaan ({} dari {} unit).",
                format_persen(r.pct_psp()),
                n(r.psp_ya),
                n(r.psp_n)
            ),
            tindakan: vec![
                "Ajukan permohonan PSP secara kolektif per satuan kerja, dahulukan tanah, bangunan, dan kendaraan.".into(),
                "Tetapkan target cakupan PSP per semester dan pantau lewat dasbor.".into(),
            ],
            dasar: D_PENGGUNAAN.into(),
            satker: vec![],
        });
    }

    if r.tanah_n > 0 && r.sertif_ya < r.tanah_n {
        let belum = r.tanah_n - r.sertif_ya;
        out.push(Rekomendasi {
            prioritas: if r.pct_sertifikat() < 80.0 { 1 } else { 2 },
            bidang: "Pengamanan".into(),
            judul: "Tuntaskan bukti kepemilikan tanah".into(),
            temuan: format!(
                "{} dari {} bidang tanah {lingkup} belum memiliki bukti kepemilikan tercatat (cakupan {}).",
                n(belum),
                n(r.tanah_n),
                format_persen(r.pct_sertifikat())
            ),
            tindakan: vec![
                "Daftarkan sertipikat atas nama Pemerintah RI c.q. Kejaksaan RI melalui Kantor Pertanahan, berkoordinasi dengan KPKNL.".into(),
                "Lengkapi nomor dan jenis dokumen kepemilikan di SIMAN bagi tanah yang sebenarnya sudah bersertipikat.".into(),
            ],
            dasar: D_PENGAMANAN.into(),
            satker: vec![],
        });
    }

    if r.lengkap_n > 0 && r.pct_lengkap() < 97.0 {
        out.push(Rekomendasi {
            prioritas: if r.pct_lengkap() < 90.0 { 2 } else { 3 },
            bidang: "Penatausahaan".into(),
            judul: "Lengkapi isian kunci data aset".into(),
            temuan: format!(
                "Kelengkapan isian kunci (kondisi, tanggal perolehan, nilai) aset {lingkup} baru {}; {} unit belum lengkap.",
                format_persen(r.pct_lengkap()),
                n(r.lengkap_n - r.lengkap_ya)
            ),
            tindakan: vec![
                "Perbaiki isian kondisi, tanggal perolehan, dan nilai pada SIMAN/SAKTI melalui inventarisasi.".into(),
                "Data yang lengkap menaikkan ketepatan proyeksi kerusakan dan kebutuhan pada dasbor ini.".into(),
            ],
            dasar: D_PENATAUSAHAAN.into(),
            satker: vec![],
        });
    }
    out
}

/// Susun sorotan dan rekomendasi tingkat nasional.
pub fn susun(a: &Analisis, satker: &[SatkerBaris]) -> (Vec<Sorotan>, Vec<Rekomendasi>) {
    let r = &a.nasional;
    if r.unit == 0 {
        return (vec![], vec![]);
    }
    let m = &a.meta;
    let akhir = m.tahun_akhir();
    let mut so = Vec::new();
    let mut tambah = |nada: &str, judul: &str, isi: String| {
        so.push(Sorotan {
            nada: nada.into(),
            judul: judul.into(),
            isi,
        });
    };

    tambah(
        "info",
        "Portofolio",
        format!(
            "{} unit BMN senilai {} (nilai buku {}) dikelola {} satuan kerja di {} provinsi.",
            n(r.unit),
            format_rupiah_kata(r.nilai),
            format_rupiah_kata(r.buku),
            n(m.total_satker),
            n(m.total_provinsi)
        ),
    );
    if r.kondisi_diketahui() > 0 {
        let nada = if r.pct_baik() >= 85.0 {
            "positif"
        } else if r.pct_baik() >= 70.0 {
            "waspada"
        } else {
            "kritis"
        };
        tambah(
            nada,
            "Kondisi fisik",
            format!(
                "{} aset fisik berkondisi baik; {} unit rusak ringan dan {} unit rusak berat (senilai {}).",
                format_persen(r.pct_baik()),
                n(r.rr),
                n(r.rb),
                format_rupiah_kata(r.nilai_rb)
            ),
        );
    }
    if r.susut_unit > 0 {
        let kat = a
            .kategori
            .iter()
            .filter(|c| c.kebutuhan != "-" && c.r.susut_unit >= 20)
            .max_by(|x, y| x.r.pct_habis().total_cmp(&y.r.pct_habis()));
        tambah(
            if r.pct_habis() >= 50.0 {
                "kritis"
            } else if r.pct_habis() >= 25.0 {
                "waspada"
            } else {
                "positif"
            },
            "Masa manfaat",
            format!(
                "{} aset yang disusutkan ({} unit) sudah habis masa manfaat; {} unit lagi habis sampai {akhir}.{}",
                format_persen(r.pct_habis()),
                n(r.habis),
                format_unit(r.habis_5()),
                kat.map(|c| format!(" Tertinggi: {} ({}).", c.label, format_persen(c.r.pct_habis())))
                    .unwrap_or_default()
            ),
        );
    }
    if r.rb_5() >= 1.0 {
        let kat = kategori_teratas(a, |c| c.r.rb_5());
        tambah(
            if r.rb_5() >= r.rb as f64 { "kritis" } else { "waspada" },
            "Prediksi rusak berat",
            format!(
                "Sekitar {} unit lagi diperkirakan menjadi rusak berat sampai {akhir}{}.",
                format_unit(r.rb_5()),
                kat.map(|c| format!(", terbanyak pada {} (±{} unit)", c.label, format_unit(c.r.rb_5())))
                    .unwrap_or_default()
            ),
        );
    }
    let per_tahun = biaya_per_tahun(r, 1);
    if r.biaya_total(2) > 0.0 {
        let rasio = if a.kapasitas.nilai_per_tahun > 0.0 {
            per_tahun / a.kapasitas.nilai_per_tahun
        } else {
            0.0
        };
        tambah(
            if rasio > 1.5 {
                "kritis"
            } else if rasio > 1.0 {
                "waspada"
            } else {
                "info"
            },
            "Kebutuhan 5 tahun",
            format!(
                "Kebutuhan penggantian dan rehabilitasi sampai {akhir} berkisar {} (dasar) hingga {} (maksimal); skenario moderat {} atau rata-rata {} per tahun{}.",
                format_rupiah_kata(r.biaya_total(0)),
                format_rupiah_kata(r.biaya_total(2)),
                format_rupiah_kata(r.biaya_total(1)),
                format_rupiah_kata(per_tahun),
                if rasio > 0.0 {
                    format!(
                        ", setara {} kali rata-rata perolehan {}–{}",
                        crate::model::format_desimal(rasio, 1),
                        a.kapasitas.tahun_awal,
                        a.kapasitas.tahun_akhir
                    )
                } else {
                    String::new()
                }
            ),
        );
    }
    if r.idle > 0 || r.hilang > 0 {
        tambah(
            if r.hilang > 0 { "waspada" } else { "info" },
            "Idle dan hilang",
            format!(
                "{} unit idle senilai {} dapat dioptimalkan; {} unit tercatat hilang senilai {}.",
                n(r.idle),
                format_rupiah_kata(r.nilai_idle),
                n(r.hilang),
                format_rupiah_kata(r.nilai_hilang)
            ),
        );
    }
    let kritis: Vec<&SatkerBaris> = satker
        .iter()
        .filter(|s| s.unit >= 20 && s.skor < 65.0)
        .collect();
    let perhatian = satker
        .iter()
        .filter(|s| s.unit >= 20 && s.skor >= 65.0 && s.skor < 80.0)
        .count();
    tambah(
        if kritis.is_empty() { "positif" } else { "waspada" },
        "Sebaran kinerja",
        format!(
            "Skor kesehatan BMN nasional {}. {} satuan kerja berpredikat kritis dan {} perlu perhatian dari {} satuan kerja.",
            crate::model::format_desimal(a.skor.total, 1),
            n(kritis.len() as i64),
            n(perhatian as i64),
            n(m.total_satker)
        ),
    );
    if r.psp_n > 0 || r.tanah_n > 0 {
        tambah(
            if r.pct_psp() < 60.0 || (r.tanah_n > 0 && r.pct_sertifikat() < 80.0) {
                "waspada"
            } else {
                "info"
            },
            "Tertib administrasi",
            format!(
                "Cakupan Penetapan Status Penggunaan {}; tanah berbukti kepemilikan {}; kelengkapan data kunci {}.",
                format_persen(r.pct_psp()),
                if r.tanah_n > 0 {
                    format_persen(r.pct_sertifikat())
                } else {
                    "-".into()
                },
                format_persen(r.pct_lengkap())
            ),
        );
    }
    if m.tabel_tersinkron < 15 && !m.is_demo {
        tambah(
            "waspada",
            "Cakupan data",
            format!(
                "Baru {} dari 15 kategori yang tersinkron; angka pada dasbor belum mencakup seluruh BMN.",
                m.tabel_tersinkron
            ),
        );
    }
    if a.kualitas.rasio_umur_sisa > 0.0 && !(0.6..=1.6).contains(&a.kualitas.rasio_umur_sisa) {
        tambah("waspada", "Satuan sisa umur", a.kualitas.catatan_umur_sisa.clone());
    }

    // Rekomendasi.
    let mut rek = aturan_umum(r, m, "Kejaksaan RI", &a.kategori, satker);

    if a.kapasitas.nilai_per_tahun > 0.0 && per_tahun > a.kapasitas.nilai_per_tahun {
        let rasio = per_tahun / a.kapasitas.nilai_per_tahun;
        let top = teratas(satker, |s| s.biaya5);
        rek.push(Rekomendasi {
            prioritas: if rasio >= 1.5 { 1 } else { 2 },
            bidang: "Perencanaan Kebutuhan".into(),
            judul: "Tutup kesenjangan antara kebutuhan dan kapasitas pengadaan".into(),
            temuan: format!(
                "Kebutuhan skenario moderat rata-rata {} per tahun, sedangkan perolehan kategori sejenis {}–{} rata-rata {} per tahun (rasio {}).",
                format_rupiah_kata(per_tahun),
                a.kapasitas.tahun_awal,
                a.kapasitas.tahun_akhir,
                format_rupiah_kata(a.kapasitas.nilai_per_tahun),
                crate::model::format_desimal(rasio, 1)
            ),
            tindakan: vec![
                "Urutkan pemenuhan berdasarkan risiko: tunggakan rusak berat lebih dulu, lalu aset rusak ringan yang habis masa manfaat.".into(),
                "Ajukan penyesuaian pagu belanja modal dengan dukungan angka kebutuhan per satuan kerja dari dasbor ini.".into(),
                "Kurangi kebutuhan lewat pengalihan aset idle dan perpanjangan umur pakai melalui pemeliharaan.".into(),
            ],
            dasar: D_PERENCANAAN.into(),
            satker: nama_satker(&top),
        });
    }

    if !kritis.is_empty() {
        let mut bawah = kritis.clone();
        bawah.sort_by(|x, y| x.skor.total_cmp(&y.skor));
        rek.push(Rekomendasi {
            prioritas: if kritis.len() as f64 >= 0.1 * satker.len() as f64 { 1 } else { 2 },
            bidang: "Pengawasan dan Pengendalian".into(),
            judul: "Bina satuan kerja berpredikat kritis".into(),
            temuan: format!(
                "{} satuan kerja memiliki skor kesehatan BMN di bawah 65.",
                n(kritis.len() as i64)
            ),
            tindakan: vec![
                "Minta rencana aksi perbaikan per satuan kerja dengan target terukur (rusak berat, idle, PSP, sertipikat).".into(),
                "Jadwalkan pemantauan dan penertiban oleh Pengguna Barang; gunakan halaman rincian satuan kerja sebagai bahan.".into(),
            ],
            dasar: D_WASDAL.into(),
            satker: nama_satker(&bawah),
        });
    }

    let rapuh: Vec<&SatkerBaris> = {
        let mut v: Vec<&SatkerBaris> = satker
            .iter()
            .filter(|s| s.unit >= 20 && s.rku >= 1.4)
            .collect();
        v.sort_by(|x, y| y.rku.total_cmp(&x.rku));
        v
    };
    if !rapuh.is_empty() {
        rek.push(Rekomendasi {
            prioritas: 2,
            bidang: "Pemeliharaan".into(),
            judul: "Evaluasi satuan kerja dengan kerusakan di atas kewajaran umur".into(),
            temuan: format!(
                "{} satuan kerja memiliki aset tidak baik sedikitnya 1,4 kali dari yang wajar menurut umur asetnya (Rasio Kerusakan terstandar Umur).",
                n(rapuh.len() as i64)
            ),
            tindakan: vec![
                "Periksa pola pemeliharaan, beban pemakaian, dan ketepatan pencatatan kondisi di satuan kerja tersebut.".into(),
                "Bandingkan dengan satuan kerja berasio rendah untuk menyebarkan praktik pemeliharaan yang baik.".into(),
            ],
            dasar: D_WASDAL.into(),
            satker: nama_satker(&rapuh),
        });
    }

    if let Some(x) = anomali(a, "rb_aktif") {
        rek.push(Rekomendasi {
            prioritas: 2,
            bidang: "Penghapusan".into(),
            judul: "Hentikan penggunaan aset rusak berat".into(),
            temuan: format!(
                "{} unit aset rusak berat senilai {} belum ditandai dihentikan dari penggunaan.",
                n(x.jumlah),
                format_rupiah_kata(x.nilai)
            ),
            tindakan: vec![
                "Terbitkan penetapan penghentian penggunaan sebagai langkah awal usulan penghapusan.".into(),
                "Pastikan aset rusak berat tidak lagi dioperasikan demi keselamatan.".into(),
            ],
            dasar: D_PENGHAPUSAN.into(),
            satker: vec![],
        });
    }
    if let Some(x) = anomali(a, "sengketa") {
        rek.push(Rekomendasi {
            prioritas: 2,
            bidang: "Pengamanan".into(),
            judul: "Kawal aset yang sedang berperkara".into(),
            temuan: format!(
                "{} aset senilai {} memuat nomor perkara hukum.",
                n(x.jumlah),
                format_rupiah_kata(x.nilai)
            ),
            tindakan: vec![
                "Lakukan pengamanan hukum dan fisik; libatkan Jaksa Pengacara Negara untuk pendampingan.".into(),
                "Mutakhirkan status perkara dan dokumen kepemilikan pada SIMAN.".into(),
            ],
            dasar: D_PENGAMANAN.into(),
            satker: vec![],
        });
    }
    if let Some(x) = anomali(a, "kdp_mangkrak") {
        rek.push(Rekomendasi {
            prioritas: 2,
            bidang: "Penatausahaan".into(),
            judul: "Selesaikan konstruksi dalam pengerjaan yang berlarut".into(),
            temuan: format!(
                "{} KDP senilai {} belum direklasifikasi lebih dari tiga tahun.",
                n(x.jumlah),
                format_rupiah_kata(x.nilai)
            ),
            tindakan: vec![
                "Percepat penyelesaian dan serah terima, lalu reklasifikasi menjadi aset definitif.".into(),
                "Tetapkan status KDP yang dihentikan permanen dan ungkapkan pada laporan keuangan.".into(),
            ],
            dasar: D_PENATAUSAHAAN.into(),
            satker: vec![],
        });
    }
    if let Some(x) = anomali(a, "rumah_kosong") {
        rek.push(Rekomendasi {
            prioritas: 3,
            bidang: "Pemanfaatan".into(),
            judul: "Tertibkan penghunian rumah negara".into(),
            temuan: format!(
                "{} rumah negara senilai {} tidak mencatat penghuni.",
                n(x.jumlah),
                format_rupiah_kata(x.nilai)
            ),
            tindakan: vec![
                "Verifikasi status hunian dan terbitkan surat izin penghunian bagi yang berhak.".into(),
                "Rumah yang kosong dialihkan kepada pegawai yang membutuhkan atau diusulkan alih fungsi.".into(),
            ],
            dasar: D_PENGGUNAAN.into(),
            satker: vec![],
        });
    }
    if let Some(x) = anomali(a, "nilai_kecil") {
        rek.push(Rekomendasi {
            prioritas: 3,
            bidang: "Penatausahaan".into(),
            judul: "Nilai kembali aset bernilai Rp1".into(),
            temuan: format!("{} unit aset tercatat bernilai Rp1 atau kosong.", n(x.jumlah)),
            tindakan: vec![
                "Usulkan penilaian agar nilai wajar tercatat dan portofolio tidak tersaji terlalu rendah.".into(),
            ],
            dasar: D_PENATAUSAHAAN.into(),
            satker: vec![],
        });
    }

    rek.sort_by(|x, y| x.prioritas.cmp(&y.prioritas));
    (so, rek)
}

/// Rekomendasi untuk satu satuan kerja.
pub fn untuk_satker(
    r: &Ringkas,
    meta: &Meta,
    kategori: &[KategoriBaris],
    rku_tinggi: bool,
) -> Vec<Rekomendasi> {
    let mut rek = aturan_umum(r, meta, "satuan kerja ini", kategori, &[]);
    if rku_tinggi {
        rek.push(Rekomendasi {
            prioritas: 2,
            bidang: "Pemeliharaan".into(),
            judul: "Tinjau pola pemeliharaan".into(),
            temuan: format!(
                "Aset tidak baik {} kali dari yang wajar menurut umur aset satuan kerja ini.",
                crate::model::format_desimal(r.rku(), 1)
            ),
            tindakan: vec![
                "Periksa pelaksanaan pemeliharaan rutin, beban pemakaian, dan ketepatan pencatatan kondisi.".into(),
                "Susun jadwal pemeliharaan preventif untuk kendaraan dan peralatan utama.".into(),
            ],
            dasar: D_WASDAL.into(),
            satker: vec![],
        });
    }
    rek.sort_by(|x, y| x.prioritas.cmp(&y.prioritas));
    rek
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dasar() -> (Analisis, Vec<SatkerBaris>) {
        let mut r = Ringkas {
            unit: 1000,
            nilai: 5e11,
            buku: 3e11,
            fisik: 800,
            baik: 600,
            rr: 120,
            rb: 80,
            nilai_rb: 2e10,
            idle: 40,
            nilai_idle: 9e9,
            hilang: 3,
            nilai_hilang: 5e7,
            susut_unit: 700,
            habis: 300,
            psp_ya: 500,
            psp_n: 1000,
            sertif_ya: 30,
            tanah_n: 50,
            lengkap_ya: 900,
            lengkap_n: 1000,
            ..Default::default()
        };
        r.rb_th = [5.0, 20.0, 22.0, 25.0, 27.0, 30.0];
        r.biaya_kini = [2e10, 3e10, 6e10];
        r.biaya_th = [[1e9; NH], [2e9; NH], [4e9; NH]];
        let a = Analisis {
            meta: Meta {
                tahun0: 2026,
                total_satker: 2,
                total_provinsi: 1,
                tabel_tersinkron: 15,
                ..Default::default()
            },
            skor: Skor::dari(&r),
            nasional: r,
            kapasitas: Kapasitas {
                nilai_per_tahun: 4e9,
                tahun_awal: 2021,
                tahun_akhir: 2025,
                ..Default::default()
            },
            ..Default::default()
        };
        let s = vec![
            SatkerBaris {
                kode: "A".into(),
                nama: "Kejari A".into(),
                unit: 500,
                rb: 60,
                skor: 55.0,
                rku: 1.8,
                biaya5: 3e10,
                ..Default::default()
            },
            SatkerBaris {
                kode: "B".into(),
                nama: "Kejari B".into(),
                unit: 500,
                rb: 20,
                skor: 88.0,
                rku: 0.8,
                biaya5: 1e10,
                ..Default::default()
            },
        ];
        (a, s)
    }

    #[test]
    fn rekomendasi_terurut_dan_berangka() {
        let (a, s) = dasar();
        let (sorotan, rek) = susun(&a, &s);
        assert!(sorotan.len() >= 6);
        assert!(rek.len() >= 7, "{}", rek.len());
        assert!(rek.windows(2).all(|w| w[0].prioritas <= w[1].prioritas));
        assert!(rek.iter().all(|r| !r.tindakan.is_empty() && !r.dasar.is_empty()));
        let hapus = rek.iter().find(|r| r.bidang == "Penghapusan").unwrap();
        assert_eq!(hapus.prioritas, 1, "10% rusak berat = prioritas tinggi");
        assert!(hapus.temuan.contains("80 unit"));
        assert_eq!(hapus.satker[0], "Kejari A", "satker dengan tunggakan terbesar di depan");
        // Kesenjangan pendanaan: moderat (3e10 + 6×2e9)/5 = 8,4 M/th vs 4 M/th.
        let gap = rek.iter().find(|r| r.judul.contains("kesenjangan")).unwrap();
        assert!(gap.temuan.contains("2,1"), "{}", gap.temuan);
        assert!(rek.iter().any(|r| r.bidang == "Pengawasan dan Pengendalian"));
        assert!(rek.iter().any(|r| r.judul.contains("kewajaran umur")));
    }

    #[test]
    fn portofolio_sehat_tidak_memicu_rekomendasi() {
        let r = Ringkas {
            unit: 100,
            fisik: 100,
            baik: 100,
            susut_unit: 100,
            psp_ya: 100,
            psp_n: 100,
            lengkap_ya: 100,
            lengkap_n: 100,
            ..Default::default()
        };
        let rek = untuk_satker(&r, &Meta::default(), &[], false);
        assert!(rek.is_empty(), "{rek:?}");
    }

    #[test]
    fn data_kosong_tanpa_sorotan() {
        let (s, r) = susun(&Analisis::default(), &[]);
        assert!(s.is_empty() && r.is_empty());
    }
}
