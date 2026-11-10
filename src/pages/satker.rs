use leptos::prelude::*;
use leptos_router::hooks::use_params_map;

use crate::components::chart::{BilahKondisi, KolomTumpuk, Seri, W_RB, W_UNGU};
use crate::components::ui::{
    kepala, Catatan, ErrorBlock, KartuRekomendasi, KepalaHalaman, Kpi, LencanaSkor, LoadingBlock,
    PitaData, SectionCard, Sel, Tabel, TabelAset,
};
use crate::dto::{SatkerBaris, SatkerRinci, BOBOT_SKOR, HORIZON, NH, SKENARIO};
use crate::model::{
    format_desimal, format_number, format_persen, format_rupiah_short, format_unit,
};
use crate::server_fns::{get_meta, get_satker, get_satker_rinci};

/// Daftar seluruh satuan kerja: cari, saring provinsi, dan urutkan menurut ukuran pilihan.
#[component]
pub fn SatkerPage() -> impl IntoView {
    let data = Resource::new(|| (), |_| async move { get_satker().await });
    let meta = Resource::new(|| (), |_| async move { get_meta().await });
    view! {
        <KepalaHalaman
            judul="Satuan Kerja"
            uraian="Peringkat dan rincian tiap satuan kerja: kondisi, masa manfaat, prediksi rusak berat, dan kebutuhan lima tahun. Klik nama untuk membuka profil lengkap."
        >
            <a class="btn-ghost" href="/api/satker.csv">"⬇️ Unduh CSV"</a>
        </KepalaHalaman>
        <Suspense fallback=|| view! { <LoadingBlock rows=10/> }>
            {move || meta.get().and_then(|m| m.ok()).map(|m| view! { <PitaData meta=m/> })}
            {move || {
                data.get()
                    .map(|r| match r {
                        Ok(v) => view! { <Daftar data=v/> }.into_any(),
                        Err(e) => view! { <ErrorBlock message=format!("Gagal memuat satuan kerja: {e}")/> }.into_any(),
                    })
            }}
        </Suspense>
    }
}

const URUTAN: &[(&str, &str)] = &[
    ("skor_naik", "Skor terendah dulu"),
    ("skor_turun", "Skor tertinggi dulu"),
    ("rb", "Rusak berat terbanyak"),
    ("rb5", "Prediksi rusak berat 5 th terbesar"),
    ("habis", "Habis masa manfaat terbanyak"),
    ("biaya", "Kebutuhan biaya terbesar"),
    ("rku", "RKU tertinggi"),
    ("nilai", "Nilai aset terbesar"),
    ("nama", "Nama A–Z"),
];

