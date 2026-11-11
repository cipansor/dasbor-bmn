use leptos::prelude::*;
use leptos_router::hooks::use_query_map;

use crate::components::ui::{Badge, ErrorBlock, KepalaHalaman, LoadingBlock, SectionCard};
use crate::model::{format_number, AssetPage};
use crate::server_fns::{get_asset_page, get_tables, TableMeta};

const PER_PAGE: i64 = 25;

/// Kolom yang ditampilkan lebih dulu agar baris langsung terbaca.
const KOLOM_UTAMA: &[&str] = &[
    "nama",
    "kd_brg",
    "no_aset",
    "merk",
    "ur_sskel",
    "nama_satker",
    "ur_kondisi",
    "tgl_perlh",
    "rph_aset",
    "rph_buku",
    "umur_sisa",
    "ur_prov",
];

/// Kolom yang dapat diurutkan (kolom fakta pada penyimpanan lokal).
const KOLOM_URUT: &[&str] = &[
    "nama",
    "kd_brg",
    "merk",
    "ur_sskel",
    "nama_satker",
    "ur_kondisi",
    "tgl_perlh",
    "rph_aset",
    "rph_buku",
    "umur_sisa",
    "ur_prov",
];

#[derive(Clone, PartialEq)]
struct Saringan {
    table: String,
    page: i64,
    q: String,
    sort: Option<String>,
    dir: String,
    satker: String,
    kondisi: i64,
    status: String,
}

/// Halaman data: telusur, saring, urut, paginasi, dan ekspor CSV.
#[component]
pub fn DataPage() -> impl IntoView {
    let query = use_query_map();
    let satker_awal = query.with_untracked(|q| q.get("satker").unwrap_or_default());
    let tables = Resource::new(|| (), |_| async move { get_tables().await });
    let (saring, set_saring) = signal(Saringan {
        table: "SIMAN2_M_ASET_ANGKUTAN_BERMOTOR".to_string(),
        page: 1,
        q: String::new(),
        sort: None,
        dir: "asc".to_string(),
        satker: satker_awal,
        kondisi: -1,
        status: String::new(),
    });
    let page_res = Resource::new(
        move || saring.get(),
        |s| async move {
            get_asset_page(
                s.table, s.page, PER_PAGE, s.q, s.sort, s.dir, s.satker, s.kondisi, s.status,
            )
            .await
        },
    );

    view! {
        <KepalaHalaman
            judul="Data Aset"
            uraian="Telusuri baris aset per kategori dengan 106 kolom SIMAN; saring menurut kondisi, status, atau satuan kerja, lalu ekspor ke CSV."
        />
        <Suspense fallback=|| view! { <LoadingBlock rows=10/> }>
            {move || {
                tables
                    .get()
                    .map(|res| match res {
                        Ok(list) => view! { <Panel tables=list saring=saring set_saring=set_saring/> }.into_any(),
                        Err(e) => view! { <ErrorBlock message=format!("Gagal memuat daftar tabel: {e}")/> }.into_any(),
                    })
            }}
        </Suspense>
        <div class="mt-4">
            <Transition fallback=|| view! { <LoadingBlock rows=8/> }>
                {move || {
                    page_res
                        .get()
                        .map(|res| match res {
                            Ok(p) => view! { <DataTable data=p saring=saring set_saring=set_saring/> }.into_any(),
                            Err(e) => view! { <ErrorBlock message=format!("Gagal memuat data: {e}")/> }.into_any(),
                        })
                }}
            </Transition>
        </div>
    }
}

