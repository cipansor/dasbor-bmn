use leptos::prelude::*;

use crate::components::chart::BilahKondisi;
use crate::components::ui::{ErrorBlock, KepalaHalaman, LoadingBlock, PitaData, SectionCard};
use crate::dto::Analisis;
use crate::model::{format_number, format_persen, format_rupiah_short};
use crate::schema::COLUMN_COUNT;
use crate::server_fns::{get_analisis, get_tables, TableMeta};

/// Halaman kategori: ringkasan 15 kategori BMN dan struktur datanya.
#[component]
pub fn CategoryPage() -> impl IntoView {
    let analisis = Resource::new(|| (), |_| async move { get_analisis().await });
    let tables = Resource::new(|| (), |_| async move { get_tables().await });
    view! {
        <Suspense fallback=|| view! { <LoadingBlock rows=6/> }>
            {move || match (analisis.get(), tables.get()) {
                (Some(Ok(a)), Some(Ok(t))) => view! { <Isi a=a tables=t/> }.into_any(),
                (Some(Err(e)), _) | (_, Some(Err(e))) => view! { <ErrorBlock message=format!("Gagal memuat kategori: {e}")/> }.into_any(),
                _ => view! { <LoadingBlock rows=6/> }.into_any(),
            }}
        </Suspense>
    }
}

#[component]
fn Isi(a: Analisis, tables: Vec<TableMeta>) -> impl IntoView {
    let (pilih, set_pilih) = signal(tables.first().map(|t| t.table.clone()).unwrap_or_default());
    let daftar = StoredValue::new(tables.clone());
    let total_nilai = a.nasional.nilai;

    view! {
        <KepalaHalaman
            judul="Kategori Aset"
            uraian=format!("Lima belas kategori Master Aset BMN pada SIMAN v2, masing-masing {COLUMN_COUNT} kolom.")
        />
        <PitaData meta=a.meta.clone()/>

        <div class="grid gap-3 sm:grid-cols-2 xl:grid-cols-3">
            {tables
                .iter()
                .map(|t| {
                    let k = a.kategori.iter().find(|c| c.tabel == t.table).cloned();
                    let tabel = t.table.clone();
                    let tabel_aktif = t.table.clone();
                    let porsi = k.as_ref().map(|c| 100.0 * c.r.nilai / total_nilai.max(1.0)).unwrap_or(0.0);
                    view! {
                        <button
                            type="button"
                            class=move || format!("card p-4 text-left transition-shadow hover:shadow-md {}", if pilih.get() == tabel_aktif { "ring-2 ring-bmn-500" } else { "" })
                            on:click=move |_| set_pilih.set(tabel.clone())
                        >
                            <div class="flex items-start justify-between gap-2">
                                <span class="text-2xl" aria-hidden="true">{t.icon.clone()}</span>
                                <span class="chip bg-ink-100 text-ink-600 dark:bg-ink-800 dark:text-ink-300">{format!("sumber {}", t.volume)}</span>
                            </div>
                            <p class="mt-2 truncate text-sm font-semibold">{t.label.clone()}</p>
                            {match k {
                                Some(c) => view! {
                                    <p class="mt-1 text-lg font-bold tabular-nums">
                                        {format_number(c.r.unit)}
                                        <span class="ml-1 text-xs font-normal text-ink-500">"unit"</span>
                                    </p>
                                    <p class="text-xs text-ink-500 dark:text-ink-400">
                                        {format!("{} · {} dari nilai total", format_rupiah_short(c.r.nilai), format_persen(porsi))}
                                    </p>
                                    {c.fisik.then(|| view! {
                                        <div class="mt-2"><BilahKondisi baik=c.r.baik rr=c.r.rr rb=c.r.rb tak=c.r.tak tipis=true/></div>
                                        <p class="mt-1 text-[0.7rem] text-ink-500 dark:text-ink-400">
                                            {format!("{} baik · {} rusak berat", format_persen(c.r.pct_baik()), format_number(c.r.rb))}
                                        </p>
                                    })}
                                }.into_any(),
                                None => view! { <p class="mt-1 text-xs text-ink-500">"Belum tersinkron"</p> }.into_any(),
                            }}
                        </button>
                    }
                })
                .collect_view()}
        </div>

        <div class="mt-4">
            <SectionCard title="Struktur Data" subtitle="definisi kolom sesuai panduan Web Service SLDK">
                {move || {
                    let nama = pilih.get();
                    daftar
                        .with_value(|d| d.iter().find(|t| t.table == nama).cloned())
                        .map(|t| {
                            view! {
                                <div class="mb-2 flex flex-wrap items-baseline gap-x-3 gap-y-1 text-xs text-ink-500 dark:text-ink-400">
                                    <span class="font-semibold text-ink-700 dark:text-ink-200">{t.nama_data.clone()}</span>
                                    <code class="rounded bg-ink-100 px-1.5 py-0.5 dark:bg-ink-800">{t.table.clone()}</code>
                                    <span>{format!("/{}", t.resource)}</span>
                                    <span>{format!("{} kolom", t.columns.len())}</span>
                                </div>
                                <div class="table-wrap max-h-[28rem] overflow-y-auto rounded-lg border border-ink-200 dark:border-ink-800">
                                    <table class="w-full min-w-[40rem] text-left text-xs">
                                        <thead class="sticky top-0 bg-ink-50 text-ink-600 dark:bg-ink-900 dark:text-ink-300">
                                            <tr>
                                                <th scope="col" class="px-3 py-2 font-semibold">"No."</th>
                                                <th scope="col" class="px-3 py-2 font-semibold">"Kolom"</th>
                                                <th scope="col" class="px-3 py-2 font-semibold">"Uraian"</th>
                                                <th scope="col" class="px-3 py-2 font-semibold">"Tipe"</th>
                                                <th scope="col" class="px-3 py-2 font-semibold">"Panjang"</th>
                                            </tr>
                                        </thead>
                                        <tbody>
                                            {t.columns
                                                .iter()
                                                .enumerate()
                                                .map(|(i, c)| {
                                                    view! {
                                                        <tr class="border-t border-ink-100 dark:border-ink-800">
                                                            <td class="px-3 py-1.5 tabular-nums text-ink-400">{i + 1}</td>
                                                            <td class="px-3 py-1.5 font-medium">{c.name.clone()}</td>
                                                            <td class="px-3 py-1.5 text-ink-600 dark:text-ink-300">{c.label.clone()}</td>
                                                            <td class="px-3 py-1.5">{c.r#type.clone()}</td>
                                                            <td class="px-3 py-1.5 tabular-nums text-ink-400">{c.len.clone()}</td>
                                                        </tr>
                                                    }
                                                })
                                                .collect_view()}
                                        </tbody>
                                    </table>
                                </div>
                            }
                        })
                }}
            </SectionCard>
        </div>
    }
}
