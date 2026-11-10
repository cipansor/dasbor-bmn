//! Grafik SVG buatan sendiri.
//!
//! Tiap grafik dibentuk sebagai teks SVG oleh fungsi murni, lalu ditanam lewat
//! `inner_html`. Dengan begitu geometri dapat diuji tanpa peramban, hidrasi
//! tidak perlu mencocokkan ratusan simpul SVG, dan tidak ada dependensi
//! JavaScript. Seluruh teks yang berasal dari data dilewatkan `esc` lebih dulu.

use leptos::prelude::*;

use crate::dto::{RamalTitik, TitikKurva};

/// Warna bermakna tetap: kondisi aset selalu memakai warna yang sama.
pub const W_BAIK: &str = "#10b981";
pub const W_RR: &str = "#f59e0b";
pub const W_RB: &str = "#ef4444";
pub const W_TAK: &str = "#94a3b8";
pub const W_UTAMA: &str = "#1f47f5";
pub const W_KEDUA: &str = "#0ea5e9";
pub const W_UNGU: &str = "#8b5cf6";

/// Palet kategori (delapan warna yang mudah dibedakan).
pub const PALET: &[&str] = &[
    "#1f47f5", "#0ea5e9", "#10b981", "#f59e0b", "#8b5cf6", "#ec4899", "#14b8a6", "#94a3b8",
];

/// Loloskan teks agar aman ditanam sebagai HTML/SVG.
pub fn esc(s: &str) -> String {
    let mut o = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => o.push_str("&amp;"),
            '<' => o.push_str("&lt;"),
            '>' => o.push_str("&gt;"),
            '"' => o.push_str("&quot;"),
            '\'' => o.push_str("&#39;"),
            c => o.push(c),
        }
    }
    o
}

/// Batas atas sumbu yang "bulat" (1, 2, 2,5, 5 × 10ⁿ) di atas nilai terbesar.
pub fn batas_bulat(maks: f64) -> f64 {
    if !(maks > 0.0) {
        return 1.0;
    }
    let pangkat = 10f64.powf(maks.log10().floor());
    for k in [1.0, 2.0, 2.5, 5.0, 10.0] {
        if k * pangkat >= maks * 0.999 {
            return k * pangkat;
        }
    }
    10.0 * pangkat
}

/// Satu seri pada diagram kolom bertumpuk.
#[derive(Debug, Clone, PartialEq)]
pub struct Seri {
    pub nama: String,
    pub warna: &'static str,
    pub nilai: Vec<f64>,
}

const KELAS_TEKS: &str = "fill-ink-500 dark:fill-ink-300";
const KELAS_GARIS: &str = "stroke-ink-200 dark:stroke-ink-700";

