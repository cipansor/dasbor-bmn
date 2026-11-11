use leptos::prelude::*;

use crate::dto::{Analisis, AsetRisiko, Meta, Rekomendasi, Sorotan};
use crate::model::{format_desimal, format_number, format_persen, format_rupiah_short};
use crate::server_fns::get_analisis;

fn kelas_nada(tone: &str) -> &'static str {
    match tone {
        "emerald" | "green" => {
            "bg-emerald-50 text-emerald-700 dark:bg-emerald-500/10 dark:text-emerald-300"
        }
        "amber" => "bg-amber-50 text-amber-700 dark:bg-amber-500/10 dark:text-amber-300",
        "rose" | "red" => "bg-rose-50 text-rose-700 dark:bg-rose-500/10 dark:text-rose-300",
        "violet" => "bg-violet-50 text-violet-700 dark:bg-violet-500/10 dark:text-violet-300",
        "slate" => "bg-ink-100 text-ink-600 dark:bg-ink-800 dark:text-ink-300",
        _ => "bg-bmn-50 text-bmn-700 dark:bg-bmn-500/10 dark:text-bmn-300",
    }
}

/// Kartu KPI: label, nilai utama, dan keterangan pendukung.
#[component]
pub fn Kpi(
    #[prop(into)] label: String,
    #[prop(into)] value: String,
    #[prop(into, optional)] hint: Option<String>,
    #[prop(default = "📊")] icon: &'static str,
    #[prop(default = "blue")] tone: &'static str,
) -> impl IntoView {
    view! {
        <div class="card p-4">
            <div class="flex items-start justify-between gap-3">
                <p class="text-xs font-medium uppercase tracking-wide text-ink-500 dark:text-ink-400">{label}</p>
                <span class=format!("grid size-9 shrink-0 place-items-center rounded-lg text-base {}", kelas_nada(tone)) aria-hidden="true">
                    {icon}
                </span>
            </div>
            <p class="mt-2 text-2xl font-bold tabular-nums leading-tight sm:text-[1.7rem]">{value}</p>
            {hint.map(|h| view! { <p class="mt-1 text-xs text-ink-500 dark:text-ink-400">{h}</p> })}
        </div>
    }
}

/// Label kecil berwarna.
#[component]
pub fn Badge(#[prop(into)] text: String, tone: &'static str) -> impl IntoView {
    let cls = match tone {
        "green" => "bg-emerald-100 text-emerald-800 dark:bg-emerald-500/15 dark:text-emerald-300",
        "amber" => "bg-amber-100 text-amber-800 dark:bg-amber-500/15 dark:text-amber-300",
        "red" => "bg-rose-100 text-rose-800 dark:bg-rose-500/15 dark:text-rose-300",
        "blue" => "bg-bmn-100 text-bmn-800 dark:bg-bmn-500/15 dark:text-bmn-300",
        _ => "bg-ink-100 text-ink-600 dark:bg-ink-800 dark:text-ink-300",
    };
    view! { <span class=format!("chip whitespace-nowrap {cls}")>{text}</span> }
}

/// Lencana skor kesehatan beserta predikatnya.
#[component]
pub fn LencanaSkor(skor: f64) -> impl IntoView {
    let (predikat, nada) = crate::dto::Skor::predikat(skor);
    view! { <Badge text=format!("{} · {predikat}", format_desimal(skor, 1)) tone=nada/> }
}

/// Kerangka isi kartu dengan judul dan keterangan.
#[component]
pub fn SectionCard(
    #[prop(into)] title: String,
    #[prop(into, optional)] subtitle: Option<String>,
    children: Children,
) -> impl IntoView {
    view! {
        <section class="card overflow-hidden">
            <header class="flex flex-wrap items-baseline justify-between gap-2 border-b border-ink-100 px-4 py-3 dark:border-ink-800">
                <h2 class="text-sm font-semibold">{title}</h2>
                {subtitle.map(|s| view! { <p class="text-xs text-ink-500 dark:text-ink-400">{s}</p> })}
            </header>
            <div class="p-4">{children()}</div>
        </section>
    }
}

