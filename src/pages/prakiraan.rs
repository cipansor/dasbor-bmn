use leptos::prelude::*;

use crate::components::chart::{GrafikRamal, KolomTumpuk, Seri, W_KEDUA, W_UTAMA};
use crate::components::ui::{
    kepala, Catatan, KepalaHalaman, Kpi, MuatAnalisis, PitaData, SectionCard, Sel, Tabel,
};
use crate::dto::{Analisis, Deret, HORIZON};
use crate::model::{format_desimal, format_persen, format_rupiah_short, format_unit};

/// Forecast perolehan per kategori dan proyeksi nilai buku.
#[component]
pub fn PrakiraanPage() -> impl IntoView {
    view! { <MuatAnalisis isi=isi/> }
}

fn galat(d: &Deret) -> String {
    if d.galat < 0.0 {
        "–".into()
    } else {
        format_persen(d.galat)
    }
}

/// Jumlah prakiraan lima tahun mendatang (tanpa taksiran tahun berjalan).
fn lima_tahun(d: &Deret) -> f64 {
    d.ramal.iter().skip(1).map(|t| t.nilai).sum()
}

fn isi(a: Analisis) -> AnyView {
    let m = a.meta.clone();
    let akhir = m.tahun_akhir();
    let total = a.prakiraan.first().cloned().unwrap_or_default();
    let rata_unit = total
        .unit
        .riwayat
        .iter()
        .rev()
        .take(5)
        .map(|p| p.1)
        .sum::<f64>()
        / 5.0;
    let ramal_unit = lima_tahun(&total.unit);
    let ramal_nilai = lima_tahun(&total.nilai);
    let butuh = a.nasional.biaya_total(1);

    let mut kep: Vec<String> = vec!["Kategori".into(), format!(">{} s.d. kini", m.tahun0)];
    kep.extend(total.unit.ramal.iter().map(|t| {
        if t.tahun == m.tahun0 {
            format!(">{}*", t.tahun)
        } else {
            format!(">{}", t.tahun)
        }
    }));
    kep.push(">Galat uji balik".into());
    kep.push("Metode".into());
    let kep_unit = kepala(&kep.iter().map(|x| x.as_str()).collect::<Vec<_>>());
    let kep_nilai = kep_unit.clone();
    let tabel = |ambil: fn(&crate::dto::Prakiraan) -> (&Deret, f64), rupiah: bool| -> Vec<Vec<Sel>> {
        a.prakiraan
            .iter()
            .map(|p| {
                let (d, berjalan) = ambil(p);
                let f = |v: f64| if rupiah { Sel::rupiah(v) } else { Sel::unit_f(v) };
                let mut sel = vec![Sel::Teks(p.label.clone()), f(berjalan)];
                sel.extend(d.ramal.iter().map(|t| f(t.nilai)));
                sel.push(Sel::Angka(galat(d)));
                sel.push(Sel::Teks(d.metode.clone()));
                sel
            })
            .collect()
    };
    let baris_unit = tabel(|p| (&p.unit, p.berjalan.0), false);
    let baris_nilai = tabel(|p| (&p.nilai, p.berjalan.1), true);

    let buku_label: Vec<String> = a
        .buku_th
        .iter()
        .map(|b| if b.tahun == 0 { "Kini".to_string() } else { b.tahun.to_string() })
        .collect();
    let buku_alat: Vec<f64> = a.buku_th.iter().map(|b| b.peralatan).collect();
    let turun = match (a.buku_th.first(), a.buku_th.last()) {
        (Some(x), Some(y)) if x.peralatan > 0.0 => 100.0 * (1.0 - y.peralatan / x.peralatan),
        _ => 0.0,
    };

    let semua = StoredValue::new(a.prakiraan.clone());
    let (pilih, set_pilih) = signal(0usize);
    let (rupiah, set_rupiah) = signal(false);
    let pilihan: Vec<(usize, String)> = a
        .prakiraan
        .iter()
        .enumerate()
        .map(|(i, p)| (i, p.label.clone()))
        .collect();

    view! {
        <KepalaHalaman
            judul="Forecast Perolehan"
            uraian=format!("Arah perolehan aset sampai {akhir} bila pola historis berlanjut, dibandingkan dengan kebutuhan penggantian, serta proyeksi nilai buku.")
        />
        <PitaData meta=m.clone()/>

        <div class="grid grid-cols-2 gap-3 lg:grid-cols-4">
            <Kpi label="Rata-rata Perolehan 5 Tahun" value=format!("{} unit/th", format_unit(rata_unit)) hint="tahun penuh terakhir" icon="📦"/>
            <Kpi label=format!("Prakiraan {}–{akhir}", m.tahun0 + 1) value=format!("{} unit", format_unit(ramal_unit)) hint=format!("rata-rata {} unit/th", format_unit(ramal_unit / HORIZON as f64)) icon="📈"/>
            <Kpi label="Prakiraan Nilai Perolehan" value=format_rupiah_short(ramal_nilai) hint=format!("galat uji balik {}", galat(&total.nilai)) icon="💰" tone="emerald"/>
            <Kpi
                label="Kebutuhan Moderat vs Tren"
                value={if ramal_nilai > 0.0 { format!("{}×", format_desimal(butuh / ramal_nilai, 1)) } else { "–".into() }}
                hint=format!("kebutuhan {} : tren perolehan", format_rupiah_short(butuh))
                icon="⚖️"
                tone={if butuh > ramal_nilai { "amber" } else { "emerald" }}
            />
        </div>

        <div class="mt-4">
            <SectionCard title="Riwayat dan Prakiraan" subtitle="pita = rentang ketidakpastian 80%">
                <div class="no-print mb-2 flex flex-wrap items-center gap-3 text-xs">
                    <label>
                        <span class="mr-2 font-medium text-ink-600 dark:text-ink-300">"Kategori"</span>
                        <select class="input inline-block w-auto" on:change=move |ev| set_pilih.set(event_target_value(&ev).parse().unwrap_or(0))>
                            {pilihan.into_iter().map(|(i, l)| view! { <option value=i.to_string()>{l}</option> }).collect_view()}
                        </select>
                    </label>
                    <label>
                        <span class="mr-2 font-medium text-ink-600 dark:text-ink-300">"Ukuran"</span>
                        <select class="input inline-block w-auto" on:change=move |ev| set_rupiah.set(event_target_value(&ev) == "nilai")>
                            <option value="unit">"Jumlah unit"</option>
                            <option value="nilai">"Nilai perolehan"</option>
                        </select>
                    </label>
                </div>
                {move || {
                    semua
                        .with_value(|v| v.get(pilih.get()).cloned())
                        .map(|p| {
                            let pakai_rupiah = rupiah.get();
                            let d = if pakai_rupiah { p.nilai.clone() } else { p.unit.clone() };
                            let metode = d.metode.clone();
                            let g = galat(&d);
                            view! {
                                <div class="mx-auto max-w-4xl">
                                    <GrafikRamal riwayat=d.riwayat ramal=d.ramal satuan={if pakai_rupiah { "rupiah" } else { "unit" }}/>
                                </div>
                                <p class="mt-2 text-xs text-ink-600 dark:text-ink-300">
                                    {format!("{} — metode terpilih: {metode}; galat uji balik (sMAPE) {g}.", p.label)}
                                </p>
                            }
                        })
                }}
            </SectionCard>
        </div>

        <div class="mt-4">
            <SectionCard title="Prakiraan Jumlah Unit per Kategori" subtitle="* = taksiran setahun penuh tahun berjalan">
                <Tabel kepala=kep_unit baris=baris_unit/>
            </SectionCard>
        </div>
        <div class="mt-4">
            <SectionCard title="Prakiraan Nilai Perolehan per Kategori" subtitle="rupiah">
                <Tabel kepala=kep_nilai baris=baris_nilai/>
            </SectionCard>
        </div>

        <div class="mt-4 grid gap-4 lg:grid-cols-2">
            <SectionCard title="Proyeksi Nilai Buku Peralatan" subtitle="portofolio yang ada, tanpa pengadaan baru">
                <KolomTumpuk label=buku_label seri=vec![Seri { nama: "Nilai buku peralatan".into(), warna: W_KEDUA, nilai: buku_alat }] satuan="rupiah"/>
                <p class="mt-2 text-xs text-ink-600 dark:text-ink-300">
                    {format!("Tanpa belanja modal baru, nilai buku peralatan dan mesin menyusut {} sampai akhir {akhir}.", format_persen(turun))}
                </p>
            </SectionCard>
            <SectionCard title="Perolehan vs Kebutuhan" subtitle=format!("{}–{akhir}", m.tahun0 + 1)>
                <KolomTumpuk
                    label=vec!["Tren perolehan".to_string(), "Kebutuhan dasar".to_string(), "Kebutuhan moderat".to_string(), "Kebutuhan maksimal".to_string()]
                    seri=vec![Seri {
                        nama: "Rupiah".into(),
                        warna: W_UTAMA,
                        nilai: vec![ramal_nilai, a.nasional.biaya_total(0), a.nasional.biaya_total(1), a.nasional.biaya_total(2)],
                    }]
                    satuan="rupiah"
                />
                <p class="mt-2 text-xs text-ink-600 dark:text-ink-300">
                    "Tren perolehan mencakup seluruh kategori termasuk tanah dan pengadaan baru; kebutuhan hanya penggantian dan rehabilitasi aset yang ada."
                </p>
            </SectionCard>
        </div>

        <div class="mt-4">
            <Catatan>
                <p>
                    <span class="font-semibold">"Cara kerja. "</span>
                    "Tiga metode (rata-rata 3 tahun, tren linear, pemulusan eksponensial Holt teredam) diadu lewat uji balik: tiap metode meramal tahun-tahun terakhir hanya dari data sebelumnya. Metode dengan galat terkecil dipakai. Galat di atas ±30% berarti pola historis tidak stabil dan prakiraan sebaiknya dibaca sebagai arah, bukan angka pasti."
                </p>
                <p class="mt-1">
                    <span class="font-semibold">"Keterbatasan. "</span>
                    "Riwayat hanya memuat aset yang masih tercatat; aset lama yang sudah dihapus tidak terlihat sehingga tahun-tahun lampau tampak lebih kecil. Prakiraan melanjutkan pola anggaran masa lalu dan tidak mengetahui kebijakan anggaran mendatang."
                </p>
            </Catatan>
        </div>
    }
    .into_any()
}