/// Diagram kolom bertumpuk dengan sumbu nilai, total di atas kolom, dan garis acuan opsional.
pub fn svg_kolom(
    label: &[String],
    seri: &[Seri],
    fmt: &dyn Fn(f64) -> String,
    acuan: Option<(f64, &str)>,
) -> String {
    const W: f64 = 720.0;
    const H: f64 = 260.0;
    const KIRI: f64 = 64.0;
    const KANAN: f64 = 12.0;
    const ATAS: f64 = 22.0;
    const BAWAH: f64 = 30.0;
    let n = label.len().max(1);
    let total: Vec<f64> = (0..n)
        .map(|i| {
            seri.iter()
                .map(|s| s.nilai.get(i).copied().unwrap_or(0.0).max(0.0))
                .sum()
        })
        .collect();
    let maks = total
        .iter()
        .copied()
        .fold(acuan.map(|a| a.0).unwrap_or(0.0), f64::max);
    let atas = batas_bulat(maks);
    let pw = W - KIRI - KANAN;
    let ph = H - ATAS - BAWAH;
    let y = |v: f64| ATAS + ph - (v / atas).clamp(0.0, 1.0) * ph;
    let lebar = pw / n as f64;
    let batang = (lebar * 0.62).min(64.0);

    let mut s = format!(
        r#"<svg viewBox="0 0 {W} {H}" class="w-full h-auto" role="img" xmlns="http://www.w3.org/2000/svg">"#
    );
    for i in 0..=4 {
        let v = atas * i as f64 / 4.0;
        let gy = y(v);
        s.push_str(&format!(
            r#"<line x1="{KIRI}" x2="{:.1}" y1="{gy:.1}" y2="{gy:.1}" class="{KELAS_GARIS}" stroke-width="1"/><text x="{:.1}" y="{:.1}" text-anchor="end" font-size="11" class="{KELAS_TEKS}">{}</text>"#,
            W - KANAN,
            KIRI - 6.0,
            gy + 4.0,
            esc(&fmt(v))
        ));
    }
    for i in 0..label.len() {
        let x = KIRI + lebar * i as f64 + (lebar - batang) / 2.0;
        let mut dasar = 0.0;
        for sr in seri {
            let v = sr.nilai.get(i).copied().unwrap_or(0.0).max(0.0);
            if v <= 0.0 {
                continue;
            }
            let y1 = y(dasar + v);
            let h = (y(dasar) - y1).max(0.0);
            s.push_str(&format!(
                r#"<rect x="{x:.1}" y="{y1:.1}" width="{batang:.1}" height="{h:.1}" fill="{}" rx="2"><title>{} — {}: {}</title></rect>"#,
                sr.warna,
                esc(&label[i]),
                esc(&sr.nama),
                esc(&fmt(v))
            ));
            dasar += v;
        }
        s.push_str(&format!(
            r#"<text x="{:.1}" y="{:.1}" text-anchor="middle" font-size="11" font-weight="600" class="fill-ink-700 dark:fill-ink-100">{}</text><text x="{:.1}" y="{:.1}" text-anchor="middle" font-size="11" class="{KELAS_TEKS}">{}</text>"#,
            x + batang / 2.0,
            y(total[i]) - 5.0,
            esc(&fmt(total[i])),
            x + batang / 2.0,
            H - 10.0,
            esc(&label[i])
        ));
    }
    if let Some((v, teks)) = acuan {
        let gy = y(v);
        s.push_str(&format!(
            r##"<line x1="{KIRI}" x2="{:.1}" y1="{gy:.1}" y2="{gy:.1}" stroke="#64748b" stroke-width="1.5" stroke-dasharray="6 4"/><text x="{:.1}" y="{:.1}" text-anchor="end" font-size="10.5" class="{KELAS_TEKS}">{}</text>"##,
            W - KANAN,
            W - KANAN - 2.0,
            gy - 5.0,
            esc(teks)
        ));
    }
    s.push_str("</svg>");
    s
}

/// Satu seri garis.
#[derive(Debug, Clone, PartialEq)]
pub struct Garis {
    pub nama: String,
    pub warna: &'static str,
    pub titik: Vec<(f64, f64)>,
    /// Garis putus-putus (dipakai untuk prakiraan dan kurva model).
    pub putus: bool,
    /// Gambar penanda titik.
    pub penanda: bool,
}

