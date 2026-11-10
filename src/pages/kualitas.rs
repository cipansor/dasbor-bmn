use leptos::prelude::*;

use crate::components::chart::BarList;
use crate::components::ui::{
    kepala, Catatan, KepalaHalaman, Kpi, MuatAnalisis, PitaData, SectionCard, Sel, Tabel,
};
use crate::dto::Analisis;
use crate::model::{format_number, format_persen};

/// Mutu data dan kepatuhan administrasi.
#[component]
pub fn KualitasPage() -> impl IntoView {
    view! { <MuatAnalisis isi=isi/> }
}

fn isi(a: Analisis) -> AnyView {
    let r = a.nasional;
    let q = a.kualitas.clone();
    let catatan_sisa = q.catatan_umur_sisa.clone();
    let rasio_sisa = q.rasio_umur_sisa;
    let kolom: Vec<Vec<Sel>> = q
        .kolom
        .iter()
        .map(|k| {
            let pct = k.pct();
            vec![
                Sel::Teks(k.label.clone()),
                Sel::Teks(k.lingkup.clone()),
                Sel::unit(k.terisi),
                Sel::unit(k.total),
                Sel::unit(k.total - k.terisi),
                Sel::Lencana(
                    if k.total > 0 { format_persen(pct) } else { "–".into() },
                    if k.total == 0 { "slate" } else if pct >= 95.0 { "green" } else if pct >= 80.0 { "amber" } else { "red" },
                ),
            ]
        })
        .collect();
    let anomali: Vec<Vec<Sel>> = q
        .anomali
        .iter()
        .filter(|x| x.jumlah > 0)
        .map(|x| {
            vec![
                Sel::Ganda(x.judul.clone(), x.uraian.clone()),
                Sel::unit(x.jumlah),
                Sel::Angka(if x.nilai > 0.0 { crate::model::format_rupiah_short(x.nilai) } else { "–".into() }),
            ]
        })
        .collect();
    let per_kategori: Vec<(String, f64, String, Option<String>)> = q
        .per_kategori
        .iter()
        .map(|(nama, pct, unit)| {
            (nama.clone(), *pct, format!("{} · {} unit", format_persen(*pct), format_number(*unit)), None)
        })
        .collect();
    let dikecualikan = a.meta.dikecualikan;

    view! {
        <KepalaHalaman
            judul="Mutu Data"
            uraian="Kelengkapan isian yang menentukan ketepatan analitik, serta temuan administrasi yang perlu dibenahi di SIMAN."
        />
        <PitaData meta=a.meta.clone()/>

        <div class="grid grid-cols-2 gap-3 lg:grid-cols-4">
            <Kpi label="Kelengkapan Data Kunci" value=format_persen(r.pct_lengkap()) hint="kondisi, tanggal perolehan, nilai" icon="🧪" tone={if r.pct_lengkap() >= 95.0 { "emerald" } else { "amber" }}/>
            <Kpi label="Cakupan PSP" value=format_persen(r.pct_psp()) hint=format!("{} dari {} unit", format_number(r.psp_ya), format_number(r.psp_n)) icon="📜" tone={if r.pct_psp() >= 90.0 { "emerald" } else { "amber" }}/>
            <Kpi label="Tanah Berbukti Kepemilikan" value=format_persen(r.pct_sertifikat()) hint=format!("{} dari {} bidang", format_number(r.sertif_ya), format_number(r.tanah_n)) icon="🗺️" tone={if r.pct_sertifikat() >= 90.0 { "emerald" } else { "amber" }}/>
            <Kpi label="Kondisi Tidak Diisi" value=format!("{} unit", format_number(r.tak)) hint="aset fisik tanpa uraian kondisi" icon="❔" tone="rose"/>
        </div>

        <div class="mt-4 grid gap-4 xl:grid-cols-3">
            <div class="xl:col-span-2">
                <SectionCard title="Kelengkapan Isian" subtitle="isian yang dipakai analitik">
                    <Tabel kepala=kepala(&["Isian", "Lingkup", ">Terisi", ">Seharusnya", ">Kosong", "Cakupan"]) baris=kolom/>
                </SectionCard>
            </div>
            <SectionCard title="Kelengkapan per Kategori" subtitle="isian kunci">
                <BarList data=per_kategori/>
            </SectionCard>
        </div>

        <div class="mt-4">
            <SectionCard title="Temuan Administrasi dan Anomali" subtitle="hanya yang jumlahnya lebih dari nol">
                <Tabel kepala=kepala(&["Temuan", ">Jumlah", ">Nilai"]) baris=anomali kosong="Tidak ada temuan."/>
            </SectionCard>
        </div>

        <div class="mt-4">
            <Catatan>
                <p>
                    <span class="font-semibold">"Uji satuan sisa umur. "</span>
                    {catatan_sisa}
                    {(rasio_sisa > 0.0).then(|| format!(" (rasio median {}; nilai sekitar 1 berarti selaras)", crate::model::format_desimal(rasio_sisa, 2)))}
                </p>
                {(dikecualikan > 0)
                    .then(|| {
                        view! {
                            <p class="mt-1">
                                <span class="font-semibold">"Baris dikecualikan. "</span>
                                {format!("{} baris bertanggal hapus sah dikecualikan dari seluruh analisis.", format_number(dikecualikan))}
                            </p>
                        }
                    })}
                <p class="mt-1">
                    "Isian kosong tidak menghentikan analitik: aset tanpa tanggal perolehan memakai rata-rata umur kategorinya, dan aset tanpa kondisi diperlakukan sebagai baik pada proyeksi. Makin lengkap data, makin tajam hasilnya."
                </p>
            </Catatan>
        </div>
    }
    .into_any()
}
