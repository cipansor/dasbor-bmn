use leptos::prelude::*;

use crate::components::ui::{
    Badge, DaftarSorotan, KartuRekomendasi, KepalaHalaman, MuatAnalisis, PitaData, SectionCard,
};
use crate::dto::Analisis;

/// Seluruh rekomendasi tindak lanjut, urut prioritas.
#[component]
pub fn RekomendasiPage() -> impl IntoView {
    view! { <MuatAnalisis isi=isi/> }
}

fn isi(a: Analisis) -> AnyView {
    let hitung = |p: u8| a.rekomendasi.iter().filter(|r| r.prioritas == p).count();
    let (tinggi, sedang, rendah) = (hitung(1), hitung(2), hitung(3));
    let kelompok = |p: u8, judul: &'static str| {
        let daftar: Vec<_> = a
            .rekomendasi
            .iter()
            .filter(|r| r.prioritas == p)
            .cloned()
            .collect();
        (!daftar.is_empty()).then(|| {
            view! {
                <h2 class="mb-2 mt-5 text-sm font-semibold uppercase tracking-wide text-ink-500 dark:text-ink-400">{judul}</h2>
                <div class="grid gap-3 lg:grid-cols-2">
                    {daftar.into_iter().map(|x| view! { <KartuRekomendasi r=x/> }).collect_view()}
                </div>
            }
        })
    };
    view! {
        <KepalaHalaman
            judul="Rekomendasi"
            uraian="Tindak lanjut yang disarankan dari hasil analitik: temuan berangka, langkah, satuan kerja terkait, dan rujukan ketentuan."
        >
            <a class="btn-primary" href="/laporan">"🧾 Unduh laporan"</a>
        </KepalaHalaman>
        <PitaData meta=a.meta.clone()/>

        <div class="flex flex-wrap gap-2">
            <Badge text=format!("{tinggi} prioritas tinggi") tone="red"/>
            <Badge text=format!("{sedang} prioritas sedang") tone="amber"/>
            <Badge text=format!("{rendah} prioritas rendah") tone="blue"/>
        </div>

        <div class="mt-4">
            <SectionCard title="Dasar Temuan" subtitle="sorotan utama">
                <DaftarSorotan butir=a.sorotan.clone()/>
            </SectionCard>
        </div>

        {kelompok(1, "Prioritas tinggi")}
        {kelompok(2, "Prioritas sedang")}
        {kelompok(3, "Prioritas rendah")}

        <p class="mt-5 text-xs text-ink-500 dark:text-ink-400">
            "Rekomendasi dihasilkan dari aturan baku atas data yang tersinkron. Rujukan ketentuan bersifat penunjuk awal; pastikan ketentuan yang berlaku sebelum menindaklanjuti."
        </p>
    }
    .into_any()
}