#[component]
fn Daftar(data: Vec<SatkerBaris>) -> impl IntoView {
    let mut provinsi: Vec<String> = data.iter().map(|s| s.prov.clone()).collect();
    provinsi.sort();
    provinsi.dedup();
    let jumlah = data.len();
    let semua = StoredValue::new(data);
    let (q, set_q) = signal(String::new());
    let (prov, set_prov) = signal(String::new());
    let (urut, set_urut) = signal("skor_naik".to_string());

    let tersaring = Memo::new(move |_| {
        let cari = q.get().trim().to_lowercase();
        let p = prov.get();
        let u = urut.get();
        let mut v: Vec<SatkerBaris> = semua.with_value(|d| {
            d.iter()
                .filter(|s| p.is_empty() || s.prov == p)
                .filter(|s| {
                    cari.is_empty()
                        || s.nama.to_lowercase().contains(&cari)
                        || s.kode.to_lowercase().contains(&cari)
                })
                .cloned()
                .collect()
        });
        v.sort_by(|a, b| match u.as_str() {
            "skor_turun" => b.skor.total_cmp(&a.skor),
            "rb" => b.rb.cmp(&a.rb),
            "rb5" => b.rb5.total_cmp(&a.rb5),
            "habis" => b.habis.cmp(&a.habis),
            "biaya" => b.biaya5.total_cmp(&a.biaya5),
            "rku" => b.rku.total_cmp(&a.rku),
            "nilai" => b.nilai.total_cmp(&a.nilai),
            "nama" => a.nama.cmp(&b.nama),
            _ => a.skor.total_cmp(&b.skor),
        });
        v
    });

    view! {
        <SectionCard title="Daftar Satuan Kerja" subtitle=format!("{} satuan kerja", format_number(jumlah as i64))>
            <div class="no-print mb-3 flex flex-wrap items-end gap-3">
                <label class="min-w-[14rem] flex-[2]">
                    <span class="mb-1 block text-xs font-medium text-ink-600 dark:text-ink-300">"Cari satuan kerja"</span>
                    <input
                        type="search"
                        class="input"
                        placeholder="Nama atau kode satuan kerja…"
                        prop:value=move || q.get()
                        on:input=move |ev| set_q.set(event_target_value(&ev))
                    />
                </label>
                <label class="min-w-[12rem] flex-1">
                    <span class="mb-1 block text-xs font-medium text-ink-600 dark:text-ink-300">"Provinsi"</span>
                    <select class="input" on:change=move |ev| set_prov.set(event_target_value(&ev))>
                        <option value="">"Semua provinsi"</option>
                        {provinsi.into_iter().map(|p| view! { <option value=p.clone()>{p.clone()}</option> }).collect_view()}
                    </select>
                </label>
                <label class="min-w-[14rem] flex-1">
                    <span class="mb-1 block text-xs font-medium text-ink-600 dark:text-ink-300">"Urutkan"</span>
                    <select class="input" on:change=move |ev| set_urut.set(event_target_value(&ev))>
                        {URUTAN.iter().map(|(k, l)| view! { <option value=*k>{*l}</option> }).collect_view()}
                    </select>
                </label>
            </div>
            <p class="mb-2 text-xs text-ink-500 dark:text-ink-400">
                {move || format!("Menampilkan {} satuan kerja.", format_number(tersaring.get().len() as i64))}
            </p>
            <div class="table-wrap max-h-[75vh] overflow-y-auto rounded-lg border border-ink-200 dark:border-ink-800">
                <table class="w-full min-w-[68rem] text-left text-xs">
                    <thead class="sticky top-0 z-10 bg-ink-50 text-ink-600 dark:bg-ink-900 dark:text-ink-300">
                        <tr>
                            <th scope="col" class="px-3 py-2 font-semibold">"#"</th>
                            <th scope="col" class="px-3 py-2 font-semibold">"Satuan kerja"</th>
                            <th scope="col" class="px-3 py-2 text-right font-semibold">"Unit"</th>
                            <th scope="col" class="px-3 py-2 text-right font-semibold">"Nilai"</th>
                            <th scope="col" class="px-3 py-2 text-right font-semibold">"Baik"</th>
                            <th scope="col" class="px-3 py-2 text-right font-semibold">"Rusak berat"</th>
                            <th scope="col" class="px-3 py-2 text-right font-semibold">"Habis MM"</th>
                            <th scope="col" class="px-3 py-2 text-right font-semibold">"Idle"</th>
                            <th scope="col" class="px-3 py-2 text-right font-semibold">"Hilang"</th>
                            <th scope="col" class="px-3 py-2 text-right font-semibold" title="Ekspektasi rusak berat baru dalam 5 tahun">"RB baru 5 th"</th>
                            <th scope="col" class="px-3 py-2 text-right font-semibold" title="Kebutuhan skenario moderat termasuk tunggakan">"Kebutuhan 5 th"</th>
                            <th scope="col" class="px-3 py-2 text-right font-semibold" title="Rasio Kerusakan terstandar Umur">"RKU"</th>
                            <th scope="col" class="px-3 py-2 font-semibold">"Skor"</th>
                        </tr>
                    </thead>
                    <tbody>
                        {move || {
                            tersaring
                                .get()
                                .into_iter()
                                .enumerate()
                                .map(|(i, s)| {
                                    view! {
                                        <tr class="border-t border-ink-100 hover:bg-bmn-50/50 dark:border-ink-800 dark:hover:bg-bmn-900/20">
                                            <td class="px-3 py-2 tabular-nums text-ink-400">{i + 1}</td>
                                            <td class="px-3 py-2">
                                                <a class="font-medium text-bmn-700 hover:underline dark:text-bmn-300" href=format!("/satker/{}", s.kode)>{s.nama.clone()}</a>
                                                <span class="block text-[0.7rem] text-ink-500 dark:text-ink-400">{format!("{} · {}", s.prov, s.kode)}</span>
                                            </td>
                                            <td class="px-3 py-2 text-right tabular-nums">{format_number(s.unit)}</td>
                                            <td class="px-3 py-2 text-right tabular-nums">{format_rupiah_short(s.nilai)}</td>
                                            <td class="px-3 py-2 text-right tabular-nums">{format_persen(s.pct_baik)}</td>
                                            <td class="px-3 py-2 text-right tabular-nums">{format_number(s.rb)}</td>
                                            <td class="px-3 py-2 text-right tabular-nums">{format!("{} ({})", format_number(s.habis), format_persen(s.pct_habis))}</td>
                                            <td class="px-3 py-2 text-right tabular-nums">{format_number(s.idle)}</td>
                                            <td class="px-3 py-2 text-right tabular-nums">{format_number(s.hilang)}</td>
                                            <td class="px-3 py-2 text-right tabular-nums">{format_unit(s.rb5)}</td>
                                            <td class="px-3 py-2 text-right tabular-nums">{format!("{} unit · {}", format_unit(s.ganti5), format_rupiah_short(s.biaya5))}</td>
                                            <td class="px-3 py-2 text-right tabular-nums">{format!("{}×", format_desimal(s.rku, 2))}</td>
                                            <td class="px-3 py-2"><LencanaSkor skor=s.skor/></td>
                                        </tr>
                                    }
                                })
                                .collect_view()
                        }}
                    </tbody>
                </table>
            </div>
            <p class="mt-2 text-xs text-ink-500 dark:text-ink-400">
                "MM = masa manfaat. RB baru = ekspektasi model kerusakan. Kebutuhan = skenario moderat termasuk tunggakan. RKU di atas 1 berarti kerusakan melebihi kewajaran umur aset."
            </p>
        </SectionCard>
    }
}

