use leptos::prelude::*;

use crate::components::chart::{
    BarList, BilahKondisi, Donut, GrafikTren, KolomTumpuk, Legenda, Seri, W_BAIK, W_RB, W_RR,
    W_TAK, W_UNGU,
};
use crate::components::ui::{
    DaftarSorotan, KartuRekomendasi, KepalaHalaman, Kpi, MuatAnalisis, PitaData, SectionCard,
};
use crate::dto::{Analisis, Skor, NH};
use crate::model::{
    format_desimal, format_number, format_persen, format_rupiah_short, format_unit,
};

/// Ringkasan eksekutif: posisi, sorotan, pandangan lima tahun, dan rekomendasi utama.
#[component]
pub fn OverviewPage() -> impl IntoView {
    view! { <MuatAnalisis isi=isi/> }
}

fn tanda(v: i64) -> String {
    if v > 0 {
        format!("+{}", format_number(v))
    } else {
        format_number(v)
    }
}

fn isi(a: Analisis) -> AnyView {
    let r = a.nasional.clone();
    let m = a.meta.clone();
    let akhir = m.tahun_akhir();
    let (predikat, nada_skor) = Skor::predikat(a.skor.total);
    let label_th: Vec<String> = (0..NH).map(|k| m.label_tahun(k)).collect();

    let donut: Vec<(String, f64)> = a
        .kategori
        .iter()
        .map(|c| (c.label.clone(), c.r.nilai))
        .collect();
    let tren: Vec<(i32, f64)> = a.tren.iter().map(|t| (t.tahun, t.unit as f64)).collect();
    let prov: Vec<(String, f64, String, Option<String>)> = a
        .provinsi
        .iter()
        .take(10)
        .map(|p| {
            (
                p.nama.clone(),
                p.r.nilai,
                format!(
                    "{} · {} unit",
                    format_rupiah_short(p.r.nilai),
                    format_number(p.r.unit)
                ),
                None,
            )
        })
        .collect();
    let perhatian: Vec<(String, f64, String, Option<String>)> = a
        .satker_skor
        .iter()
        .map(|s| {
            (
                s.nama.clone(),
                100.0 - s.skor,
                format!(
                    "skor {} · RB {}",
                    format_desimal(s.skor, 1),
                    format_number(s.rb)
                ),
                Some(format!("/satker/{}", s.kode)),
            )
        })
        .collect();
    let kondisi_kat: Vec<_> = a.kategori.iter().filter(|c| c.fisik).cloned().collect();
    let umur_label: Vec<String> = a.umur.iter().map(|u| u.label.clone()).collect();
    let umur_seri = vec![
        Seri {
            nama: "Baik".into(),
            warna: W_BAIK,
            nilai: a.umur.iter().map(|u| u.baik as f64).collect(),
        },
        Seri {
            nama: "Rusak ringan".into(),
            warna: W_RR,
            nilai: a.umur.iter().map(|u| u.rr as f64).collect(),
        },
        Seri {
            nama: "Rusak berat".into(),
            warna: W_RB,
            nilai: a.umur.iter().map(|u| u.rb as f64).collect(),
        },
        Seri {
            nama: "Tidak diisi".into(),
            warna: W_TAK,
            nilai: a.umur.iter().map(|u| u.tak as f64).collect(),
        },
    ];
    let delta = a.delta.clone();
    let rekomendasi: Vec<_> = a.rekomendasi.iter().take(4).cloned().collect();
    let n_rek = a.rekomendasi.len();

    view! {
        <KepalaHalaman
            judul="Ringkasan Eksekutif"
            uraian="Posisi Barang Milik Negara Kejaksaan RI, risiko utama, dan pandangan lima tahun ke depan."
        >
            <a class="btn-ghost" href="/rekomendasi">"✅ Rekomendasi"</a>
            <a class="btn-primary" href="/laporan">"🧾 Unduh laporan"</a>
        </KepalaHalaman>
        <PitaData meta=m.clone()/>

        <div class="grid grid-cols-2 gap-3 lg:grid-cols-4">
            <Kpi
                label="Total BMN"
                value=format!("{} unit", format_number(r.unit))
                hint=format!("{} satuan kerja · {} provinsi", format_number(m.total_satker), m.total_provinsi)
                icon="📦"
            />
            <Kpi
                label="Nilai Aset"
                value=format_rupiah_short(r.nilai)
                hint=format!("nilai buku {} · tersusut {}", format_rupiah_short(r.buku), format_persen(r.pct_susut()))
                icon="💰"
                tone="emerald"
            />
            <Kpi
                label="Kondisi Baik"
                value=format_persen(r.pct_baik())
                hint=format!("rusak ringan {} · rusak berat {}", format_number(r.rr), format_number(r.rb))
                icon="🩺"
                tone=if r.pct_baik() >= 85.0 { "emerald" } else if r.pct_baik() >= 70.0 { "amber" } else { "rose" }
            />
            <Kpi
                label="Skor Kesehatan BMN"
                value=format!("{} / 100", format_desimal(a.skor.total, 1))
                hint=format!("predikat {predikat} · rincian di Metodologi")
                icon="🎯"
                tone=nada_skor
            />
        </div>

        <div class="mt-3 grid grid-cols-2 gap-3 lg:grid-cols-4">
            <Kpi
                label="Rusak Berat"
                value=format!("{} unit", format_number(r.rb))
                hint=format!("senilai {} · {} dari aset fisik", format_rupiah_short(r.nilai_rb), format_persen(r.pct_rb()))
                icon="🛠️"
                tone="rose"
            />
            <Kpi
                label="Habis Masa Manfaat"
                value=format!("{} unit", format_number(r.habis))
                hint=format!("{} dari aset yang disusutkan", format_persen(r.pct_habis()))
                icon="⏳"
                tone="violet"
            />
            <Kpi
                label="Aset Idle"
                value=format!("{} unit", format_number(r.idle))
                hint=format!("senilai {}", format_rupiah_short(r.nilai_idle))
                icon="⏸️"
                tone="amber"
            />
            <Kpi
                label="Barang Hilang"
                value=format!("{} unit", format_number(r.hilang))
                hint=format!("senilai {}", format_rupiah_short(r.nilai_hilang))
                icon="🚨"
                tone="rose"
            />
        </div>

        {delta
            .map(|d| {
                view! {
                    <p class="mt-3 rounded-lg bg-ink-100 px-3 py-2 text-xs text-ink-600 dark:bg-ink-900 dark:text-ink-300">
                        <span class="font-semibold">{format!("Perubahan sejak posisi {}: ", d.tanggal)}</span>
                        {format!(
                            "{} unit · rusak berat {} · habis masa manfaat {} · idle {} · hilang {}.",
                            tanda(d.unit), tanda(d.rb), tanda(d.habis), tanda(d.idle), tanda(d.hilang),
                        )}
                    </p>
                }
            })}

        <div class="mt-4 grid gap-4 xl:grid-cols-5">
            <div class="xl:col-span-3">
                <SectionCard title="Sorotan Utama" subtitle="Dibaca dalam satu menit">
                    <DaftarSorotan butir=a.sorotan.clone()/>
                </SectionCard>
            </div>
            <div class="xl:col-span-2">
                <SectionCard title=format!("Pandangan sampai {akhir}") subtitle="Proyeksi dari data hari ini">
                    <dl class="space-y-3 text-sm">
                        <div class="flex items-baseline justify-between gap-3">
                            <dt class="text-ink-600 dark:text-ink-300">"Habis masa manfaat (tambahan)"</dt>
                            <dd class="text-lg font-bold tabular-nums">{format!("{} unit", format_unit(r.habis_5()))}</dd>
                        </div>
                        <div class="flex items-baseline justify-between gap-3">
                            <dt class="text-ink-600 dark:text-ink-300">"Perkiraan rusak berat baru"</dt>
                            <dd class="text-lg font-bold tabular-nums text-rose-600 dark:text-rose-400">{format!("±{} unit", format_unit(r.rb_5()))}</dd>
                        </div>
                        <div class="flex items-baseline justify-between gap-3">
                            <dt class="text-ink-600 dark:text-ink-300">"Kebutuhan (skenario moderat)"</dt>
                            <dd class="text-lg font-bold tabular-nums">{format!("{} unit", format_unit(r.ganti_total(1)))}</dd>
                        </div>
                        <div class="flex items-baseline justify-between gap-3">
                            <dt class="text-ink-600 dark:text-ink-300">"Perkiraan biaya moderat"</dt>
                            <dd class="text-lg font-bold tabular-nums">{format_rupiah_short(r.biaya_total(1))}</dd>
                        </div>
                        <div class="flex items-baseline justify-between gap-3 text-xs text-ink-500 dark:text-ink-400">
                            <dt>"Rentang biaya dasar – maksimal"</dt>
                            <dd class="tabular-nums">{format!("{} – {}", format_rupiah_short(r.biaya_total(0)), format_rupiah_short(r.biaya_total(2)))}</dd>
                        </div>
                    </dl>
                    <div class="no-print mt-4 flex flex-wrap gap-2">
                        <a class="btn-ghost" href="/prediksi">"🔮 Prediksi"</a>
                        <a class="btn-ghost" href="/kebutuhan">"🧮 Kebutuhan"</a>
                    </div>
                </SectionCard>
            </div>
        </div>

        <div class="mt-4 grid gap-4 lg:grid-cols-2">
            <SectionCard title="Habis Masa Manfaat per Tahun" subtitle="unit · * = sisa tahun berjalan">
                <KolomTumpuk
                    label=label_th.clone()
                    seri=vec![Seri { nama: "Habis masa manfaat".into(), warna: W_UNGU, nilai: r.habis_th.to_vec() }]
                    satuan="unit"
                />
            </SectionCard>
            <SectionCard title="Perkiraan Rusak Berat Baru per Tahun" subtitle="unit · ekspektasi model kerusakan">
                <KolomTumpuk
                    label=label_th
                    seri=vec![Seri { nama: "Rusak berat baru".into(), warna: W_RB, nilai: r.rb_th.to_vec() }]
                    satuan="unit"
                />
            </SectionCard>
        </div>

        <div class="mt-4">
            <SectionCard title="Rekomendasi Prioritas" subtitle=format!("{} teratas dari {} rekomendasi", rekomendasi.len(), n_rek)>
                <div class="grid gap-3 lg:grid-cols-2">
                    {rekomendasi.into_iter().map(|x| view! { <KartuRekomendasi r=x ringkas=true/> }).collect_view()}
                </div>
                <a class="no-print btn-ghost mt-3" href="/rekomendasi">"Lihat seluruh rekomendasi dan langkahnya →"</a>
            </SectionCard>
        </div>

        <div class="mt-4 grid gap-4 xl:grid-cols-3">
            <div class="xl:col-span-2">
                <SectionCard title="Kondisi per Kategori" subtitle="aset fisik · porsi baik dan jumlah rusak berat">
                    <ul class="space-y-2.5">
                        {kondisi_kat
                            .into_iter()
                            .map(|c| {
                                view! {
                                    <li class="grid grid-cols-[minmax(0,11rem)_1fr_auto] items-center gap-3 text-xs">
                                        <span class="truncate" title=c.label.clone()>
                                            <span aria-hidden="true">{c.icon.clone()}</span>
                                            " "
                                            {c.label.clone()}
                                        </span>
                                        <BilahKondisi baik=c.r.baik rr=c.r.rr rb=c.r.rb tak=c.r.tak/>
                                        <span class="w-36 text-right tabular-nums">
                                            {format!("{} baik · {} RB", format_persen(c.r.pct_baik()), format_number(c.r.rb))}
                                        </span>
                                    </li>
                                }
                            })
                            .collect_view()}
                    </ul>
                    <Legenda butir=vec![
                        ("Baik".to_string(), W_BAIK),
                        ("Rusak ringan".to_string(), W_RR),
                        ("Rusak berat".to_string(), W_RB),
                        ("Tidak diisi".to_string(), W_TAK),
                    ]/>
                </SectionCard>
            </div>
            <SectionCard title="Komposisi Nilai" subtitle="per kategori">
                <Donut data=donut label_pusat="Total".to_string() nilai_pusat=format_rupiah_short(r.nilai)/>
            </SectionCard>
        </div>

        <div class="mt-4 grid gap-4 lg:grid-cols-2">
            <SectionCard title="Profil Umur dan Kondisi" subtitle="unit aset fisik menurut umur">
                <KolomTumpuk label=umur_label seri=umur_seri satuan="unit"/>
            </SectionCard>
            <SectionCard title="Perolehan per Tahun" subtitle="unit yang masih tercatat, menurut tahun perolehan">
                <GrafikTren titik=tren satuan="unit"/>
            </SectionCard>
        </div>

        <div class="mt-4 grid gap-4 lg:grid-cols-2">
            <SectionCard title="Satuan Kerja Perlu Perhatian" subtitle="skor kesehatan terendah">
                <BarList data=perhatian warna=W_RB/>
                <a class="no-print btn-ghost mt-3" href="/satker">"Lihat seluruh satuan kerja →"</a>
            </SectionCard>
            <SectionCard title="Provinsi dengan Nilai Aset Terbesar" subtitle="10 teratas">
                <BarList data=prov/>
                <a class="no-print btn-ghost mt-3" href="/wilayah">"Lihat seluruh provinsi →"</a>
            </SectionCard>
        </div>
    }
    .into_any()
}
