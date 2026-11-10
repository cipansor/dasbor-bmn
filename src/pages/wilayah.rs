use leptos::prelude::*;

use crate::components::chart::{BarList, W_RB};
use crate::components::ui::{
    kepala, KepalaHalaman, MuatAnalisis, PitaData, SectionCard, Sel, Tabel,
};
use crate::dto::Analisis;
use crate::model::{format_number, format_rupiah_short, format_unit};

/// Sebaran per provinsi (lokasi aset).
#[component]
pub fn WilayahPage() -> impl IntoView {
    view! { <MuatAnalisis isi=isi/> }
}

fn isi(a: Analisis) -> AnyView {
    let akhir = a.meta.tahun_akhir();
    let baris: Vec<Vec<Sel>> = a
        .provinsi
        .iter()
        .map(|p| {
            let r = &p.r;
            vec![
                Sel::Teks(p.nama.clone()),
                Sel::unit(p.satker),
                Sel::unit(r.unit),
                Sel::rupiah(r.nilai),
                Sel::persen_bila(r.pct_baik(), r.kondisi_diketahui() > 0),
                Sel::unit(r.rb),
                Sel::Angka(format!("{} ({})", format_number(r.habis), crate::model::format_persen(r.pct_habis()))),
                Sel::unit(r.idle),
                Sel::unit(r.hilang),
                Sel::unit_f(r.rb_5()),
                Sel::unit_f(r.ganti_total(1)),
                Sel::rupiah(r.biaya_total(1)),
                Sel::skor(p.skor),
            ]
        })
        .collect();
    let n = &a.nasional;
    let total = vec![
        Sel::Teks("Nasional".into()),
        Sel::unit(a.meta.total_satker),
        Sel::unit(n.unit),
        Sel::rupiah(n.nilai),
        Sel::persen(n.pct_baik()),
        Sel::unit(n.rb),
        Sel::Angka(format!("{} ({})", format_number(n.habis), crate::model::format_persen(n.pct_habis()))),
        Sel::unit(n.idle),
        Sel::unit(n.hilang),
        Sel::unit_f(n.rb_5()),
        Sel::unit_f(n.ganti_total(1)),
        Sel::rupiah(n.biaya_total(1)),
        Sel::skor(a.skor.total),
    ];
    let mut butuh: Vec<_> = a.provinsi.iter().collect();
    butuh.sort_by(|x, y| y.r.biaya_total(1).total_cmp(&x.r.biaya_total(1)));
    let butuh: Vec<(String, f64, String, Option<String>)> = butuh
        .iter()
        .take(12)
        .map(|p| {
            (
                p.nama.clone(),
                p.r.biaya_total(1),
                format!("{} · {} unit", format_rupiah_short(p.r.biaya_total(1)), format_unit(p.r.ganti_total(1))),
                None,
            )
        })
        .collect();
    let mut rusak: Vec<_> = a.provinsi.iter().filter(|p| p.r.kondisi_diketahui() >= 30).collect();
    rusak.sort_by(|x, y| y.r.pct_rb().total_cmp(&x.r.pct_rb()));
    let rusak: Vec<(String, f64, String, Option<String>)> = rusak
        .iter()
        .take(12)
        .map(|p| {
            (
                p.nama.clone(),
                p.r.pct_rb(),
                format!("{} · {} unit", crate::model::format_persen(p.r.pct_rb()), format_number(p.r.rb)),
                None,
            )
        })
        .collect();

    view! {
        <KepalaHalaman
            judul="Wilayah"
            uraian="Perbandingan antarprovinsi menurut lokasi aset. Jumlah satuan kerja dihitung dari provinsi tempat sebagian besar aset satuan kerja berada."
        />
        <PitaData meta=a.meta.clone()/>
        <div class="grid gap-4 lg:grid-cols-2">
            <SectionCard title="Porsi Rusak Berat Tertinggi" subtitle="% aset fisik · 12 provinsi">
                <BarList data=rusak warna=W_RB/>
            </SectionCard>
            <SectionCard title="Kebutuhan Biaya Terbesar" subtitle=format!("skenario moderat s.d. {akhir} · 12 provinsi")>
                <BarList data=butuh/>
            </SectionCard>
        </div>
        <div class="mt-4">
            <SectionCard title="Seluruh Provinsi" subtitle="urut nilai aset">
                <Tabel
                    kepala=kepala(&["Provinsi", ">Satker", ">Unit", ">Nilai", ">Baik", ">RB", ">Habis MM", ">Idle", ">Hilang", ">RB baru 5 th", ">Kebutuhan 5 th", ">Perkiraan biaya", "Skor"])
                    baris=baris
                    total=total
                />
            </SectionCard>
        </div>
    }
    .into_any()
}