/// Kepala halaman: judul, keterangan, dan tombol aksi.
#[component]
pub fn KepalaHalaman(
    #[prop(into)] judul: String,
    #[prop(into)] uraian: String,
    #[prop(optional)] children: Option<Children>,
) -> impl IntoView {
    view! {
        <div class="mb-4 flex flex-wrap items-end justify-between gap-3">
            <div class="min-w-0">
                <h1 class="text-lg font-semibold sm:text-xl">{judul}</h1>
                <p class="mt-0.5 max-w-3xl text-sm text-ink-500 dark:text-ink-400">{uraian}</p>
            </div>
            <div class="no-print flex flex-wrap items-center gap-2">{children.map(|c| c())}</div>
        </div>
    }
}

/// Penanda mode dan posisi data.
#[component]
pub fn PitaData(meta: Meta) -> impl IntoView {
    let posisi = if meta.tanggal_data.is_empty() {
        String::new()
    } else {
        format!("Posisi data: {}", meta.tanggal_data)
    };
    if meta.is_demo {
        view! {
            <div class="mb-4 flex flex-wrap items-center gap-2 rounded-xl border border-amber-200 bg-amber-50 px-4 py-3 text-sm text-amber-900 dark:border-amber-500/30 dark:bg-amber-500/10 dark:text-amber-200" role="status">
                <span aria-hidden="true">"🧪"</span>
                <p class="min-w-0 flex-1">
                    <span class="font-semibold">"Data contoh — bukan data riil. "</span>
                    "Seluruh angka dan satuan kerja bersifat ilustratif. Isi kredensial SLDK pada halaman Pengaturan untuk menarik data sebenarnya."
                </p>
            </div>
        }
        .into_any()
    } else {
        view! {
            <div class="mb-4 flex flex-wrap items-center gap-2 rounded-xl border border-emerald-200 bg-emerald-50 px-4 py-2.5 text-sm text-emerald-900 dark:border-emerald-500/30 dark:bg-emerald-500/10 dark:text-emerald-200" role="status">
                <span aria-hidden="true">"✅"</span>
                <p class="min-w-0 flex-1">
                    <span class="font-semibold">"Data SLDK SIMAN v2. "</span>
                    {format!("{} dari 15 kategori tersinkron.", meta.tabel_tersinkron)}
                </p>
                <span class="text-xs">{posisi}</span>
            </div>
        }
        .into_any()
    }
}

/// Kerangka pemuatan (skeleton) untuk konten yang menunggu data.
#[component]
pub fn LoadingBlock(rows: usize) -> impl IntoView {
    view! {
        <div class="space-y-3" aria-busy="true" aria-live="polite">
            <span class="sr-only">"Memuat data…"</span>
            {(0..rows)
                .map(|_| view! { <div class="h-4 w-full animate-pulse rounded bg-ink-200/70 dark:bg-ink-800"></div> })
                .collect_view()}
        </div>
    }
}

/// Pesan kesalahan yang ramah pengguna.
#[component]
pub fn ErrorBlock(#[prop(into)] message: String) -> impl IntoView {
    view! {
        <div class="rounded-xl border border-rose-200 bg-rose-50 p-4 text-sm text-rose-900 dark:border-rose-500/30 dark:bg-rose-500/10 dark:text-rose-200" role="alert">
            <p class="font-semibold">"Terjadi kendala"</p>
            <p class="mt-1 break-words">{message}</p>
        </div>
    }
}