/// Profil lengkap satu satuan kerja.
#[component]
pub fn SatkerRinciPage() -> impl IntoView {
    let params = use_params_map();
    let kode = move || params.with(|p| p.get("kode").unwrap_or_default());
    let data = Resource::new(kode, |k| async move { get_satker_rinci(k).await });
    view! {
        <Suspense fallback=|| view! { <LoadingBlock rows=10/> }>
            {move || {
                data.get()
                    .map(|r| match r {
                        Ok(s) => view! { <Rinci s=s/> }.into_any(),
                        Err(e) => view! {
                            <ErrorBlock message=format!("{e}")/>
                            <a class="btn-ghost mt-3" href="/satker">"← Kembali ke daftar satuan kerja"</a>
                        }.into_any(),
                    })
            }}
        </Suspense>
    }
}

#[component]
fn Rinci(s: SatkerRinci) -> impl IntoView {
    let r = s.r;
    let m = s.meta.clone();
    let akhir = m.tahun_akhir();
    let label_th: Vec<String> = (0..NH).map(|k| m.label_tahun(k)).collect();
    let banding = |nilai: f64, nasional: f64, lebih_baik_tinggi: bool| -> String {
        let selisih = nilai - nasional;
        let baik = (selisih >= 0.0) == lebih_baik_tinggi;
        format!(
            "nasional {} · {} {} poin",
            format_persen(nasional),
            if baik { "lebih baik" } else { "lebih buruk" },
            format_desimal(selisih.abs(), 1)
        )
    };
    let komponen = [
        s.skor.kondisi,
        s.skor.umur,
        s.skor.pemanfaatan,
        s.skor.pengamanan,
        s.skor.tertib,
    ];
    let kategori: Vec<Vec<Sel>> = s
        .kategori
        .iter()
        .map(|c| {
            let k = &c.r;
            vec![
                Sel::Teks(format!("{} {}", c.icon, c.label)),
                Sel::unit(k.unit),
                Sel::rupiah(k.nilai),
                Sel::persen_bila(k.pct_baik(), c.fisik && k.kondisi_diketahui() > 0),
                Sel::Angka(if c.fisik { format_number(k.rr) } else { "–".into() }),
                Sel::Angka(if c.fisik { format_number(k.rb) } else { "–".into() }),
                Sel::Angka(if c.disusutkan { format_number(k.habis) } else { "–".into() }),
                Sel::Angka(if c.disusutkan { format_unit(k.habis_5()) } else { "–".into() }),
                Sel::Angka(if c.fisik { format_unit(k.rb_5()) } else { "–".into() }),
                Sel::Angka(if c.kebutuhan != "-" { format_unit(k.ganti_total(1)) } else { "–".into() }),
                Sel::Angka(if c.kebutuhan != "-" { format_rupiah_short(k.biaya_total(1)) } else { "–".into() }),
            ]
        })
        .collect();
    let mut kep_sk: Vec<String> = vec!["Skenario".into(), ">Tunggakan".into()];
    kep_sk.extend(label_th.iter().map(|l| format!(">{l}")));
    kep_sk.push(">Total unit".into());
    kep_sk.push(">Perkiraan biaya".into());
    let kep_sk = kepala(&kep_sk.iter().map(|x| x.as_str()).collect::<Vec<_>>());
    let label_rb = label_th.clone();
    let skenario: Vec<Vec<Sel>> = (0..3)
        .map(|i| {
            let mut sel = vec![Sel::Teks(SKENARIO[i].0.to_string()), Sel::unit_f(r.ganti_kini[i])];
            sel.extend(r.ganti_th[i].iter().map(|v| Sel::unit_f(*v)));
            sel.push(Sel::unit_f(r.ganti_total(i)));
            sel.push(Sel::rupiah(r.biaya_total(i)));
            sel
        })
        .collect();
    let ada_rekomendasi = !s.rekomendasi.is_empty();

    view! {
        <a class="no-print mb-2 inline-block text-xs text-bmn-700 hover:underline dark:text-bmn-300" href="/satker">"← Seluruh satuan kerja"</a>
        <KepalaHalaman
            judul=s.nama.clone()
            uraian=format!("{} · kode {} · peringkat {} dari {} satuan kerja", s.prov, s.kode, s.peringkat, s.dari)
        >
            <a class="btn-ghost" href=format!("/data?satker={}", s.kode)>"🔎 Lihat aset"</a>
            <a class="btn-primary" href=format!("/api/laporan.pdf?satker={}", s.kode)>"🧾 Laporan PDF satuan kerja"</a>
        </KepalaHalaman>
        <PitaData meta=m.clone()/>

        <div class="grid grid-cols-2 gap-3 lg:grid-cols-4">
            <Kpi label="Total BMN" value=format!("{} unit", format_number(r.unit)) hint=format!("nilai {} · buku {}", format_rupiah_short(r.nilai), format_rupiah_short(r.buku)) icon="📦"/>
            <Kpi label="Kondisi Baik" value=format_persen(r.pct_baik()) hint=banding(r.pct_baik(), s.nasional_pct_baik, true) icon="🩺" tone={if r.pct_baik() >= s.nasional_pct_baik { "emerald" } else { "amber" }}/>
            <Kpi label="Rusak Berat" value=format!("{} unit", format_number(r.rb)) hint=format!("senilai {} · rusak ringan {}", format_rupiah_short(r.nilai_rb), format_number(r.rr)) icon="🛠️" tone="rose"/>
            <Kpi label="Habis Masa Manfaat" value=format!("{} unit", format_number(r.habis)) hint=banding(r.pct_habis(), s.nasional_pct_habis, false) icon="⏳" tone="violet"/>
        </div>
        <div class="mt-3 grid grid-cols-2 gap-3 lg:grid-cols-4">
            <Kpi label=format!("Rusak Berat Baru s.d. {akhir}") value=format!("±{} unit", format_unit(r.rb_5())) hint="ekspektasi model kerusakan" icon="🔮" tone="rose"/>
            <Kpi label=format!("Habis MM s.d. {akhir}") value=format!("+{} unit", format_unit(r.habis_5())) hint="tambahan dari posisi hari ini" icon="📅" tone="violet"/>
            <Kpi label="Kebutuhan Moderat 5 Tahun" value=format!("{} unit", format_unit(r.ganti_total(1))) hint=format!("perkiraan {} · {}/tahun", format_rupiah_short(r.biaya_total(1)), format_rupiah_short(r.biaya_total(1) / HORIZON as f64)) icon="🧮"/>
            <Kpi label="Idle / Hilang" value=format!("{} / {}", format_number(r.idle), format_number(r.hilang)) hint=format!("PSP {} · data lengkap {}", format_persen(r.pct_psp()), format_persen(r.pct_lengkap())) icon="⏸️" tone="amber"/>
        </div>

        <div class="mt-4 grid gap-4 lg:grid-cols-2">
            <SectionCard title="Skor Kesehatan BMN" subtitle=format!("nasional {}", format_desimal(s.skor_nasional, 1))>
                <div class="mb-3 flex items-center gap-3">
                    <p class="text-3xl font-bold tabular-nums">{format_desimal(s.skor.total, 1)}</p>
                    <LencanaSkor skor=s.skor.total/>
                    <span class="text-xs text-ink-500 dark:text-ink-400">{format!("RKU {}×", format_desimal(r.rku(), 2))}</span>
                </div>
                <ul class="space-y-2">
                    {BOBOT_SKOR
                        .iter()
                        .zip(komponen)
                        .map(|((nama, bobot, uraian), nilai)| {
                            let warna = if nilai >= 80.0 { "#10b981" } else if nilai >= 65.0 { "#f59e0b" } else { "#ef4444" };
                            view! {
                                <li title=*uraian>
                                    <div class="flex justify-between text-xs">
                                        <span>{format!("{nama} (bobot {:.0}%)", bobot * 100.0)}</span>
                                        <span class="tabular-nums font-medium">{format_desimal(nilai, 1)}</span>
                                    </div>
                                    <div class="mt-1 h-2 overflow-hidden rounded-full bg-ink-100 dark:bg-ink-800">
                                        <div class="h-full rounded-full" style=format!("width:{:.1}%;background:{warna}", nilai.clamp(0.0, 100.0))></div>
                                    </div>
                                </li>
                            }
                        })
                        .collect_view()}
                </ul>
            </SectionCard>
            <SectionCard title="Sebaran Kondisi" subtitle="aset fisik">
                <BilahKondisi baik=r.baik rr=r.rr rb=r.rb tak=r.tak/>
                <dl class="mt-3 grid grid-cols-2 gap-x-4 gap-y-1 text-sm">
                    <dt>"Baik"</dt><dd class="text-right tabular-nums">{format_number(r.baik)}</dd>
                    <dt>"Rusak ringan"</dt><dd class="text-right tabular-nums">{format_number(r.rr)}</dd>
                    <dt>"Rusak berat"</dt><dd class="text-right tabular-nums">{format_number(r.rb)}</dd>
                    <dt>"Kondisi tidak diisi"</dt><dd class="text-right tabular-nums">{format_number(r.tak)}</dd>
                    <dt>"Umur rata-rata"</dt><dd class="text-right tabular-nums">{format!("{} th", format_desimal(r.umur_rata(), 1))}</dd>
                    <dt>"Layak dipelihara"</dt><dd class="text-right tabular-nums">{format_number(r.layak_pelihara)}</dd>
                    <dt>"Tanah berbukti kepemilikan"</dt>
                    <dd class="text-right tabular-nums">{if r.tanah_n > 0 { format!("{} dari {}", r.sertif_ya, r.tanah_n) } else { "–".into() }}</dd>
                </dl>
            </SectionCard>
        </div>

        <div class="mt-4 grid gap-4 lg:grid-cols-2">
            <SectionCard title="Habis Masa Manfaat per Tahun" subtitle="unit">
                <KolomTumpuk label=label_th seri=vec![Seri { nama: "Habis masa manfaat".into(), warna: W_UNGU, nilai: r.habis_th.to_vec() }] satuan="unit"/>
            </SectionCard>
            <SectionCard title="Perkiraan Rusak Berat Baru per Tahun" subtitle="unit · ekspektasi">
                <KolomTumpuk label=label_rb seri=vec![Seri { nama: "Rusak berat baru".into(), warna: W_RB, nilai: r.rb_th.to_vec() }] satuan="unit"/>
            </SectionCard>
        </div>

        <div class="mt-4">
            <SectionCard title="Kebutuhan Lima Tahun per Skenario" subtitle="unit per tahun anggaran">
                <Tabel kepala=kep_sk baris=skenario/>
            </SectionCard>
        </div>

        <div class="mt-4">
            <SectionCard title="Rincian per Kategori" subtitle="kebutuhan = skenario moderat">
                <Tabel
                    kepala=kepala(&["Kategori", ">Unit", ">Nilai", ">Baik", ">Rusak ringan", ">RB", ">Habis MM", ">Habis MM 5 th", ">RB baru 5 th", ">Kebutuhan 5 th", ">Perkiraan biaya"])
                    baris=kategori
                />
            </SectionCard>
        </div>

        {ada_rekomendasi
            .then(|| {
                view! {
                    <div class="mt-4">
                        <SectionCard title="Rekomendasi untuk Satuan Kerja Ini" subtitle="urut prioritas">
                            <div class="grid gap-3 lg:grid-cols-2">
                                {s.rekomendasi.iter().cloned().map(|x| view! { <KartuRekomendasi r=x/> }).collect_view()}
                            </div>
                        </SectionCard>
                    </div>
                }
            })}

        <div class="mt-4">
            <SectionCard title="Aset Rusak Berat" subtitle="bernilai terbesar · calon usulan penghapusan">
                <TabelAset aset=s.aset_rb.clone() dengan_satker=false dengan_peluang=false/>
            </SectionCard>
        </div>
        <div class="mt-4">
            <SectionCard title="Aset Paling Berisiko Menjadi Rusak Berat" subtitle="peluang dalam 5 tahun">
                <TabelAset aset=s.aset_risiko.clone() dengan_satker=false/>
            </SectionCard>
        </div>
        <div class="mt-4">
            <SectionCard title="Aset Habis Masa Manfaat yang Masih Tercatat" subtitle="bernilai terbesar · bahan rencana peremajaan">
                <TabelAset aset=s.aset_habis.clone() dengan_satker=false/>
            </SectionCard>
        </div>

        <div class="mt-4">
            <Catatan>
                "Daftar aset memuat 15 teratas tiap jenis. Seluruh aset satuan kerja ini dapat ditelusuri dan diekspor lewat tombol Lihat aset."
            </Catatan>
        </div>
    }
}
