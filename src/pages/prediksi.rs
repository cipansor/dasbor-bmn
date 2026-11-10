use leptos::prelude::*;

use crate::components::chart::{KolomTumpuk, Seri, W_RB, W_UNGU};
use crate::components::ui::{
    kepala, Catatan, KepalaHalaman, Kpi, MuatAnalisis, PitaData, SectionCard, Sel, Tabel, TabelAset,
};
use crate::dto::{Analisis, Ringkas, NH};
use crate::model::{format_desimal, format_number, format_persen, format_unit};

/// Prediksi lima tahun: habis masa manfaat dan rusak berat, per kategori dan satuan kerja.
#[component]
pub fn PrediksiPage() -> impl IntoView {
    view! { <MuatAnalisis isi=isi/> }
}

fn baris_tahun(nama: String, kini: i64, th: &[f64; NH]) -> Vec<Sel> {
    let mut sel = vec![Sel::Teks(nama), Sel::unit(kini)];
    sel.extend(th.iter().map(|v| Sel::unit_f(*v)));
    sel.push(Sel::unit_f(th.iter().sum()));
    sel
}

fn isi(a: Analisis) -> AnyView {
    let r: Ringkas = a.nasional.clone();
    let m = a.meta.clone();
    let akhir = m.tahun_akhir();
    let label_th: Vec<String> = (0..NH).map(|k| m.label_tahun(k)).collect();
    let mut kep: Vec<String> = vec!["Kategori".into(), ">Sudah".into()];
    kep.extend(label_th.iter().map(|l| format!(">{l}")));
    kep.push(format!(">Tambahan s.d. {akhir}"));
    let kep_ref: Vec<&str> = kep.iter().map(|s| s.as_str()).collect();

    let habis: Vec<Vec<Sel>> = a
        .kategori
        .iter()
        .filter(|c| c.disusutkan)
        .map(|c| baris_tahun(format!("{} {}", c.icon, c.label), c.r.habis, &c.r.habis_th))
        .collect();
    let rb: Vec<Vec<Sel>> = a
        .kategori
        .iter()
        .filter(|c| c.fisik)
        .map(|c| baris_tahun(format!("{} {}", c.icon, c.label), c.r.rb, &c.r.rb_th))
        .collect();

    let satker_rb: Vec<Vec<Sel>> = a
        .satker_rb
        .iter()
        .map(|s| {
            vec![
                Sel::Tautan(s.nama.clone(), format!("/satker/{}", s.kode)),
                Sel::Teks(s.prov.clone()),
                Sel::unit(s.unit),
                Sel::unit(s.rb),
                Sel::unit(s.rr),
                Sel::unit_f(s.rb5),
                Sel::unit(s.habis),
                Sel::unit_f(s.habis5),
                Sel::Angka(format!("{}×", format_desimal(s.rku, 2))),
            ]
        })
        .collect();
    let model: Vec<Vec<Sel>> = a
        .model
        .iter()
        .map(|x| {
            vec![
                Sel::Teks(x.label.clone()),
                Sel::unit(x.n),
                Sel::Angka(format_desimal(x.median_mulai, 1)),
                Sel::Angka(format_desimal(x.lama_rr, 1)),
                Sel::Angka(format_desimal(x.bentuk, 2)),
                Sel::Angka(format_desimal(x.mae, 1)),
                Sel::Teks(if x.gabungan { "kurva gabungan".into() } else { "data kategori".into() }),
            ]
        })
        .collect();
    let rr_persen = if r.rr > 0 {
        format_persen(100.0 * r.rb_5() / (r.rr as f64 + r.baik as f64).max(1.0))
    } else {
        "–".into()
    };

    view! {
        <KepalaHalaman
            judul="Prediksi 5 Tahun"
            uraian=format!("Aset yang akan habis masa manfaat dan yang diperkirakan menjadi rusak berat sampai akhir {akhir}, dihitung dari umur, sisa umur, dan kondisi tiap aset hari ini.")
        >
            <a class="btn-ghost" href="/api/satker.csv">"⬇️ Rincian per satuan kerja (CSV)"</a>
        </KepalaHalaman>
        <PitaData meta=m.clone()/>

        <div class="grid grid-cols-2 gap-3 lg:grid-cols-4">
            <Kpi label="Sudah Habis Masa Manfaat" value=format!("{} unit", format_number(r.habis)) hint=format!("{} dari aset yang disusutkan", format_persen(r.pct_habis())) icon="⏳" tone="violet"/>
            <Kpi label=format!("Habis s.d. {akhir}") value=format!("+{} unit", format_unit(r.habis_5())) hint="tambahan dari posisi hari ini" icon="📅" tone="violet"/>
            <Kpi label="Rusak Berat Saat Ini" value=format!("{} unit", format_number(r.rb)) hint=format!("{} dari aset fisik", format_persen(r.pct_rb())) icon="🛠️" tone="rose"/>
            <Kpi label=format!("Rusak Berat Baru s.d. {akhir}") value=format!("±{} unit", format_unit(r.rb_5())) hint=format!("{rr_persen} dari aset yang kini belum rusak berat") icon="🔮" tone="rose"/>
        </div>

        <div class="mt-4 grid gap-4 lg:grid-cols-2">
            <SectionCard title="Habis Masa Manfaat per Tahun" subtitle="unit · * = sisa tahun berjalan">
                <KolomTumpuk label=label_th.clone() seri=vec![Seri { nama: "Habis masa manfaat".into(), warna: W_UNGU, nilai: r.habis_th.to_vec() }] satuan="unit"/>
            </SectionCard>
            <SectionCard title="Perkiraan Rusak Berat Baru per Tahun" subtitle="unit · ekspektasi">
                <KolomTumpuk label=label_th seri=vec![Seri { nama: "Rusak berat baru".into(), warna: W_RB, nilai: r.rb_th.to_vec() }] satuan="unit"/>
            </SectionCard>
        </div>

        <div class="mt-4">
            <SectionCard title="Jadwal Habis Masa Manfaat per Kategori" subtitle="unit · pasti menurut sisa umur pada SIMAN">
                <Tabel kepala=kepala(&kep_ref) baris=habis total=baris_tahun("Seluruh kategori".into(), r.habis, &r.habis_th)/>
            </SectionCard>
        </div>

        <div class="mt-4">
            <SectionCard title="Perkiraan Rusak Berat Baru per Kategori" subtitle="unit · ekspektasi model, kolom Sudah = rusak berat saat ini">
                <Tabel kepala=kepala(&kep_ref) baris=rb total=baris_tahun("Seluruh kategori".into(), r.rb, &r.rb_th)/>
            </SectionCard>
        </div>

        <div class="mt-4">
            <SectionCard title="Satuan Kerja dengan Perkiraan Rusak Berat Terbesar" subtitle="10 teratas · buka Satuan Kerja untuk daftar lengkap">
                <Tabel
                    kepala=kepala(&["Satuan kerja", "Provinsi", ">Unit", ">RB kini", ">Rusak ringan", ">RB baru 5 th", ">Habis MM kini", ">Habis MM 5 th", ">RKU"])
                    baris=satker_rb
                />
                <a class="no-print btn-ghost mt-3" href="/satker">"Rincian seluruh satuan kerja →"</a>
            </SectionCard>
        </div>

        <div class="mt-4">
            <SectionCard title="Aset Paling Berisiko Menjadi Rusak Berat" subtitle="belum rusak berat, peluang tertinggi dalam 5 tahun">
                <TabelAset aset=a.aset_risiko.clone()/>
            </SectionCard>
        </div>

        <div class="mt-4">
            <SectionCard title="Parameter Model Kerusakan" subtitle="ditaksir dari data umur dan kondisi">
                <Tabel
                    kepala=kepala(&["Kategori", ">Sampel", ">Median mulai tidak baik (th)", ">Rata-rata rusak ringan → berat (th)", ">Bentuk kurva", ">Selisih model–data (poin %)", "Sumber"])
                    baris=model
                />
                <div class="mt-3">
                    <Catatan>
                        <p>
                            <span class="font-semibold">"Cara membaca. "</span>
                            "Habis masa manfaat bersifat pasti karena mengikuti sisa umur ekonomis dari SIMAN. Rusak berat bersifat peluang: tiap aset diberi peluang berdasarkan kategori, umur, kondisi saat ini, dan rekam kerusakan satuan kerjanya, lalu dijumlahkan menjadi ekspektasi."
                        </p>
                        <p class="mt-1">
                            <span class="font-semibold">"Keterbatasan. "</span>
                            "Aset rusak berat yang sudah dihapus tidak lagi tercatat, sehingga angka proyeksi cenderung batas bawah. Proyeksi tidak memperhitungkan pemeliharaan besar atau pengadaan pengganti di masa depan. Penjelasan lengkap ada di halaman Metodologi."
                        </p>
                    </Catatan>
                </div>
            </SectionCard>
        </div>
    }
    .into_any()
}
