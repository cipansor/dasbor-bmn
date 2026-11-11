use leptos::prelude::*;

use crate::components::chart::{KolomTumpuk, Seri, W_KEDUA, W_UTAMA};
use crate::components::ui::{
    kepala, Catatan, KepalaHalaman, Kpi, MuatAnalisis, PitaData, SectionCard, Sel, Tabel,
};
use crate::dto::{Analisis, HORIZON, NH, SKENARIO};
use crate::model::{
    format_desimal, format_number, format_persen, format_rupiah_short, format_unit,
};

/// Analisis kebutuhan lima tahun: penggantian dan rehabilitasi per skenario.
#[component]
pub fn KebutuhanPage() -> impl IntoView {
    view! { <MuatAnalisis isi=isi/> }
}

fn isi(a: Analisis) -> AnyView {
    let m = a.meta.clone();
    let akhir = m.tahun_akhir();
    let asumsi = m.asumsi.clone().unwrap_or_default();
    let a = StoredValue::new(a);
    // Skenario moderat menjadi bawaan: jalan tengah antara kondisi dan umur.
    let (s, set_s) = signal(1usize);

    let tombol = move || {
        SKENARIO
            .iter()
            .enumerate()
            .map(|(i, (nama, _))| {
                view! {
                    <button
                        type="button"
                        class=move || {
                            format!(
                                "rounded-lg px-3.5 py-2 text-sm font-medium transition-colors {}",
                                if s.get() == i {
                                    "bg-bmn-600 text-white"
                                } else {
                                    "bg-white text-ink-700 hover:bg-ink-50 dark:bg-ink-900 dark:text-ink-200 dark:hover:bg-ink-800"
                                },
                            )
                        }
                        aria-pressed=move || (s.get() == i).to_string()
                        on:click=move |_| set_s.set(i)
                    >
                        {*nama}
                    </button>
                }
            })
            .collect_view()
    };

    let badan = move || {
        let sk = s.get();
        {
            let a = a.get_value();
            let r = a.nasional;
            let m = a.meta.clone();
            let akhir = m.tahun_akhir();
            let label_th: Vec<String> = std::iter::once("Tunggakan".to_string())
                .chain((0..NH).map(|k| m.label_tahun(k)))
                .collect();
            let unit: Vec<f64> = std::iter::once(r.ganti_kini[sk])
                .chain(r.ganti_th[sk].iter().copied())
                .collect();
            let biaya: Vec<f64> = std::iter::once(r.biaya_kini[sk])
                .chain(r.biaya_th[sk].iter().copied())
                .collect();
            let per_tahun = r.biaya_total(sk) / HORIZON as f64;
            let rasio = if a.kapasitas.nilai_per_tahun > 0.0 {
                per_tahun / a.kapasitas.nilai_per_tahun
            } else {
                0.0
            };
            let acuan = (a.kapasitas.nilai_per_tahun > 0.0).then(|| {
                (
                    a.kapasitas.nilai_per_tahun,
                    format!(
                        "rata-rata perolehan {}–{}",
                        a.kapasitas.tahun_awal, a.kapasitas.tahun_akhir
                    ),
                )
            });

            let mut kep: Vec<String> = vec!["Kategori".into(), "Jenis".into(), ">Tunggakan".into()];
            kep.extend((0..NH).map(|k| format!(">{}", m.label_tahun(k))));
            kep.push(">Total unit".into());
            kep.push(">Perkiraan biaya".into());
            let kep = kepala(&kep.iter().map(|x| x.as_str()).collect::<Vec<_>>());
            let label_biaya = label_th.clone();
            let kapasitas = a.kapasitas.clone();
            let baris: Vec<Vec<Sel>> = a
                .kategori
                .iter()
                .filter(|c| c.kebutuhan != "-")
                .map(|c| {
                    let mut sel = vec![
                        Sel::Teks(format!("{} {}", c.icon, c.label)),
                        Sel::Teks(c.kebutuhan.clone()),
                        Sel::unit_f(c.r.ganti_kini[sk]),
                    ];
                    sel.extend(c.r.ganti_th[sk].iter().map(|v| Sel::unit_f(*v)));
                    sel.push(Sel::unit_f(c.r.ganti_total(sk)));
                    sel.push(Sel::rupiah(c.r.biaya_total(sk)));
                    sel
                })
                .collect();
            let mut total = vec![
                Sel::Teks("Seluruh kategori".into()),
                Sel::Teks("".into()),
                Sel::unit_f(r.ganti_kini[sk]),
            ];
            total.extend(r.ganti_th[sk].iter().map(|v| Sel::unit_f(*v)));
            total.push(Sel::unit_f(r.ganti_total(sk)));
            total.push(Sel::rupiah(r.biaya_total(sk)));

            let mut kelompok: Vec<_> = a.kelompok_butuh.iter().collect();
            kelompok.sort_by(|x, y| y.biaya[sk].total_cmp(&x.biaya[sk]));
            let kelompok: Vec<Vec<Sel>> = kelompok
                .into_iter()
                .filter(|k| k.unit[sk] >= 0.5)
                .take(20)
                .map(|k| {
                    vec![
                        Sel::Ganda(k.kelompok.clone(), k.kategori.clone()),
                        Sel::unit(k.unit_total),
                        Sel::unit_f(k.unit[sk]),
                        Sel::persen(100.0 * k.unit[sk] / k.unit_total.max(1) as f64),
                        Sel::Ganda(format_rupiah_short(k.harga), k.sumber_harga.clone()),
                        Sel::rupiah(k.biaya[sk]),
                        Sel::Angka(if k.idle_baik > 0 {
                            format_number(k.idle_baik)
                        } else {
                            "–".into()
                        }),
                    ]
                })
                .collect();

            let mut satker = a.satker_butuh.clone();
            if sk != 1 {
                // Daftar teratas disusun menurut skenario moderat; urutan tetap informatif.
                satker.truncate(10);
            }
            let satker: Vec<Vec<Sel>> = satker
                .iter()
                .map(|x| {
                    vec![
                        Sel::Tautan(x.nama.clone(), format!("/satker/{}", x.kode)),
                        Sel::Teks(x.prov.clone()),
                        Sel::unit(x.unit),
                        Sel::unit(x.rb),
                        Sel::unit(x.habis),
                        Sel::unit_f(x.ganti5),
                        Sel::rupiah(x.biaya5),
                    ]
                })
                .collect();
            let idle_baik: i64 = a.kelompok_butuh.iter().map(|k| k.idle_baik).sum();
            let banding: Vec<Vec<Sel>> = (0..3)
                .map(|i| {
                    vec![
                        Sel::Teks(SKENARIO[i].0.to_string()),
                        Sel::unit_f(r.ganti_kini[i]),
                        Sel::unit_f(r.ganti_total(i)),
                        Sel::rupiah(r.biaya_total(i)),
                        Sel::rupiah(r.biaya_total(i) / HORIZON as f64),
                    ]
                })
                .collect();

            view! {
                <p class="mt-2 text-sm text-ink-600 dark:text-ink-300">
                    <span class="font-semibold">{format!("Skenario {}: ", SKENARIO[sk].0)}</span>
                    {SKENARIO[sk].1}
                </p>

                <div class="mt-3 grid grid-cols-2 gap-3 lg:grid-cols-4">
                    <Kpi label="Tunggakan Saat Ini" value=format!("{} unit", format_unit(r.ganti_kini[sk])) hint=format!("perkiraan {}", format_rupiah_short(r.biaya_kini[sk])) icon="📌" tone="rose"/>
                    <Kpi label=format!("Kebutuhan s.d. {akhir}") value=format!("{} unit", format_unit(r.ganti_total(sk))) hint="termasuk tunggakan" icon="🧮"/>
                    <Kpi label="Perkiraan Biaya" value=format_rupiah_short(r.biaya_total(sk)) hint=format!("rata-rata {} per tahun", format_rupiah_short(per_tahun)) icon="💰" tone="emerald"/>
                    <Kpi
                        label="Dibanding Perolehan Historis"
                        value={if rasio > 0.0 { format!("{}×", format_desimal(rasio, 1)) } else { "–".into() }}
                        hint=format!("rata-rata {}–{}: {}/tahun", kapasitas.tahun_awal, kapasitas.tahun_akhir, format_rupiah_short(kapasitas.nilai_per_tahun))
                        icon="⚖️"
                        tone={if rasio > 1.5 { "rose" } else if rasio > 1.0 { "amber" } else { "emerald" }}
                    />
                </div>

                <div class="mt-4 grid gap-4 lg:grid-cols-2">
                    <SectionCard title="Kebutuhan Unit per Tahun" subtitle="penggantian + rehabilitasi">
                        <KolomTumpuk label=label_th seri=vec![Seri { nama: "Unit".into(), warna: W_UTAMA, nilai: unit }] satuan="unit"/>
                    </SectionCard>
                    <SectionCard title="Perkiraan Biaya per Tahun" subtitle="rupiah · harga dieskalasi per tahun">
                        {match acuan {
                            Some(ac) => view! { <KolomTumpuk label=label_biaya seri=vec![Seri { nama: "Biaya".into(), warna: W_KEDUA, nilai: biaya }] satuan="rupiah" acuan=ac/> }.into_any(),
                            None => view! { <KolomTumpuk label=label_biaya seri=vec![Seri { nama: "Biaya".into(), warna: W_KEDUA, nilai: biaya }] satuan="rupiah"/> }.into_any(),
                        }}
                    </SectionCard>
                </div>

                <div class="mt-4">
                    <SectionCard title="Kebutuhan per Kategori" subtitle="unit per tahun anggaran">
                        <Tabel kepala=kep baris=baris total=total/>
                    </SectionCard>
                </div>

                <div class="mt-4">
                    <SectionCard title="Apa yang Perlu Diadakan" subtitle="20 kelompok barang dengan biaya terbesar">
                        <Tabel
                            kepala=kepala(&["Kelompok barang", ">Unit ada", ">Kebutuhan 5 th", ">% dari yang ada", ">Harga satuan acuan", ">Perkiraan biaya", ">Idle baik"])
                            baris=kelompok
                        />
                        <p class="mt-2 text-xs text-ink-500 dark:text-ink-400">
                            {format!(
                                "Kolom Idle baik = unit idle berkondisi baik pada kelompok yang sama; seluruhnya {} unit dapat dialihkan lebih dulu sebelum pengadaan.",
                                format_number(idle_baik),
                            )}
                        </p>
                    </SectionCard>
                </div>

                <div class="mt-4">
                    <SectionCard title="Satuan Kerja dengan Kebutuhan Terbesar" subtitle="skenario moderat · 10 teratas">
                        <Tabel
                            kepala=kepala(&["Satuan kerja", "Provinsi", ">Unit", ">RB", ">Habis MM", ">Kebutuhan 5 th (unit)", ">Perkiraan biaya"])
                            baris=satker
                        />
                        <div class="no-print mt-3 flex flex-wrap gap-2">
                            <a class="btn-ghost" href="/satker">"Rincian seluruh satuan kerja →"</a>
                            <a class="btn-ghost" href="/api/kebutuhan.csv">"⬇️ Kebutuhan per satuan kerja × kategori (CSV)"</a>
                        </div>
                    </SectionCard>
                </div>

                <div class="mt-4 grid gap-4 lg:grid-cols-2">
                    <SectionCard title="Kebutuhan Pemeliharaan" subtitle="bahan RKBMN pemeliharaan">
                        <dl class="space-y-2 text-sm">
                            <div class="flex justify-between gap-3">
                                <dt>"Unit layak dipelihara (baik/rusak ringan, tidak idle)"</dt>
                                <dd class="font-semibold tabular-nums">{format_number(r.layak_pelihara)}</dd>
                            </div>
                            <div class="flex justify-between gap-3">
                                <dt>"Rusak ringan — perlu perbaikan agar tidak menjadi rusak berat"</dt>
                                <dd class="font-semibold tabular-nums">{format_number(r.rr)}</dd>
                            </div>
                            <div class="flex justify-between gap-3">
                                <dt>"Rusak berat — dikeluarkan dari pemeliharaan, diusulkan penghapusan"</dt>
                                <dd class="font-semibold tabular-nums">{format_number(r.rb)}</dd>
                            </div>
                        </dl>
                    </SectionCard>
                    <SectionCard title="Ketiga Skenario Berdampingan" subtitle=format!("total s.d. {akhir}")>
                        <Tabel
                            kepala=kepala(&["Skenario", ">Tunggakan (unit)", ">Total unit", ">Perkiraan biaya", ">Per tahun"])
                            baris=banding
                        />
                    </SectionCard>
                </div>
            }
        }
    };

    view! {
        <KepalaHalaman
            judul="Analisis Kebutuhan"
            uraian=format!("Berapa unit yang perlu diganti atau direhabilitasi sampai {akhir}, berapa perkiraan biayanya, dan apa yang dapat dipenuhi tanpa pengadaan.")
        />
        <PitaData meta=m/>

        <div class="no-print inline-flex gap-1 rounded-xl border border-ink-200 bg-ink-100 p-1 dark:border-ink-800 dark:bg-ink-950" role="group" aria-label="Pilih skenario">
            {tombol}
        </div>
        {badan}

        <div class="mt-4">
            <Catatan>
                <p>
                    <span class="font-semibold">"Asumsi. "</span>
                    {format!(
                        "Harga satuan peralatan memakai median perolehan lima tahun terakhir untuk kelompok barang yang sama; bila tidak tersedia, nilai perolehan aset dieskalasi {} per tahun. Rehabilitasi bangunan dihitung {} dari nilai aset. Biaya tahun-tahun mendatang dieskalasi {} per tahun. Asumsi dapat diubah pada halaman Pengaturan.",
                        format_persen(asumsi.eskalasi * 100.0),
                        format_persen(asumsi.faktor_rehab * 100.0),
                        format_persen(asumsi.eskalasi * 100.0),
                    )}
                </p>
                <p class="mt-1">
                    <span class="font-semibold">"Batasan. "</span>
                    "Analisis ini menghitung kebutuhan mengganti dan merehabilitasi aset yang sudah ada. Kebutuhan baru karena penambahan pegawai, satuan kerja baru, atau perubahan standar barang dan standar kebutuhan perlu ditambahkan tersendiri saat menyusun RKBMN."
                </p>
            </Catatan>
        </div>
    }
    .into_any()
}