/// Pesan ketika belum ada data sama sekali.
#[component]
pub fn EmptyBlock(#[prop(into)] title: String, #[prop(into)] message: String) -> impl IntoView {
    view! {
        <div class="grid place-items-center rounded-xl border border-dashed border-ink-200 p-8 text-center dark:border-ink-700">
            <div class="max-w-md">
                <p class="text-2xl" aria-hidden="true">"📭"</p>
                <p class="mt-2 font-semibold">{title}</p>
                <p class="mt-1 text-sm text-ink-500 dark:text-ink-400">{message}</p>
                <a class="btn-primary mt-4" href="/sinkronisasi">"Buka Sinkronisasi"</a>
            </div>
        </div>
    }
}

/// Muat hasil analitik nasional lalu serahkan ke `isi`.
#[component]
pub fn MuatAnalisis(isi: fn(Analisis) -> AnyView) -> impl IntoView {
    let res = Resource::new(|| (), |_| async move { get_analisis().await });
    view! {
        <Suspense fallback=|| view! { <LoadingBlock rows=10/> }>
            {move || {
                res.get()
                    .map(|r| match r {
                        Ok(a) if a.meta.kosong => {
                            view! {
                                <EmptyBlock
                                    title="Belum ada data"
                                    message="Tarik data dari SLDK pada halaman Sinkronisasi, atau isi kredensial pada halaman Pengaturan."
                                />
                            }
                                .into_any()
                        }
                        Ok(a) => isi(a),
                        Err(e) => view! { <ErrorBlock message=format!("Gagal memuat analitik: {e}")/> }.into_any(),
                    })
            }}
        </Suspense>
    }
}