/// Diagram garis dengan pita ketidakpastian opsional; sumbu-x bernilai angka (tahun/umur).
pub fn svg_garis(
    garis: &[Garis],
    pita: &[(f64, f64, f64)],
    fmt_y: &dyn Fn(f64) -> String,
    fmt_x: &dyn Fn(f64) -> String,
    y_maks: Option<f64>,
) -> String {
    const W: f64 = 720.0;
    const H: f64 = 260.0;
    const KIRI: f64 = 64.0;
    const KANAN: f64 = 14.0;
    const ATAS: f64 = 14.0;
    const BAWAH: f64 = 28.0;
    let semua_x = garis
        .iter()
        .flat_map(|g| g.titik.iter().map(|p| p.0))
        .chain(pita.iter().map(|p| p.0));
    let (x_min, x_maks) = semua_x.fold((f64::INFINITY, f64::NEG_INFINITY), |(a, b), x| {
        (a.min(x), b.max(x))
    });
    if !x_min.is_finite() || x_maks <= x_min {
        return r#"<p class="py-6 text-center text-sm text-ink-400">Data belum cukup untuk digambar.</p>"#
            .to_string();
    }
    let maks = garis
        .iter()
        .flat_map(|g| g.titik.iter().map(|p| p.1))
        .chain(pita.iter().map(|p| p.2))
        .fold(0.0_f64, f64::max);
    let atas = y_maks.unwrap_or_else(|| batas_bulat(maks));
    let pw = W - KIRI - KANAN;
    let ph = H - ATAS - BAWAH;
    let px = |x: f64| KIRI + (x - x_min) / (x_maks - x_min) * pw;
    let py = |v: f64| ATAS + ph - (v / atas).clamp(0.0, 1.0) * ph;

    let mut s = format!(
        r#"<svg viewBox="0 0 {W} {H}" class="w-full h-auto" role="img" xmlns="http://www.w3.org/2000/svg">"#
    );
    for i in 0..=4 {
        let v = atas * i as f64 / 4.0;
        let gy = py(v);
        s.push_str(&format!(
            r#"<line x1="{KIRI}" x2="{:.1}" y1="{gy:.1}" y2="{gy:.1}" class="{KELAS_GARIS}" stroke-width="1"/><text x="{:.1}" y="{:.1}" text-anchor="end" font-size="11" class="{KELAS_TEKS}">{}</text>"#,
            W - KANAN,
            KIRI - 6.0,
            gy + 4.0,
            esc(&fmt_y(v))
        ));
    }
    // Label sumbu-x: paling banyak sembilan, pada nilai bulat.
    let rentang = x_maks - x_min;
    let langkah = [1.0, 2.0, 5.0, 10.0, 20.0]
        .into_iter()
        .find(|l| rentang / l <= 9.0)
        .unwrap_or(25.0);
    let mut x = (x_min / langkah).ceil() * langkah;
    while x <= x_maks + 1e-9 {
        s.push_str(&format!(
            r#"<text x="{:.1}" y="{:.1}" text-anchor="middle" font-size="11" class="{KELAS_TEKS}">{}</text>"#,
            px(x),
            H - 8.0,
            esc(&fmt_x(x))
        ));
        x += langkah;
    }
    if pita.len() >= 2 {
        let mut pts: Vec<String> = pita
            .iter()
            .map(|p| format!("{:.1},{:.1}", px(p.0), py(p.2)))
            .collect();
        pts.extend(
            pita.iter()
                .rev()
                .map(|p| format!("{:.1},{:.1}", px(p.0), py(p.1))),
        );
        s.push_str(&format!(
            r#"<polygon points="{}" fill="{W_UTAMA}" fill-opacity="0.12"/>"#,
            pts.join(" ")
        ));
    }
    for g in garis {
        if g.titik.is_empty() {
            continue;
        }
        let pts: Vec<String> = g
            .titik
            .iter()
            .map(|p| format!("{:.1},{:.1}", px(p.0), py(p.1)))
            .collect();
        if g.titik.len() >= 2 {
            s.push_str(&format!(
                r#"<polyline points="{}" fill="none" stroke="{}" stroke-width="2.2" stroke-linejoin="round" stroke-linecap="round"{}/>"#,
                pts.join(" "),
                g.warna,
                if g.putus { r#" stroke-dasharray="6 4""# } else { "" }
            ));
        }
        if g.penanda {
            for p in &g.titik {
                s.push_str(&format!(
                    r##"<circle cx="{:.1}" cy="{:.1}" r="3.2" fill="#fff" stroke="{}" stroke-width="2"><title>{}: {} — {}</title></circle>"##,
                    px(p.0),
                    py(p.1),
                    g.warna,
                    esc(&g.nama),
                    esc(&fmt_x(p.0)),
                    esc(&fmt_y(p.1))
                ));
            }
        }
    }
    s.push_str("</svg>");
    s
}

/// Cincin komposisi; bagian di luar tujuh terbesar digabung menjadi "Lainnya".
pub fn svg_donut(data: &[(String, f64)], label_pusat: &str, nilai_pusat: &str) -> (String, Vec<(String, f64, &'static str)>) {
    let mut urut: Vec<(String, f64)> = data.iter().filter(|d| d.1 > 0.0).cloned().collect();
    urut.sort_by(|a, b| b.1.total_cmp(&a.1));
    let total: f64 = urut.iter().map(|d| d.1).sum();
    let mut bagian: Vec<(String, f64)> = urut.iter().take(7).cloned().collect();
    let sisa: f64 = urut.iter().skip(7).map(|d| d.1).sum();
    if sisa > 0.0 {
        bagian.push(("Lainnya".to_string(), sisa));
    }
    let r = 60.0_f64;
    let keliling = 2.0 * std::f64::consts::PI * r;
    let mut s = String::from(
        r#"<svg viewBox="0 0 160 160" class="size-44" role="img" xmlns="http://www.w3.org/2000/svg"><circle cx="80" cy="80" r="60" fill="none" stroke-width="20" class="stroke-ink-100 dark:stroke-ink-800"/><g transform="rotate(-90 80 80)">"#,
    );
    let mut legenda = Vec::new();
    let mut geser = 0.0;
    for (i, (nama, v)) in bagian.iter().enumerate() {
        let porsi = if total > 0.0 { v / total } else { 0.0 };
        let panjang = porsi * keliling;
        let warna = PALET[i % PALET.len()];
        s.push_str(&format!(
            r#"<circle cx="80" cy="80" r="60" fill="none" stroke="{warna}" stroke-width="20" stroke-dasharray="{panjang:.3} {:.3}" stroke-dashoffset="{:.3}"><title>{}: {:.1}%</title></circle>"#,
            (keliling - panjang).max(0.0),
            -geser,
            esc(nama),
            porsi * 100.0
        ));
        geser += panjang;
        legenda.push((nama.clone(), porsi * 100.0, warna));
    }
    s.push_str(&format!(
        r#"</g><text x="80" y="74" text-anchor="middle" font-size="9" letter-spacing="0.6" class="{KELAS_TEKS}">{}</text><text x="80" y="92" text-anchor="middle" font-size="15" font-weight="700" class="fill-ink-900 dark:fill-ink-50">{}</text></svg>"#,
        esc(&label_pusat.to_uppercase()),
        esc(nilai_pusat)
    ));
    (s, legenda)
}

fn fmt_rupiah(v: f64) -> String {
    crate::model::format_rupiah_short(v).replace("Rp ", "")
}

fn fmt_unit(v: f64) -> String {
    crate::model::format_unit(v)
}

/// Legenda warna di bawah grafik.
#[component]
pub fn Legenda(butir: Vec<(String, &'static str)>) -> impl IntoView {
    view! {
        <ul class="mt-2 flex flex-wrap gap-x-4 gap-y-1 text-xs text-ink-500 dark:text-ink-300">
            {butir
                .into_iter()
                .map(|(nama, warna)| {
                    view! {
                        <li class="flex items-center gap-1.5">
                            <span class="size-2.5 rounded-sm" style=format!("background:{warna}") aria-hidden="true"></span>
                            {nama}
                        </li>
                    }
                })
                .collect_view()}
        </ul>
    }
}

/// Diagram kolom bertumpuk beserta legenda.
#[component]
pub fn KolomTumpuk(
    label: Vec<String>,
    seri: Vec<Seri>,
    /// "unit" atau "rupiah".
    satuan: &'static str,
    #[prop(optional)] acuan: Option<(f64, String)>,
) -> impl IntoView {
    let fmt: &dyn Fn(f64) -> String = if satuan == "rupiah" {
        &fmt_rupiah
    } else {
        &fmt_unit
    };
    let svg = svg_kolom(&label, &seri, fmt, acuan.as_ref().map(|a| (a.0, a.1.as_str())));
    let butir: Vec<(String, &'static str)> = seri.iter().map(|s| (s.nama.clone(), s.warna)).collect();
    let banyak = butir.len() > 1;
    view! {
        <div>
            <div inner_html=svg></div>
            {banyak.then(|| view! { <Legenda butir=butir/> })}
        </div>
    }
}

/// Riwayat tahunan dan prakiraannya dengan pita ketidakpastian 80%.
#[component]
pub fn GrafikRamal(
    riwayat: Vec<(i32, f64)>,
    ramal: Vec<RamalTitik>,
    satuan: &'static str,
) -> impl IntoView {
    let fmt_y: &dyn Fn(f64) -> String = if satuan == "rupiah" {
        &fmt_rupiah
    } else {
        &fmt_unit
    };
    let mut garis = vec![Garis {
        nama: "Realisasi".into(),
        warna: W_UTAMA,
        titik: riwayat.iter().map(|p| (p.0 as f64, p.1)).collect(),
        putus: false,
        penanda: true,
    }];
    let mut pita = Vec::new();
    if let (Some(akhir), false) = (riwayat.last(), ramal.is_empty()) {
        let mut titik = vec![(akhir.0 as f64, akhir.1)];
        titik.extend(ramal.iter().map(|r| (r.tahun as f64, r.nilai)));
        pita.push((akhir.0 as f64, akhir.1, akhir.1));
        pita.extend(ramal.iter().map(|r| (r.tahun as f64, r.bawah, r.atas)));
        garis.push(Garis {
            nama: "Prakiraan".into(),
            warna: W_UTAMA,
            titik,
            putus: true,
            penanda: true,
        });
    }
    let svg = svg_garis(&garis, &pita, fmt_y, &|x| format!("{x:.0}"), None);
    view! {
        <div>
            <div inner_html=svg></div>
            <Legenda butir=vec![
                ("Realisasi (tahun penuh)".to_string(), W_UTAMA),
                ("Prakiraan (garis putus) dan rentang 80%".to_string(), "#c7d2fe"),
            ]/>
        </div>
    }
}

/// Kurva kerusakan: porsi tidak baik dan rusak berat menurut umur, data vs model.
#[component]
pub fn GrafikKurva(titik: Vec<TitikKurva>) -> impl IntoView {
    let ambil = |f: fn(&TitikKurva) -> f64| -> Vec<(f64, f64)> {
        titik.iter().map(|t| (t.umur, 100.0 * f(t))).collect()
    };
    let garis = vec![
        Garis {
            nama: "Tidak baik (data)".into(),
            warna: W_RR,
            titik: ambil(|t| t.obs_tb),
            putus: false,
            penanda: true,
        },
        Garis {
            nama: "Tidak baik (model)".into(),
            warna: W_RR,
            titik: ambil(|t| t.mod_tb),
            putus: true,
            penanda: false,
        },
        Garis {
            nama: "Rusak berat (data)".into(),
            warna: W_RB,
            titik: ambil(|t| t.obs_rb),
            putus: false,
            penanda: true,
        },
        Garis {
            nama: "Rusak berat (model)".into(),
            warna: W_RB,
            titik: ambil(|t| t.mod_rb),
            putus: true,
            penanda: false,
        },
    ];
    // Skala mengikuti data agar kurva kategori berumur panjang tetap terbaca.
    let puncak = garis
        .iter()
        .flat_map(|g| g.titik.iter().map(|p| p.1))
        .fold(0.0_f64, f64::max);
    let svg = svg_garis(
        &garis,
        &[],
        &|v| format!("{v:.0}%"),
        &|x| format!("{x:.0} th"),
        Some(batas_bulat(puncak.max(5.0)).min(100.0)),
    );
    view! {
        <div>
            <div inner_html=svg></div>
            <Legenda butir=vec![
                ("Tidak baik: rusak ringan + rusak berat".to_string(), W_RR),
                ("Rusak berat".to_string(), W_RB),
                ("Garis putus = model".to_string(), W_TAK),
            ]/>
        </div>
    }
}

/// Garis tren sederhana (tanpa prakiraan).
#[component]
pub fn GrafikTren(titik: Vec<(i32, f64)>, satuan: &'static str) -> impl IntoView {
    view! { <GrafikRamal riwayat=titik ramal=vec![] satuan=satuan/> }
}

/// Cincin komposisi beserta legenda yang terbaca penuh.
#[component]
pub fn Donut(data: Vec<(String, f64)>, label_pusat: String, nilai_pusat: String) -> impl IntoView {
    let (svg, legenda) = svg_donut(&data, &label_pusat, &nilai_pusat);
    view! {
        <div class="flex flex-col items-center gap-4">
            <div class="shrink-0" inner_html=svg></div>
            <ul class="grid w-full min-w-0 gap-x-4 gap-y-1.5 sm:grid-cols-2 xl:grid-cols-1">
                {legenda
                    .into_iter()
                    .map(|(nama, pct, warna)| {
                        view! {
                            <li class="flex items-center gap-2 text-xs">
                                <span class="size-2.5 shrink-0 rounded-sm" style=format!("background:{warna}") aria-hidden="true"></span>
                                <span class="min-w-0 flex-1 truncate" title=nama.clone()>{nama.clone()}</span>
                                <span class="shrink-0 tabular-nums font-medium">{crate::model::format_persen(pct)}</span>
                            </li>
                        }
                    })
                    .collect_view()}
            </ul>
        </div>
    }
}

/// Daftar batang mendatar; panjang batang sebanding dengan nilai yang ditampilkan.
#[component]
pub fn BarList(
    /// (nama, nilai, teks nilai, tautan opsional).
    data: Vec<(String, f64, String, Option<String>)>,
    #[prop(default = W_UTAMA)] warna: &'static str,
    /// Nilai yang mengisi batang penuh; bawaan = nilai terbesar pada data.
    #[prop(optional)] skala: Option<f64>,
) -> impl IntoView {
    let maks = skala.unwrap_or_else(|| data.iter().map(|d| d.1).fold(0.0_f64, f64::max));
    view! {
        <ul class="space-y-2.5">
            {data
                .into_iter()
                .map(|(nama, nilai, teks, tautan)| {
                    let pct = if maks > 0.0 { (nilai / maks * 100.0).clamp(0.0, 100.0) } else { 0.0 };
                    let judul = nama.clone();
                    view! {
                        <li>
                            <div class="flex items-baseline justify-between gap-3 text-xs">
                                {match tautan {
                                    Some(href) => view! {
                                        <a class="min-w-0 flex-1 truncate hover:text-bmn-600 hover:underline" href=href title=judul>{nama}</a>
                                    }.into_any(),
                                    None => view! {
                                        <span class="min-w-0 flex-1 truncate" title=judul>{nama}</span>
                                    }.into_any(),
                                }}
                                <span class="shrink-0 tabular-nums font-medium">{teks}</span>
                            </div>
                            <div class="mt-1 h-2 w-full overflow-hidden rounded-full bg-ink-100 dark:bg-ink-800">
                                <div class="h-full rounded-full" style=format!("width:{pct:.1}%;background:{warna}")></div>
                            </div>
                        </li>
                    }
                })
                .collect_view()}
        </ul>
    }
}

/// Batang 100% sebaran kondisi (baik / rusak ringan / rusak berat / tidak diisi).
#[component]
pub fn BilahKondisi(
    baik: i64,
    rr: i64,
    rb: i64,
    tak: i64,
    #[prop(default = false)] tipis: bool,
) -> impl IntoView {
    let total = (baik + rr + rb + tak).max(1) as f64;
    let bagian = [
        ("Baik", baik, W_BAIK),
        ("Rusak ringan", rr, W_RR),
        ("Rusak berat", rb, W_RB),
        ("Tidak diisi", tak, W_TAK),
    ];
    let judul = bagian
        .iter()
        .map(|(n, v, _)| format!("{n}: {}", crate::model::format_number(*v)))
        .collect::<Vec<_>>()
        .join(" · ");
    view! {
        <div
            class=format!("flex w-full overflow-hidden rounded-full bg-ink-100 dark:bg-ink-800 {}", if tipis { "h-2" } else { "h-3" })
            title=judul
            role="img"
            aria-label="Sebaran kondisi aset"
        >
            {bagian
                .into_iter()
                .filter(|(_, v, _)| *v > 0)
                .map(|(_, v, warna)| {
                    view! { <div style=format!("width:{:.2}%;background:{warna}", v as f64 / total * 100.0)></div> }
                })
                .collect_view()}
        </div>
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn teks_data_diloloskan() {
        assert_eq!(esc("<script>\"a\" & 'b'"), "&lt;script&gt;&quot;a&quot; &amp; &#39;b&#39;");
        let svg = svg_kolom(
            &["<b>2027</b>".to_string()],
            &[Seri {
                nama: "A&B".into(),
                warna: W_UTAMA,
                nilai: vec![5.0],
            }],
            &|v| format!("{v:.0}"),
            Some((3.0, "acuan <x>")),
        );
        assert!(!svg.contains("<b>") && !svg.contains("<x>"));
        assert!(svg.contains("&lt;b&gt;2027") && svg.contains("A&amp;B"));
    }

    #[test]
    fn batas_sumbu_bulat() {
        assert_eq!(batas_bulat(0.0), 1.0);
        assert_eq!(batas_bulat(7.0), 10.0);
        assert_eq!(batas_bulat(18.0), 20.0);
        assert_eq!(batas_bulat(230.0), 250.0);
        assert_eq!(batas_bulat(1000.0), 1000.0);
        assert_eq!(batas_bulat(f64::NAN), 1.0);
        assert!(batas_bulat(3.2e11) >= 3.2e11);
    }

    #[test]
    fn kolom_setinggi_nilainya() {
        // Kolom bernilai setengah batas sumbu harus setengah tinggi bidang gambar.
        let svg = svg_kolom(
            &["a".to_string(), "b".to_string()],
            &[Seri {
                nama: "x".into(),
                warna: W_UTAMA,
                nilai: vec![10.0, 5.0],
            }],
            &|v| format!("{v:.0}"),
            None,
        );
        let tinggi: Vec<f64> = svg
            .split("<rect ")
            .skip(1)
            .map(|r| {
                let h = r.split("height=\"").nth(1).unwrap();
                h[..h.find('"').unwrap()].parse().unwrap()
            })
            .collect();
        assert_eq!(tinggi.len(), 2);
        assert!((tinggi[0] - 2.0 * tinggi[1]).abs() < 0.2, "{tinggi:?}");
    }

    #[test]
    fn kolom_tanpa_label_tidak_panik() {
        let svg = svg_kolom(&[], &[], &|v| format!("{v:.0}"), None);
        assert!(svg.starts_with("<svg") && svg.ends_with("</svg>"));
        assert!(!svg.contains("<rect"));
    }

    #[test]
    fn donut_menggabungkan_sisa() {
        let data: Vec<(String, f64)> = (0..12).map(|i| (format!("K{i}"), 12.0 - i as f64)).collect();
        let (svg, legenda) = svg_donut(&data, "Total", "Rp 1 T");
        assert_eq!(legenda.len(), 8, "tujuh terbesar + Lainnya");
        assert_eq!(legenda[7].0, "Lainnya");
        let jumlah: f64 = legenda.iter().map(|l| l.1).sum();
        assert!((jumlah - 100.0).abs() < 1e-6);
        assert!(svg.contains("TOTAL"));
    }

    #[test]
    fn garis_tanpa_data_tidak_panik() {
        let svg = svg_garis(&[], &[], &|v| v.to_string(), &|x| x.to_string(), None);
        assert!(svg.contains("belum cukup"));
        let satu = svg_garis(
            &[Garis {
                nama: "a".into(),
                warna: W_UTAMA,
                titik: vec![(2020.0, 5.0)],
                putus: false,
                penanda: true,
            }],
            &[],
            &|v| v.to_string(),
            &|x| x.to_string(),
            None,
        );
        assert!(satu.contains("belum cukup"), "satu titik belum membentuk garis");
    }
}