#[component]
fn Panel(
    tables: Vec<TableMeta>,
    saring: ReadSignal<Saringan>,
    set_saring: WriteSignal<Saringan>,
) -> impl IntoView {
    let ubah = move |f: &dyn Fn(&mut Saringan)| {
        set_saring.update(|s| {
            f(s);
            s.page = 1;
        })
    };
    view! {
        <SectionCard title="Saringan" subtitle="pencarian mencakup nama, kode barang, merk, kelompok, satuan kerja, dan lokasi">
            <div class="flex flex-wrap items-end gap-3">
                <label class="min-w-[12rem] flex-1">
                    <span class="mb-1 block text-xs font-medium text-ink-600 dark:text-ink-300">"Kategori BMN"</span>
                    <select class="input" on:change=move |ev| { let v = event_target_value(&ev); ubah(&|s| s.table = v.clone()); }>
                        {tables
                            .iter()
                            .map(|t| {
                                let nama = t.table.clone();
                                let nama_pilih = t.table.clone();
                                view! {
                                    <option value=nama selected=move || saring.with(|s| s.table == nama_pilih)>{t.label.clone()}</option>
                                }
                            })
                            .collect_view()}
                    </select>
                </label>
                <label class="min-w-[14rem] flex-[2]">
                    <span class="mb-1 block text-xs font-medium text-ink-600 dark:text-ink-300">"Pencarian"</span>
                    <input
                        type="search"
                        class="input"
                        placeholder="Nama barang, kode, merk, satuan kerja…"
                        prop:value=move || saring.with(|s| s.q.clone())
                        on:change=move |ev| { let v = event_target_value(&ev); ubah(&|s| s.q = v.clone()); }
                    />
                </label>
                <label class="min-w-[9rem]">
                    <span class="mb-1 block text-xs font-medium text-ink-600 dark:text-ink-300">"Kondisi"</span>
                    <select class="input" on:change=move |ev| { let v: i64 = event_target_value(&ev).parse().unwrap_or(-1); ubah(&|s| s.kondisi = v); }>
                        <option value="-1">"Semua"</option>
                        <option value="1">"Baik"</option>
                        <option value="2">"Rusak ringan"</option>
                        <option value="3">"Rusak berat"</option>
                        <option value="0">"Tidak diisi"</option>
                    </select>
                </label>
                <label class="min-w-[10rem]">
                    <span class="mb-1 block text-xs font-medium text-ink-600 dark:text-ink-300">"Status"</span>
                    <select class="input" on:change=move |ev| { let v = event_target_value(&ev); ubah(&|s| s.status = v.clone()); }>
                        <option value="">"Semua"</option>
                        <option value="habis">"Habis masa manfaat"</option>
                        <option value="idle">"Idle"</option>
                        <option value="hilang">"Hilang"</option>
                    </select>
                </label>
                <a
                    class="btn-primary"
                    rel="external"
                    href=move || saring.with(|s| format!("/api/ekspor.csv?tabel={}", s.table))
                >
                    "⬇️ Ekspor CSV kategori"
                </a>
            </div>
            {move || {
                let kode = saring.with(|s| s.satker.clone());
                (!kode.is_empty())
                    .then(|| {
                        let kode_tampil = kode.clone();
                        view! {
                            <p class="mt-3 flex flex-wrap items-center gap-2 text-xs">
                                <Badge text=format!("Satuan kerja: {kode_tampil}") tone="blue"/>
                                <a class="text-bmn-700 hover:underline dark:text-bmn-300" href=format!("/satker/{kode}")>"profil satuan kerja"</a>
                                <button type="button" class="text-ink-500 hover:underline" on:click=move |_| ubah(&|s| s.satker.clear())>
                                    "hapus saringan"
                                </button>
                            </p>
                        }
                    })
            }}
        </SectionCard>
    }
}

