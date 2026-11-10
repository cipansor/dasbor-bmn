use leptos::prelude::*;

use crate::components::ui::{
    kepala, KepalaHalaman, MuatAnalisis, PitaData, SectionCard, Sel, Tabel,
};
use crate::dto::{Analisis, BOBOT_SKOR, SKENARIO};
use crate::model::{format_desimal, format_persen};

/// Penjelasan cara hitung, asumsi, dan keterbatasan.
#[component]
pub fn MetodologiPage() -> impl IntoView {
    view! { <MuatAnalisis isi=isi/> }
}

#[component]
fn Butir(#[prop(into)] judul: String, children: Children) -> impl IntoView {
    view! {
        <div class="border-l-2 border-bmn-300 pl-3 dark:border-bmn-700">
            <p class="text-sm font-semibold">{judul}</p>
            <div class="mt-0.5 text-sm leading-relaxed text-ink-600 dark:text-ink-300">{children()}</div>
        </div>
    }
}

fn isi(a: Analisis) -> AnyView {
    let m = a.meta.clone();
    let asumsi = m.asumsi.clone().unwrap_or_default();
    let skor: Vec<Vec<Sel>> = BOBOT_SKOR
        .iter()
        .map(|(nama, bobot, uraian)| {
            vec![
                Sel::Teks(nama.to_string()),
                Sel::Angka(format_persen(bobot * 100.0)),
                Sel::Teks(uraian.to_string()),
            ]
        })
        .collect();
    let skenario: Vec<Vec<Sel>> = SKENARIO
        .iter()
        .map(|(nama, uraian)| vec![Sel::Teks(nama.to_string()), Sel::Teks(uraian.to_string())])
        .collect();
    let kategori: Vec<Vec<Sel>> = a
        .kategori
        .iter()
        .map(|c| {
            vec![
                Sel::Teks(format!("{} {}", c.icon, c.label)),
                Sel::Teks(if c.fisik { "ya".into() } else { "tidak".into() }),
                Sel::Teks(if c.disusutkan { "ya".into() } else { "tidak".into() }),
                Sel::Angka(if c.disusutkan { format_desimal(c.masa_manfaat, 1) } else { "–".into() }),
                Sel::Teks(c.kebutuhan.clone()),
            ]
        })
        .collect();

    view! {
        <KepalaHalaman
            judul="Metodologi"
            uraian="Bagaimana setiap angka pada dasbor dan laporan dihitung, asumsi yang dipakai, dan batas kepercayaannya."
        />
        <PitaData meta=m.clone()/>

        <div class="space-y-4">
            <SectionCard title="1. Sumber dan Cakupan Data" subtitle="SLDK SIMAN v2">
                <div class="space-y-3">
                    <Butir judul="Sumber">
                        "Lima belas tabel Master Aset BMN dari Web Service SLDK (DJKN, Kementerian Keuangan), masing-masing 106 kolom. Sumber diperbarui mingguan; dasbor menyajikan posisi pada sinkronisasi terakhir."
                    </Butir>
                    <Butir judul="Penyeragaman">
                        "Nama kolom diseragamkan ke huruf kecil; tanggal, angka, dan penanda Y/N dibaca dari berbagai bentuk penulisan. Kondisi dikelompokkan menjadi baik, rusak ringan, rusak berat, atau tidak diisi."
                    </Butir>
                    <Butir judul="Pengecualian">
                        "Baris dengan tanggal hapus yang sah dikeluarkan dari analisis. Tanggal bawaan basis data (mis. 1900-01-01) tidak dianggap tanggal sungguhan."
                    </Butir>
                </div>
            </SectionCard>

            <SectionCard title="2. Sifat Tiap Kategori" subtitle="menentukan ukuran mana yang berlaku">
                <Tabel kepala=kepala(&["Kategori", "Kondisi fisik relevan", "Disusutkan", ">Masa manfaat tipikal (th)", "Kebutuhan saat rusak berat"]) baris=kategori/>
                <p class="mt-2 text-xs text-ink-500 dark:text-ink-400">
                    "Masa manfaat tipikal dihitung dari data (median umur + sisa umur). Bila kolom sisa umur kosong, dipakai masa manfaat bawaan per kategori yang disederhanakan dari tabel masa manfaat penyusutan BMN; angka bawaan perlu diverifikasi untuk kelompok barang yang berbeda."
                </p>
            </SectionCard>

            <SectionCard title="3. Habis Masa Manfaat" subtitle="pasti, mengikuti SIMAN">
                <div class="space-y-3">
                    <Butir judul="Sisa masa manfaat">
                        {format!("Diambil dari kolom umur_sisa (satuan: {}). Aset disebut habis masa manfaat bila sisa umurnya nol. Tahun habis = tanggal data + sisa umur.", asumsi.satuan_umur_sisa)}
                    </Butir>
                    <Butir judul="Bila sisa umur kosong atau janggal">
                        "Sisa umur ditaksir dari porsi nilai buku terhadap nilai aset (penyusutan garis lurus tanpa nilai sisa), atau dari masa manfaat dikurangi umur. Sisa umur nol padahal nilai buku masih berarti dicatat sebagai anomali dan ditaksir ulang."
                    </Butir>
                    <Butir judul="Makna">
                        "Habis masa manfaat berarti nilai buku habis, bukan berarti barang rusak. Karena itu kebutuhan penggantian disajikan dalam tiga skenario."
                    </Butir>
                </div>
            </SectionCard>

            <SectionCard title="4. Prediksi Rusak Berat" subtitle="model kerusakan dua tahap">
                <div class="space-y-3">
                    <Butir judul="Gagasan">
                        "Aset bergerak dari baik ke rusak ringan lalu ke rusak berat. Dari potret umur dan kondisi seluruh aset hari ini, ditaksir (a) kurva peluang aset masih baik menurut umur (Weibull) dan (b) laju perpindahan dari rusak ringan ke rusak berat, untuk tiap kategori."
                    </Butir>
                    <Butir judul="Proyeksi per aset">
                        "Aset yang kini rusak ringan mendapat peluang rusak berat yang tinggi; aset baik mendapat peluang sesuai umurnya (makin tua makin besar). Peluang dijumlahkan menjadi ekspektasi jumlah unit per tahun, per kategori, dan per satuan kerja."
                    </Butir>
                    <Butir judul="Rasio Kerusakan terstandar Umur (RKU)">
                        "Jumlah aset tidak baik yang teramati di satuan kerja dibagi jumlah yang wajar menurut umur asetnya (dengan penyusutan statistik agar satuan kerja kecil tidak berayun). RKU menjadi faktor kerapuhan pada proyeksi satuan kerja tersebut, dibatasi 0,5–2,5."
                    </Butir>
                    <Butir judul="Keterbatasan">
                        "Aset rusak berat yang sudah dihapus tidak lagi tercatat sehingga laju kerusakan cenderung terlalu rendah — proyeksi adalah batas bawah. Model tidak mengetahui pemeliharaan besar atau pengadaan pengganti mendatang. Kategori bersampel sedikit memakai kurva gabungan dan ditandai."
                    </Butir>
                </div>
            </SectionCard>

            <SectionCard title="5. Analisis Kebutuhan" subtitle="tiga skenario">
                <Tabel kepala=kepala(&["Skenario", "Aset yang dihitung perlu diganti/direhabilitasi"]) baris=skenario/>
                <div class="mt-3 space-y-3">
                    <Butir judul="Harga satuan">
                        {format!(
                            "Peralatan: median nilai perolehan lima tahun terakhir pada kelompok barang yang sama (minimal lima unit); bila tidak ada, nilai perolehan aset dieskalasi {} per tahun (paling jauh 20 tahun). Bangunan dan prasarana: {} dari nilai aset sebagai biaya rehabilitasi berat.",
                            format_persen(asumsi.eskalasi * 100.0),
                            format_persen(asumsi.faktor_rehab * 100.0),
                        )}
                    </Butir>
                    <Butir judul="Tunggakan dan tahun">
                        "Tunggakan = kebutuhan yang sudah ada hari ini. Kebutuhan tahun berikutnya dihitung dari selisih peluang kumulatif antartahun dan biayanya dieskalasi. Tanda * pada tahun berjalan berarti sisa tahun sejak tanggal data."
                    </Butir>
                    <Butir judul="Di luar cakupan">
                        "Kebutuhan baru (pegawai/satuan kerja baru, perubahan standar barang dan standar kebutuhan), tanah, aset tak berwujud, dan KDP tidak dihitung sebagai kebutuhan penggantian."
                    </Butir>
                </div>
            </SectionCard>

            <SectionCard title="6. Forecast Perolehan" subtitle="deret tahunan">
                <div class="space-y-3">
                    <Butir judul="Metode">
                        "Rata-rata tiga tahun, tren linear, dan pemulusan eksponensial Holt teredam diadu dengan uji balik berjalan pada hingga 15 tahun terakhir. Metode bergalat terkecil dipakai; galat (sMAPE) ditampilkan."
                    </Butir>
                    <Butir judul="Rentang">
                        "Pita 80% diturunkan dari galat uji balik dan melebar seiring jauhnya tahun. Tahun berjalan belum penuh sehingga tidak ikut riwayat; taksiran setahun penuhnya ditandai *."
                    </Butir>
                </div>
            </SectionCard>

            <SectionCard title="7. Skor Kesehatan BMN" subtitle="0–100 · indikator internal dasbor, bukan indikator resmi">
                <Tabel kepala=kepala(&["Komponen", ">Bobot", "Cara hitung"]) baris=skor/>
                <p class="mt-2 text-xs text-ink-500 dark:text-ink-400">
                    "Predikat: Baik ≥ 80, Perlu Perhatian 65–80, Kritis < 65. Satuan kerja dengan kurang dari 20 unit tidak masuk daftar peringkat terbawah."
                </p>
            </SectionCard>

            <SectionCard title="8. Rujukan Ketentuan" subtitle="penunjuk awal untuk tindak lanjut">
                <ul class="list-disc space-y-1 pl-5 text-sm text-ink-600 dark:text-ink-300">
                    <li>"PP 27/2014 jo. PP 28/2020 — Pengelolaan Barang Milik Negara/Daerah."</li>
                    <li>"PMK 181/PMK.06/2016 — Penatausahaan BMN."</li>
                    <li>"PMK 65/PMK.06/2017 dan KMK 295/KM.6/2019 beserta perubahannya — penyusutan dan tabel masa manfaat."</li>
                    <li>"PMK 153/PMK.06/2021 — Perencanaan Kebutuhan BMN (RKBMN)."</li>
                    <li>"PMK 83/PMK.06/2016 — Pemusnahan dan Penghapusan BMN."</li>
                    <li>"PMK 71/PMK.06/2016 — BMN yang tidak digunakan untuk tugas dan fungsi (idle)."</li>
                    <li>"PMK 246/PMK.06/2014 beserta perubahannya — Penggunaan BMN (PSP)."</li>
                    <li>"PMK 207/PMK.06/2021 — Pengawasan dan Pengendalian BMN."</li>
                </ul>
                <p class="mt-2 text-xs text-ink-500 dark:text-ink-400">
                    "Daftar ini penunjuk awal. Pastikan ketentuan yang berlaku saat tindak lanjut karena peraturan dapat berubah."
                </p>
            </SectionCard>
        </div>
    }
    .into_any()
}
