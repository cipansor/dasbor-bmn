use leptos::prelude::*;

use crate::components::ui::{
    Catatan, ErrorBlock, KepalaHalaman, LoadingBlock, PitaData, SectionCard,
};
use crate::dto::SatkerBaris;
use crate::server_fns::{get_meta, get_satker};

/// Halaman laporan: pilih lingkup dan bulan pelaporan, lalu unduh PDF.
#[component]
pub fn ReportPage() -> impl IntoView {
    let satker = Resource::new(|| (), |_| async move { get_satker().await });
    let meta = Resource::new(|| (), |_| async move { get_meta().await });
    view! {
        <KepalaHalaman
            judul="Laporan PDF"
            uraian="Laporan siap cetak untuk pimpinan: ringkasan eksekutif, temuan dan rekomendasi, prediksi lima tahun, analisis kebutuhan, forecast, serta rincian per satuan kerja."
        />
        <Suspense fallback=|| view! { <LoadingBlock rows=6/> }>
            {move || meta.get().and_then(|m| m.ok()).map(|m| view! { <PitaData meta=m/> })}
            {move || {
                satker
                    .get()
                    .map(|r| match r {
                        Ok(v) => view! { <Formulir satker=v/> }.into_any(),
                        Err(e) => view! { <ErrorBlock message=format!("Gagal memuat daftar satuan kerja: {e}")/> }.into_any(),
                    })
            }}
        </Suspense>
    }
}

#[component]
fn Formulir(satker: Vec<SatkerBaris>) -> impl IntoView {
    // Kosong = bulan berjalan; server memakai zona waktu Indonesia Barat.
    let (periode, set_periode) = signal(String::new());
    let (lingkup, set_lingkup) = signal(String::new());
    let tautan = move || {
        let mut q = vec![format!("periode={}", periode.get())];
        let s = lingkup.get();
        if !s.is_empty() {
            q.push(format!("satker={s}"));
        }
        format!("/api/laporan.pdf?{}", q.join("&"))
    };
    let isi_nasional = [
        (
            "📄",
            "Sampul dan ringkasan eksekutif: posisi, sorotan utama, skor kesehatan",
        ),
        (
            "✅",
            "Temuan dan rekomendasi berprioritas beserta rujukan ketentuan",
        ),
        ("🩺", "Kondisi dan masa manfaat per kategori, profil umur"),
        (
            "🔮",
            "Prediksi habis masa manfaat dan rusak berat per tahun",
        ),
        (
            "🧮",
            "Analisis kebutuhan tiga skenario, per kategori dan kelompok barang",
        ),
        ("📈", "Forecast perolehan dan perbandingan dengan kebutuhan"),
        (
            "🏛️",
            "Peringkat provinsi dan lampiran rincian seluruh satuan kerja",
        ),
        ("📐", "Mutu data, metodologi, asumsi, dan keterbatasan"),
    ];
    view! {
        <div class="grid gap-4 lg:grid-cols-[1fr_22rem]">
            <SectionCard title="Isi Laporan" subtitle="format A4, grafik vektor">
                <ul class="grid gap-2 text-sm text-ink-700 sm:grid-cols-2 dark:text-ink-200">
                    {isi_nasional
                        .into_iter()
                        .map(|(ikon, teks)| {
                            view! {
                                <li class="flex items-start gap-2">
                                    <span aria-hidden="true">{ikon}</span>
                                    <span>{teks}</span>
                                </li>
                            }
                        })
                        .collect_view()}
                </ul>
                <div class="mt-4">
                    <Catatan>
                        <p>
                            <span class="font-semibold">"Lingkup satuan kerja. "</span>
                            "Bila satu satuan kerja dipilih, laporan memuat profil satuan kerja itu: skor, kondisi, proyeksi, kebutuhan per skenario, rincian per kategori, daftar aset prioritas, dan rekomendasinya."
                        </p>
                        <p class="mt-1">
                            <span class="font-semibold">"Bulan pelaporan. "</span>
                            "Laporan menyajikan posisi data pada sinkronisasi terakhir. Bulan pelaporan menentukan judul serta bagian perolehan pada bulan dan tahun tersebut; laporan tidak merekonstruksi posisi masa lalu."
                        </p>
                    </Catatan>
                </div>
            </SectionCard>

            <SectionCard title="Unduh" subtitle="PDF">
                <label class="block text-xs font-medium text-ink-600 dark:text-ink-300" for="lingkup">"Lingkup"</label>
                <select id="lingkup" class="input mt-1" on:change=move |ev| set_lingkup.set(event_target_value(&ev))>
                    <option value="">"Nasional — Kejaksaan RI"</option>
                    {satker.into_iter().map(|s| view! { <option value=s.kode.clone()>{s.nama.clone()}</option> }).collect_view()}
                </select>
                <label class="mt-3 block text-xs font-medium text-ink-600 dark:text-ink-300" for="periode">"Bulan pelaporan"</label>
                <input
                    id="periode"
                    type="month"
                    class="input mt-1"
                    prop:value=move || periode.get()
                    on:input=move |ev| set_periode.set(event_target_value(&ev))
                />
                <a class="btn-primary mt-4 w-full" href=tautan rel="external">
                    <span aria-hidden="true">"⬇️"</span>
                    <span>"Unduh laporan PDF"</span>
                </a>
                <p class="mt-2 text-xs text-ink-500 dark:text-ink-400">"Kosongkan bulan untuk memakai bulan berjalan."</p>
                <hr class="my-4 border-ink-200 dark:border-ink-800"/>
                <p class="text-xs font-medium text-ink-600 dark:text-ink-300">"Lampiran data (CSV)"</p>
                <div class="mt-2 flex flex-col gap-2">
                    <a class="btn-ghost" href="/api/satker.csv" rel="external">"⬇️ Ringkasan per satuan kerja"</a>
                    <a class="btn-ghost" href="/api/kebutuhan.csv" rel="external">"⬇️ Kebutuhan per satuan kerja × kategori"</a>
                </div>
            </SectionCard>
        </div>
    }
}