/// Satu sel tabel.
#[derive(Debug, Clone, PartialEq)]
pub enum Sel {
    Teks(String),
    /// Angka rata kanan.
    Angka(String),
    /// Teks bertaut (teks, href).
    Tautan(String, String),
    /// Lencana (teks, nada).
    Lencana(String, &'static str),
    /// Dua baris: utama dan keterangan kecil.
    Ganda(String, String),
}

impl Sel {
    pub fn unit(v: i64) -> Sel {
        Sel::Angka(format_number(v))
    }
    pub fn unit_f(v: f64) -> Sel {
        Sel::Angka(format_number(v.round() as i64))
    }
    pub fn rupiah(v: f64) -> Sel {
        Sel::Angka(format_rupiah_short(v))
    }
    pub fn persen(v: f64) -> Sel {
        Sel::Angka(format_persen(v))
    }
    /// Persen, atau tanda pisah bila penyebutnya nol.
    pub fn persen_bila(v: f64, ada: bool) -> Sel {
        Sel::Angka(if ada { format_persen(v) } else { "–".into() })
    }
    pub fn skor(v: f64) -> Sel {
        let (p, nada) = crate::dto::Skor::predikat(v);
        Sel::Lencana(format!("{} · {p}", format_desimal(v, 1)), nada)
    }
}

fn lihat_sel(s: Sel, tebal: bool) -> AnyView {
    let dasar = if tebal {
        "px-3 py-2 font-semibold"
    } else {
        "px-3 py-2"
    };
    match s {
        Sel::Teks(t) => view! { <td class=dasar>{t}</td> }.into_any(),
        Sel::Angka(t) => {
            view! { <td class=format!("{dasar} whitespace-nowrap text-right tabular-nums")>{t}</td> }
                .into_any()
        }
        Sel::Tautan(t, href) => view! {
            <td class=dasar>
                <a class="font-medium text-bmn-700 hover:underline dark:text-bmn-300" href=href>{t}</a>
            </td>
        }
        .into_any(),
        Sel::Lencana(t, nada) => view! { <td class=dasar><Badge text=t tone=nada/></td> }.into_any(),
        Sel::Ganda(a, b) => view! {
            <td class=dasar>
                <span class="block">{a}</span>
                <span class="block text-[0.7rem] text-ink-500 dark:text-ink-400">{b}</span>
            </td>
        }
        .into_any(),
    }
}

/// Tabel data sederhana dengan baris total opsional.
#[component]
pub fn Tabel(
    /// (judul kolom, rata kanan).
    kepala: Vec<(String, bool)>,
    baris: Vec<Vec<Sel>>,
    #[prop(optional)] total: Option<Vec<Sel>>,
    #[prop(default = "Tidak ada data untuk ditampilkan.")] kosong: &'static str,
) -> impl IntoView {
    let n_kolom = kepala.len();
    let tanpa_isi = baris.is_empty();
    view! {
        <div class="table-wrap rounded-lg border border-ink-200 dark:border-ink-800">
            <table class="w-full text-left text-xs">
                <thead class="bg-ink-50 text-ink-600 dark:bg-ink-900 dark:text-ink-300">
                    <tr>
                        {kepala
                            .into_iter()
                            .map(|(judul, kanan)| {
                                view! {
                                    <th scope="col" class=format!("whitespace-nowrap px-3 py-2 font-semibold {}", if kanan { "text-right" } else { "" })>
                                        {judul}
                                    </th>
                                }
                            })
                            .collect_view()}
                    </tr>
                </thead>
                <tbody>
                    {tanpa_isi
                        .then(|| {
                            view! {
                                <tr>
                                    <td colspan=n_kolom class="px-3 py-8 text-center text-ink-500">{kosong}</td>
                                </tr>
                            }
                        })}
                    {baris
                        .into_iter()
                        .map(|sel| {
                            view! {
                                <tr class="border-t border-ink-100 dark:border-ink-800">
                                    {sel.into_iter().map(|s| lihat_sel(s, false)).collect_view()}
                                </tr>
                            }
                        })
                        .collect_view()}
                </tbody>
                {total
                    .map(|sel| {
                        view! {
                            <tfoot class="border-t-2 border-ink-200 bg-ink-50/60 dark:border-ink-700 dark:bg-ink-900/60">
                                <tr>{sel.into_iter().map(|s| lihat_sel(s, true)).collect_view()}</tr>
                            </tfoot>
                        }
                    })}
            </table>
        </div>
    }
}

/// Kepala kolom: nama saja → rata kiri; diawali `>` → rata kanan.
pub fn kepala(nama: &[&str]) -> Vec<(String, bool)> {
    nama.iter()
        .map(|n| match n.strip_prefix('>') {
            Some(s) => (s.to_string(), true),
            None => (n.to_string(), false),
        })
        .collect()
}

/// Daftar sorotan untuk pembaca cepat.
#[component]
pub fn DaftarSorotan(butir: Vec<Sorotan>) -> impl IntoView {
    view! {
        <ul class="space-y-2.5">
            {butir
                .into_iter()
                .map(|s| {
                    let (ikon, warna) = match s.nada.as_str() {
                        "positif" => ("✅", "border-emerald-400"),
                        "waspada" => ("⚠️", "border-amber-400"),
                        "kritis" => ("🚨", "border-rose-500"),
                        _ => ("ℹ️", "border-bmn-400"),
                    };
                    view! {
                        <li class=format!("flex gap-3 border-l-4 pl-3 {warna}")>
                            <span aria-hidden="true">{ikon}</span>
                            <p class="text-sm leading-relaxed">
                                <span class="font-semibold">{s.judul}". "</span>
                                {s.isi}
                            </p>
                        </li>
                    }
                })
                .collect_view()}
        </ul>
    }
}

/// Kartu satu rekomendasi; `ringkas` menyembunyikan langkah dan dasar ketentuan.
#[component]
pub fn KartuRekomendasi(r: Rekomendasi, #[prop(default = false)] ringkas: bool) -> impl IntoView {
    let (label, nada) = r.label_prioritas();
    let satker = r.satker.join("; ");
    view! {
        <article class="rounded-xl border border-ink-200 p-4 dark:border-ink-800">
            <div class="flex flex-wrap items-center gap-2">
                <Badge text=label tone=nada/>
                <Badge text=r.bidang.clone() tone="slate"/>
            </div>
            <h3 class="mt-2 text-sm font-semibold">{r.judul.clone()}</h3>
            <p class="mt-1 text-sm text-ink-600 dark:text-ink-300">{r.temuan.clone()}</p>
            {(!ringkas)
                .then(|| {
                    view! {
                        <ol class="mt-2 list-decimal space-y-1 pl-5 text-sm">
                            {r.tindakan.iter().map(|t| view! { <li>{t.clone()}</li> }).collect_view()}
                        </ol>
                        {(!satker.is_empty())
                            .then(|| {
                                view! {
                                    <p class="mt-2 text-xs text-ink-600 dark:text-ink-300">
                                        <span class="font-semibold">"Satuan kerja terkait: "</span>
                                        {satker.clone()}
                                    </p>
                                }
                            })}
                        <p class="mt-2 text-xs text-ink-500 dark:text-ink-400">
                            <span class="font-semibold">"Rujukan: "</span>
                            {r.dasar.clone()}
                        </p>
                    }
                })}
        </article>
    }
}

/// Kotak catatan metodologi/keterbatasan.
#[component]
pub fn Catatan(children: Children) -> impl IntoView {
    view! {
        <div class="rounded-xl border border-ink-200 bg-ink-50 px-4 py-3 text-xs leading-relaxed text-ink-600 dark:border-ink-800 dark:bg-ink-900 dark:text-ink-300">
            {children()}
        </div>
    }
}

/// Tabel daftar aset prioritas.
#[component]
pub fn TabelAset(
    aset: Vec<AsetRisiko>,
    /// Tampilkan kolom satuan kerja (untuk daftar nasional).
    #[prop(default = true)]
    dengan_satker: bool,
    /// Tampilkan kolom peluang rusak berat.
    #[prop(default = true)]
    dengan_peluang: bool,
) -> impl IntoView {
    let mut kep = vec!["Aset", "Kategori"];
    if dengan_satker {
        kep.push("Satuan kerja");
    }
    kep.extend([">Tahun", ">Umur", "Kondisi", ">Sisa umur", ">Nilai"]);
    if dengan_peluang {
        kep.push(">Peluang RB 5 th");
    }
    let baris = aset
        .into_iter()
        .map(|a| {
            let mut sel = vec![
                Sel::Ganda(
                    if a.nama.is_empty() {
                        a.kelompok.clone()
                    } else {
                        a.nama.clone()
                    },
                    format!("{} · No. {}", a.kd_brg, a.no_aset),
                ),
                Sel::Ganda(a.kategori.clone(), a.kelompok.clone()),
            ];
            if dengan_satker {
                sel.push(Sel::Tautan(
                    a.satker.clone(),
                    format!("/satker/{}", a.kode_satker),
                ));
            }
            sel.push(Sel::Angka(if a.tahun > 0 {
                a.tahun.to_string()
            } else {
                "–".into()
            }));
            sel.push(Sel::Angka(if a.umur >= 0.0 {
                format!("{} th", format_desimal(a.umur, 1))
            } else {
                "–".into()
            }));
            sel.push(Sel::Lencana(
                a.kondisi.clone(),
                match a.kondisi.as_str() {
                    "Baik" => "green",
                    "Rusak Ringan" => "amber",
                    "Rusak Berat" => "red",
                    _ => "slate",
                },
            ));
            sel.push(Sel::Angka(if a.sisa >= 0.0 {
                format!("{} th", format_desimal(a.sisa, 1))
            } else {
                "–".into()
            }));
            sel.push(Sel::rupiah(a.nilai));
            if dengan_peluang {
                sel.push(Sel::persen(a.p_rb * 100.0));
            }
            sel
        })
        .collect();
    view! { <Tabel kepala=kepala(&kep) baris=baris kosong="Tidak ada aset pada daftar ini."/> }
}
