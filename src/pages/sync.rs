use std::time::Duration;

use leptos::prelude::*;

use crate::components::ui::{Badge, ErrorBlock, KepalaHalaman, LoadingBlock, SectionCard};
use crate::dto::Progres;
use crate::model::format_number;
use crate::server_fns::{get_progres, get_sync_statuses, mulai_sinkron, reset_cache};

/// Tanyakan kemajuan berkala selama pekerjaan latar berjalan.
fn pantau(set_progres: WriteSignal<Option<Progres>>, set_refresh: WriteSignal<u32>) {
    leptos::task::spawn_local(async move {
        if let Ok(p) = get_progres().await {
            let berjalan = p.berjalan;
            let pernah = !p.mulai.is_empty();
            set_progres.set(Some(p));
            if berjalan {
                set_timeout(
                    move || pantau(set_progres, set_refresh),
                    Duration::from_millis(1500),
                );
            } else if pernah {
                set_refresh.update(|v| *v += 1);
            }
        }
    });
}

/// Halaman sinkronisasi: status per tabel, tarik data di latar, dan reset cache.
#[component]
pub fn SyncPage() -> impl IntoView {
    let (refresh, set_refresh) = signal(0_u32);
    let (progres, set_progres) = signal(Option::<Progres>::None);
    let (error, set_error) = signal(Option::<String>::None);
    let statuses = Resource::new(
        move || refresh.get(),
        |_| async move { get_sync_statuses().await },
    );
    let berjalan = move || progres.with(|p| p.as_ref().is_some_and(|p| p.berjalan));

    // Saat halaman dibuka, lanjutkan memantau pekerjaan yang mungkin masih berjalan.
    Effect::new(move |_| pantau(set_progres, set_refresh));

    let mulai = move |table: String| {
        set_error.set(None);
        leptos::task::spawn_local(async move {
            match mulai_sinkron(table).await {
                Ok(p) => {
                    set_progres.set(Some(p));
                    pantau(set_progres, set_refresh);
                }
                Err(e) => set_error.set(Some(e.to_string())),
            }
        });
    };

    view! {
        <KepalaHalaman
            judul="Sinkronisasi"
            uraian="Tarik data terbaru dari Web Service SLDK SIMAN v2. Penarikan berjalan di latar; halaman boleh ditinggalkan dan data lama tetap tersaji sampai unduhan lengkap."
        />
        <div class="space-y-4">
            <SectionCard title="Tarik Data" subtitle="sumber diperbarui mingguan">
                <div class="flex flex-wrap items-center gap-2">
                    <button type="button" class="btn-primary" disabled=berjalan on:click=move |_| mulai(String::new())>
                        {move || if berjalan() { "⏳ Sedang berjalan…" } else { "🔄 Sinkronkan semua kategori" }}
                    </button>
                    <button
                        type="button"
                        class="btn-ghost"
                        disabled=berjalan
                        on:click=move |_| {
                            set_error.set(None);
                            leptos::task::spawn_local(async move {
                                match reset_cache().await {
                                    Ok(_) => set_refresh.update(|v| *v += 1),
                                    Err(e) => set_error.set(Some(e.to_string())),
                                }
                            });
                        }
                    >
                        "🗑️ Kosongkan data lokal"
                    </button>
                </div>
                {move || error.get().map(|e| view! { <div class="mt-3"><ErrorBlock message=e/></div> })}
                {move || {
                    progres
                        .get()
                        .filter(|p| !p.mulai.is_empty())
                        .map(|p| {
                            let selesai = p.tabel.iter().filter(|t| t.status == "selesai").count();
                            let gagal = p.tabel.iter().filter(|t| t.status == "gagal").count();
                            view! {
                                <div class="mt-4 rounded-xl border border-ink-200 p-3 dark:border-ink-800" aria-live="polite">
                                    <p class="text-sm font-semibold">
                                        {if p.berjalan { "Sinkronisasi berjalan" } else { "Sinkronisasi terakhir" }}
                                        <span class="ml-2 text-xs font-normal text-ink-500 dark:text-ink-400">
                                            {format!("mulai {} · {selesai} selesai · {gagal} gagal dari {} tabel", p.mulai, p.tabel.len())}
                                        </span>
                                    </p>
                                    <ul class="mt-2 space-y-2">
                                        {p.tabel
                                            .iter()
                                            .filter(|t| t.status != "antre")
                                            .map(|t| {
                                                let pct = if t.total > 0 { (t.terambil as f64 / t.total as f64 * 100.0).clamp(0.0, 100.0) } else if t.status == "selesai" { 100.0 } else { 0.0 };
                                                let warna = match t.status.as_str() {
                                                    "gagal" => "#ef4444",
                                                    "selesai" => "#10b981",
                                                    _ => "#1f47f5",
                                                };
                                                view! {
                                                    <li class="text-xs">
                                                        <div class="flex justify-between gap-3">
                                                            <span>{t.label.clone()}</span>
                                                            <span class="tabular-nums text-ink-500 dark:text-ink-400">
                                                                {format!("{} / {} baris · {}", format_number(t.terambil), format_number(t.total), t.status)}
                                                            </span>
                                                        </div>
                                                        <div class="mt-1 h-1.5 overflow-hidden rounded-full bg-ink-100 dark:bg-ink-800">
                                                            <div class="h-full rounded-full" style=format!("width:{pct:.1}%;background:{warna}")></div>
                                                        </div>
                                                        {(!t.pesan.is_empty()).then(|| view! { <p class="mt-1 break-words text-ink-600 dark:text-ink-300">{t.pesan.clone()}</p> })}
                                                    </li>
                                                }
                                            })
                                            .collect_view()}
                                    </ul>
                                </div>
                            }
                        })
                }}
            </SectionCard>

            <SectionCard title="Status per Kategori" subtitle="isi penyimpanan lokal">
                <Transition fallback=|| view! { <LoadingBlock rows=6/> }>
                    {move || {
                        statuses
                            .get()
                            .map(|res| match res {
                                Ok(list) => {
                                    view! {
                                        <div class="table-wrap rounded-lg border border-ink-200 dark:border-ink-800">
                                            <table class="w-full text-left text-xs">
                                                <thead class="bg-ink-50 text-ink-600 dark:bg-ink-900 dark:text-ink-300">
                                                    <tr>
                                                        <th scope="col" class="px-3 py-2 font-semibold">"Kategori"</th>
                                                        <th scope="col" class="px-3 py-2 font-semibold">"Tabel sumber"</th>
                                                        <th scope="col" class="px-3 py-2 text-right font-semibold">"Baris"</th>
                                                        <th scope="col" class="px-3 py-2 font-semibold">"Sinkron terakhir"</th>
                                                        <th scope="col" class="px-3 py-2 font-semibold">"Status"</th>
                                                        <th scope="col" class="px-3 py-2 font-semibold"></th>
                                                    </tr>
                                                </thead>
                                                <tbody>
                                                    {list
                                                        .into_iter()
                                                        .map(|s| {
                                                            let tabel = s.table.clone();
                                                            let ada = s.last_sync.is_some();
                                                            view! {
                                                                <tr class="border-t border-ink-100 dark:border-ink-800">
                                                                    <td class="px-3 py-2 font-medium">{s.label.clone()}</td>
                                                                    <td class="px-3 py-2"><code class="text-[0.7rem] text-ink-500">{s.table.clone()}</code></td>
                                                                    <td class="px-3 py-2 text-right tabular-nums">{format_number(s.row_count)}</td>
                                                                    <td class="px-3 py-2 text-ink-600 dark:text-ink-300">{s.last_sync.clone().unwrap_or_else(|| "—".into())}</td>
                                                                    <td class="px-3 py-2">
                                                                        {if ada {
                                                                            view! { <Badge text="tersinkron" tone="green"/> }.into_any()
                                                                        } else {
                                                                            view! { <Badge text="belum" tone="slate"/> }.into_any()
                                                                        }}
                                                                    </td>
                                                                    <td class="px-3 py-2 text-right">
                                                                        <button type="button" class="btn-ghost px-2.5 py-1" disabled=berjalan on:click=move |_| mulai(tabel.clone())>
                                                                            "Tarik"
                                                                        </button>
                                                                    </td>
                                                                </tr>
                                                            }
                                                        })
                                                        .collect_view()}
                                                </tbody>
                                            </table>
                                        </div>
                                    }
                                        .into_any()
                                }
                                Err(e) => view! { <ErrorBlock message=format!("Gagal memuat status: {e}")/> }.into_any(),
                            })
                    }}
                </Transition>
            </SectionCard>

            <SectionCard title="Catatan Proses" subtitle="terbaru di bawah">
                {move || {
                    let log = progres.with(|p| p.as_ref().map(|p| p.log.clone()).unwrap_or_default());
                    if log.is_empty() {
                        view! { <p class="text-sm text-ink-500 dark:text-ink-400">"Belum ada catatan pada sesi server ini."</p> }.into_any()
                    } else {
                        view! {
                            <ol class="max-h-72 space-y-1 overflow-y-auto font-mono text-[0.7rem] leading-relaxed text-ink-600 dark:text-ink-300">
                                {log.into_iter().map(|l| view! { <li class="break-words">{l}</li> }).collect_view()}
                            </ol>
                        }
                            .into_any()
                    }
                }}
            </SectionCard>
        </div>
    }
}
