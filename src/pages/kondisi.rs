use leptos::prelude::*;

use crate::components::chart::{GrafikKurva, KolomTumpuk, Seri, W_BAIK, W_RB, W_RR, W_TAK};
use crate::components::ui::{
    kepala, Catatan, KepalaHalaman, Kpi, MuatAnalisis, PitaData, SectionCard, Sel, Tabel, TabelAset,
};
use crate::dto::Analisis;
use crate::model::{format_desimal, format_number, format_persen, format_rupiah_short};

/// Kondisi dan risiko: sebaran kondisi, masa manfaat, kurva kerusakan, dan tunggakan.
#[component]
pub fn KondisiPage() -> impl IntoView {
    view! { <MuatAnalisis isi=isi/> }
}

fn isi(a: Analisis) -> AnyView {
    let r = a.nasional.clone();
    let baris: Vec<Vec<Sel>> = a
        .kategori
        .iter()
        .map(|c| {
            let k = &c.r;
            vec![
                Sel::Teks(format!("{} {}", c.icon, c.label)),
                Sel::unit(k.unit),
                Sel::persen_bila(k.pct_baik(), c.fisik && k.kondisi_diketahui() > 0),
                Sel::Angka(if c.fisik { format_number(k.rr) } else { "–".into() }),
                Sel::Angka(if c.fisik { format_number(k.rb) } else { "–".into() }),
                Sel::Angka(if c.fisik { format_rupiah_short(k.nilai_rb) } else { "–".into() }),
                Sel::Angka(if c.disusutkan { format_number(k.habis) } else { "–".into() }),
                Sel::persen_bila(k.pct_habis(), c.disusutkan && k.susut_unit > 0),
                Sel::Angka(if k.umur_n > 0 { format_desimal(k.umur_rata(), 1) } else { "–".into() }),
                Sel::Angka(if c.disusutkan { format_desimal(c.masa_manfaat, 1) } else { "–".into() }),
                Sel::unit(k.idle),
                Sel::unit(k.hilang),
            ]
        })
        .collect();
    let total = vec![
        Sel::Teks("Seluruh kategori".into()),
        Sel::unit(r.unit),
        Sel::persen(r.pct_baik()),
        Sel::unit(r.rr),
        Sel::unit(r.rb),
        Sel::rupiah(r.nilai_rb),
        Sel::unit(r.habis),
        Sel::persen(r.pct_habis()),
        Sel::Angka(format_desimal(r.umur_rata(), 1)),
        Sel::Angka("".into()),
        Sel::unit(r.idle),
        Sel::unit(r.hilang),
    ];

    let umur_label: Vec<String> = a.umur.iter().map(|u| u.label.clone()).collect();
    let persen = |f: fn(&crate::dto::UmurPita) -> i64| -> Vec<f64> {
        a.umur
            .iter()
            .map(|u| {
                let n = (u.baik + u.rr + u.rb + u.tak).max(1) as f64;
                100.0 * f(u) as f64 / n
            })
            .collect()
    };
    let umur_seri = vec![
        Seri { nama: "Baik".into(), warna: W_BAIK, nilai: persen(|u| u.baik) },
        Seri { nama: "Rusak ringan".into(), warna: W_RR, nilai: persen(|u| u.rr) },
        Seri { nama: "Rusak berat".into(), warna: W_RB, nilai: persen(|u| u.rb) },
        Seri { nama: "Tidak diisi".into(), warna: W_TAK, nilai: persen(|u| u.tak) },
    ];

    let rku: Vec<Vec<Sel>> = a
        .satker_rku
        .iter()
        .map(|s| {
            vec![
                Sel::Tautan(s.nama.clone(), format!("/satker/{}", s.kode)),
                Sel::Teks(s.prov.clone()),
                Sel::unit(s.unit),
                Sel::persen(s.pct_baik),
                Sel::unit(s.rb),
                Sel::Angka(format!("{}×", format_desimal(s.rku, 2))),
                Sel::skor(s.skor),
            ]
        })
        .collect();

    let model = StoredValue::new(a.model.clone());
    let (pilih, set_pilih) = signal(0usize);
    let pilihan: Vec<(usize, String)> = a
        .model
        .iter()
        .enumerate()
        .map(|(i, m)| (i, m.label.clone()))
        .collect();

    view! {
        <KepalaHalaman
            judul="Kondisi & Risiko"
            uraian="Keadaan fisik aset hari ini, porsi yang sudah habis masa manfaat, dan pola kerusakan menurut umur."
        />
        <PitaData meta=a.meta.clone()/>

        <div class="grid grid-cols-2 gap-3 lg:grid-cols-4">
            <Kpi label="Baik" value=format!("{} unit", format_number(r.baik)) hint=format_persen(r.pct_baik()) icon="🟢" tone="emerald"/>
            <Kpi label="Rusak Ringan" value=format!("{} unit", format_number(r.rr)) hint=format!("{} · perlu perbaikan", format_persen(r.pct_rr())) icon="🟡" tone="amber"/>
            <Kpi label="Rusak Berat" value=format!("{} unit", format_number(r.rb)) hint=format!("{} · senilai {}", format_persen(r.pct_rb()), format_rupiah_short(r.nilai_rb)) icon="🔴" tone="rose"/>
            <Kpi label="Habis Masa Manfaat" value=format!("{} unit", format_number(r.habis)) hint=format!("{} · nilai perolehan {}", format_persen(r.pct_habis()), format_rupiah_short(r.nilai_habis)) icon="⏳" tone="violet"/>
        </div>

        <div class="mt-4">
            <SectionCard title="Kondisi dan Masa Manfaat per Kategori" subtitle="RB = rusak berat · MM = masa manfaat">
                <Tabel
                    kepala=kepala(&["Kategori", ">Unit", ">Baik", ">Rusak ringan", ">RB", ">Nilai RB", ">Habis MM", ">% habis", ">Umur rata-rata (th)", ">MM tipikal (th)", ">Idle", ">Hilang"])
                    baris=baris
                    total=total
                />
                <p class="mt-2 text-xs text-ink-500 dark:text-ink-400">
                    "Tanda – berarti ukuran tidak berlaku: kondisi fisik tidak relevan (tanah, KDP, aset tak berwujud) atau kategori tidak disusutkan. MM tipikal dihitung dari data (median umur + sisa umur)."
                </p>
            </SectionCard>
        </div>

        <div class="mt-4 grid gap-4 lg:grid-cols-2">
            <SectionCard title="Kondisi Menurut Umur" subtitle="% unit aset fisik pada tiap pita umur">
                <KolomTumpuk label=umur_label seri=umur_seri satuan="unit"/>
            </SectionCard>
            <SectionCard title="Kurva Kerusakan: Data vs Model" subtitle="dasar prediksi rusak berat">
                <label class="no-print mb-2 block text-xs">
                    <span class="mr-2 font-medium text-ink-600 dark:text-ink-300">"Kategori"</span>
                    <select
                        class="input inline-block w-auto"
                        on:change=move |ev| set_pilih.set(event_target_value(&ev).parse().unwrap_or(0))
                    >
                        {pilihan.into_iter().map(|(i, l)| view! { <option value=i.to_string()>{l}</option> }).collect_view()}
                    </select>
                </label>
                {move || {
                    model
                        .with_value(|m| m.get(pilih.get()).cloned())
                        .map(|m| {
                            view! {
                                <GrafikKurva titik=m.kurva.clone()/>
                                <p class="mt-2 text-xs text-ink-600 dark:text-ink-300">
                                    {format!(
                                        "{}: separuh aset mulai tidak baik pada umur ±{} tahun; setelah rusak ringan, rata-rata {} tahun sebelum tercatat rusak berat. Sampel {} unit, selisih model–data {} poin persentase{}.",
                                        m.label,
                                        format_desimal(m.median_mulai, 1),
                                        format_desimal(m.lama_rr, 1),
                                        format_number(m.n),
                                        format_desimal(m.mae, 1),
                                        if m.gabungan { " (memakai kurva gabungan karena sampel kategori sedikit)" } else { "" },
                                    )}
                                </p>
                            }
                        })
                }}
            </SectionCard>
        </div>

        <div class="mt-4">
            <SectionCard title="Kerusakan di Atas Kewajaran Umur" subtitle="Rasio Kerusakan terstandar Umur (RKU) tertinggi">
                <Tabel
                    kepala=kepala(&["Satuan kerja", "Provinsi", ">Unit", ">Baik", ">RB", ">RKU", "Skor"])
                    baris=rku
                />
                <div class="mt-3">
                    <Catatan>
                        "RKU membandingkan jumlah aset tidak baik yang teramati dengan jumlah yang wajar menurut umur aset satuan kerja itu. RKU 1,5× berarti kerusakan 50% lebih banyak daripada satuan kerja lain dengan aset seumur — petunjuk untuk memeriksa pemeliharaan, beban pakai, atau ketepatan pencatatan kondisi."
                    </Catatan>
                </div>
            </SectionCard>
        </div>

        <div class="mt-4">
            <SectionCard title="Aset Rusak Berat Bernilai Terbesar" subtitle="calon utama usulan penghapusan/penggantian">
                <TabelAset aset=a.aset_rb.clone() dengan_peluang=false/>
            </SectionCard>
        </div>
    }
    .into_any()
}