#[component]
fn DataTable(
    data: AssetPage,
    saring: ReadSignal<Saringan>,
    set_saring: WriteSignal<Saringan>,
) -> impl IntoView {
    let total_pages = ((data.total as f64) / (data.per_page as f64))
        .ceil()
        .max(1.0) as i64;
    let page = data.page;
    // Susun ulang kolom: yang paling sering dibaca di depan, sisanya mengikuti.
    let mut urutan: Vec<usize> = KOLOM_UTAMA
        .iter()
        .filter_map(|n| data.columns.iter().position(|c| c.name == *n))
        .collect();
    urutan.extend(
        (0..data.columns.len()).filter(|i| !KOLOM_UTAMA.contains(&data.columns[*i].name.as_str())),
    );
    let rupiah = |name: &str| name.starts_with("rph_");
    let angka = |name: &str| rupiah(name) || matches!(name, "umur_sisa" | "luas" | "no_aset");
    let kolom: Vec<(usize, String, String)> = urutan
        .iter()
        .map(|i| {
            (
                *i,
                data.columns[*i].name.clone(),
                data.columns[*i].label.clone(),
            )
        })
        .collect();
    let n_kolom = kolom.len();
    let rows = data.rows.clone();
    let per_page = data.per_page;

    view! {
        <section class="card overflow-hidden">
            <header class="flex flex-wrap items-center justify-between gap-2 border-b border-ink-100 px-4 py-3 dark:border-ink-800">
                <div class="flex items-center gap-2">
                    <h2 class="text-sm font-semibold">{data.label.clone()}</h2>
                    {if data.is_demo {
                        view! { <Badge text="Data contoh" tone="amber"/> }.into_any()
                    } else {
                        view! { <Badge text="Data SLDK" tone="green"/> }.into_any()
                    }}
                </div>
                <p class="text-xs text-ink-500 dark:text-ink-400">
                    {format!("{} baris cocok · halaman {} dari {}", format_number(data.total), format_number(page), format_number(total_pages))}
                </p>
            </header>
            <div class="table-wrap max-h-[70vh] overflow-y-auto">
                <table class="w-full min-w-[60rem] text-left text-xs">
                    <thead class="sticky top-0 z-10 bg-ink-50 text-ink-600 dark:bg-ink-900 dark:text-ink-300">
                        <tr>
                            <th scope="col" class="px-3 py-2 font-semibold">"#"</th>
                            {kolom
                                .iter()
                                .map(|(_, name, label)| {
                                    let dapat = KOLOM_URUT.contains(&name.as_str());
                                    let nama = name.clone();
                                    let nama_tanda = name.clone();
                                    let label = label.clone();
                                    view! {
                                        <th scope="col" class="whitespace-nowrap px-3 py-2 font-semibold">
                                            {if dapat {
                                                view! {
                                                    <button
                                                        type="button"
                                                        class="inline-flex items-center gap-1 hover:text-bmn-600"
                                                        title="Urutkan"
                                                        on:click=move |_| {
                                                            set_saring.update(|s| {
                                                                if s.sort.as_deref() == Some(nama.as_str()) {
                                                                    s.dir = if s.dir == "asc" { "desc".into() } else { "asc".into() };
                                                                } else {
                                                                    s.sort = Some(nama.clone());
                                                                    s.dir = "asc".into();
                                                                }
                                                                s.page = 1;
                                                            })
                                                        }
                                                    >
                                                        {label}
                                                        <span aria-hidden="true">
                                                            {move || saring.with(|s| {
                                                                if s.sort.as_deref() == Some(nama_tanda.as_str()) {
                                                                    if s.dir == "asc" { "▲" } else { "▼" }
                                                                } else {
                                                                    "↕"
                                                                }
                                                            })}
                                                        </span>
                                                    </button>
                                                }
                                                    .into_any()
                                            } else {
                                                view! { <span>{label}</span> }.into_any()
                                            }}
                                        </th>
                                    }
                                })
                                .collect_view()}
                        </tr>
                    </thead>
                    <tbody>
                        {if rows.is_empty() {
                            view! {
                                <tr>
                                    <td colspan=n_kolom + 1 class="px-3 py-10 text-center text-ink-500">
                                        "Tidak ada baris yang cocok dengan saringan."
                                    </td>
                                </tr>
                            }
                                .into_any()
                        } else {
                            rows
                                .iter()
                                .enumerate()
                                .map(|(i, r)| {
                                    let nomor = ((page - 1) * per_page) as usize + i + 1;
                                    view! {
                                        <tr class="border-t border-ink-100 hover:bg-bmn-50/50 dark:border-ink-800 dark:hover:bg-bmn-900/20">
                                            <td class="px-3 py-1.5 tabular-nums text-ink-400">{nomor}</td>
                                            {kolom
                                                .iter()
                                                .map(|(ci, name, _)| {
                                                    let v = r.values.get(*ci).cloned().unwrap_or_default();
                                                    let teks = if v.is_empty() { "—".to_string() } else { v.clone() };
                                                    let tampil = if !v.is_empty() && rupiah(name) {
                                                        crate::model::format_rupiah(crate::model::parse_angka(&v).unwrap_or(0.0))
                                                    } else {
                                                        teks.clone()
                                                    };
                                                    view! {
                                                        <td
                                                            class=format!("max-w-[16rem] truncate px-3 py-1.5 {}", if angka(name) { "tabular-nums text-right" } else { "" })
                                                            title=teks
                                                        >
                                                            {tampil}
                                                        </td>
                                                    }
                                                })
                                                .collect_view()}
                                        </tr>
                                    }
                                })
                                .collect_view()
                                .into_any()
                        }}
                    </tbody>
                </table>
            </div>
            <footer class="flex flex-wrap items-center justify-between gap-2 border-t border-ink-100 px-4 py-3 dark:border-ink-800">
                <p class="text-xs text-ink-500 dark:text-ink-400">{format!("{n_kolom} kolom per baris · gulir ke kanan untuk kolom lain")}</p>
                <div class="flex items-center gap-2">
                    <button type="button" class="btn-ghost" disabled={page <= 1} on:click=move |_| set_saring.update(|s| s.page = (page - 1).max(1))>
                        "← Sebelumnya"
                    </button>
                    <button type="button" class="btn-ghost" disabled={page >= total_pages} on:click=move |_| set_saring.update(|s| s.page = page + 1)>
                        "Berikutnya →"
                    </button>
                </div>
            </footer>
        </section>
    }
}
