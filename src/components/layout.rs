use leptos::prelude::*;
use leptos_router::components::A;
use leptos_router::hooks::use_location;

/// Skrip kecil yang dijalankan sebelum render untuk menghindari kedip tema.
#[component]
pub fn ThemeInit() -> impl IntoView {
    view! {
        <script>
            {r#"(function(){try{var t=localStorage.getItem('bmn-theme');if(t==='dark'||(!t&&window.matchMedia('(prefers-color-scheme: dark)').matches)){document.documentElement.classList.add('dark');}}catch(e){}})();"#}
        </script>
    }
}

#[derive(Clone, Copy, PartialEq)]
struct NavItem {
    href: &'static str,
    label: &'static str,
    icon: &'static str,
}

const fn item(href: &'static str, label: &'static str, icon: &'static str) -> NavItem {
    NavItem { href, label, icon }
}

/// Navigasi dikelompokkan menurut cara pimpinan membaca: ringkasan dan
/// pandangan ke depan lebih dulu, rincian sesudahnya, pengelolaan sistem terakhir.
const NAV: &[(&str, &[NavItem])] = &[
    (
        "Pimpinan",
        &[
            item("/", "Ringkasan Eksekutif", "📊"),
            item("/kondisi", "Kondisi & Risiko", "🩺"),
            item("/prediksi", "Prediksi 5 Tahun", "🔮"),
            item("/kebutuhan", "Analisis Kebutuhan", "🧮"),
            item("/prakiraan", "Forecast Perolehan", "📈"),
            item("/rekomendasi", "Rekomendasi", "✅"),
        ],
    ),
    (
        "Rincian",
        &[
            item("/satker", "Satuan Kerja", "🏛️"),
            item("/wilayah", "Wilayah", "🗺️"),
            item("/kategori", "Kategori Aset", "🗂️"),
            item("/data", "Data Aset", "🔎"),
        ],
    ),
    (
        "Tata Kelola",
        &[
            item("/kualitas", "Mutu Data", "🧪"),
            item("/laporan", "Laporan PDF", "🧾"),
            item("/metodologi", "Metodologi", "📐"),
        ],
    ),
    (
        "Sistem",
        &[
            item("/sinkronisasi", "Sinkronisasi", "🔄"),
            item("/pengaturan", "Pengaturan", "⚙️"),
        ],
    ),
];

/// Kerangka aplikasi: sidebar desktop, drawer mobile, dan bilah atas.
#[component]
pub fn AppShell(children: Children) -> impl IntoView {
    let location = use_location();
    let (open, set_open) = signal(false);
    let pathname = move || location.pathname.get();

    let is_active = Callback::new(move |href: &'static str| {
        let p = pathname();
        if href == "/" {
            p == "/"
        } else {
            p == href || p.starts_with(&format!("{href}/"))
        }
    });

    Effect::new(move |_| {
        let _ = pathname();
        set_open.set(false);
    });

    view! {
        <div class="min-h-dvh lg:grid lg:grid-cols-[17rem_1fr]">
            // Sidebar (desktop)
            <aside class="no-print sticky top-0 hidden h-dvh flex-col border-r border-ink-200 bg-white lg:flex dark:border-ink-800 dark:bg-ink-900">
                <Brand/>
                <Menu is_active=is_active/>
                <SidebarFooter/>
            </aside>

            <div class="flex min-w-0 flex-col">
                <header class="no-print sticky top-0 z-30 flex items-center gap-3 border-b border-ink-200 bg-white/85 px-3 py-2.5 backdrop-blur-md lg:px-6 dark:border-ink-800 dark:bg-ink-900/85">
                    <button
                        type="button"
                        class="btn-ghost px-2.5 py-2 lg:hidden"
                        aria-label="Buka menu navigasi"
                        aria-expanded=move || open.get().to_string()
                        on:click=move |_| set_open.update(|o| *o = !*o)
                    >
                        <span class="text-lg" aria-hidden="true">"☰"</span>
                    </button>
                    <div class="min-w-0 flex-1">
                        <h1 class="truncate text-sm font-semibold sm:text-base">{move || title_for(&pathname())}</h1>
                        <p class="hidden truncate text-xs text-ink-400 sm:block">
                            "Barang Milik Negara Kejaksaan RI · sumber data SLDK SIMAN v2"
                        </p>
                    </div>
                    <ThemeToggle/>
                </header>

                <main class="mx-auto w-full max-w-[1600px] flex-1 px-3 py-4 sm:px-4 lg:px-6 lg:py-6">
                    {children()}
                </main>

                <footer class="border-t border-ink-200 px-4 py-4 text-xs text-ink-400 dark:border-ink-800">
                    <p>
                        "Data bersumber dari Web Service SLDK SIMAN v2 (DJKN, Kementerian Keuangan RI). "
                        <span class="hidden sm:inline">"Klasifikasi data: Terbatas — gunakan sesuai kewenangan."</span>
                    </p>
                </footer>
            </div>
        </div>

        // Drawer (mobile)
        <Show when=move || open.get()>
            <div class="fixed inset-0 z-40 lg:hidden" role="dialog" aria-modal="true" aria-label="Menu navigasi">
                <div
                    class="absolute inset-0 bg-ink-950/50 backdrop-blur-sm"
                    on:click=move |_| set_open.set(false)
                ></div>
                <div class="absolute inset-y-0 left-0 flex w-72 max-w-[85%] flex-col bg-white shadow-xl dark:bg-ink-900">
                    <Brand/>
                    <Menu is_active=is_active/>
                    <SidebarFooter/>
                </div>
            </div>
        </Show>
    }
}

/// Daftar menu berkelompok; dipakai sidebar desktop dan laci seluler.
#[component]
fn Menu(is_active: Callback<&'static str, bool>) -> impl IntoView {
    view! {
        <nav class="flex-1 space-y-4 overflow-y-auto p-3" aria-label="Navigasi utama">
            {NAV
                .iter()
                .map(|(kelompok, butir)| {
                    view! {
                        <div>
                            <p class="px-3 pb-1 text-[0.65rem] font-semibold uppercase tracking-wider text-ink-400">
                                {*kelompok}
                            </p>
                            <ul class="space-y-0.5">
                                {butir
                                    .iter()
                                    .map(|it| {
                                        let href = it.href;
                                        view! {
                                            <li>
                                                <A
                                                    href=href
                                                    attr:class=move || {
                                                        format!(
                                                            "flex items-center gap-3 rounded-lg px-3 py-2 text-sm transition-colors {}",
                                                            if is_active.run(href) {
                                                                "bg-bmn-50 text-bmn-700 font-semibold dark:bg-bmn-900/40 dark:text-bmn-200"
                                                            } else {
                                                                "text-ink-600 hover:bg-ink-50 dark:text-ink-300 dark:hover:bg-ink-800"
                                                            },
                                                        )
                                                    }
                                                >
                                                    <span class="w-5 text-center" aria-hidden="true">{it.icon}</span>
                                                    <span class="truncate">{it.label}</span>
                                                </A>
                                            </li>
                                        }
                                    })
                                    .collect_view()}
                            </ul>
                        </div>
                    }
                })
                .collect_view()}
        </nav>
    }
}

#[component]
fn Brand() -> impl IntoView {
    view! {
        <div class="flex items-center gap-3 border-b border-ink-200 px-4 py-4 dark:border-ink-800">
            <div class="grid size-10 shrink-0 place-items-center rounded-xl bg-bmn-600 text-lg font-bold text-white shadow-sm">
                "BMN"
            </div>
            <div class="min-w-0">
                <p class="truncate text-sm font-bold leading-tight">"Dasbor BMN"</p>
                <p class="truncate text-xs text-ink-400">"Kejaksaan Republik Indonesia"</p>
            </div>
        </div>
    }
}

#[component]
fn SidebarFooter() -> impl IntoView {
    view! {
        <div class="border-t border-ink-200 p-4 text-xs text-ink-400 dark:border-ink-800">
            <p class="font-medium text-ink-500 dark:text-ink-300">"SLDK SIMAN v2"</p>
            <p>"Sumber: DJKN · Kemenkeu RI"</p>
            <p class="mt-1">"Klasifikasi: Terbatas"</p>
            <form method="post" action="/api/logout" class="mt-3">
                <button type="submit" class="btn-ghost w-full justify-center px-2.5 py-2">
                    <span aria-hidden="true">"🚪"</span>
                    <span class="ml-1">"Keluar"</span>
                </button>
            </form>
        </div>
    }
}

#[component]
fn ThemeToggle() -> impl IntoView {
    let (dark, set_dark) = signal(false);
    Effect::new(move |_| {
        #[cfg(feature = "hydrate")]
        {
            let is_dark = web_sys_dark();
            set_dark.set(is_dark);
        }
    });
    let toggle = move |_| {
        let next = !dark.get_untracked();
        set_dark.set(next);
        #[cfg(feature = "hydrate")]
        {
            set_web_theme(next);
        }
    };
    view! {
        <button
            type="button"
            class="btn-ghost px-2.5 py-2"
            aria-label="Ubah tema terang/gelap"
            title="Ubah tema"
            on:click=toggle
        >
            <span aria-hidden="true">{move || if dark.get() { "🌙" } else { "☀️" }}</span>
        </button>
    }
}

#[cfg(feature = "hydrate")]
fn web_sys_dark() -> bool {
    web_sys::window()
        .and_then(|w| w.document())
        .and_then(|d| d.document_element())
        .map(|e| e.class_list().contains("dark"))
        .unwrap_or(false)
}

#[cfg(feature = "hydrate")]
fn set_web_theme(dark: bool) {
    if let Some(doc) = web_sys::window().and_then(|w| w.document()) {
        if let Some(el) = doc.document_element() {
            let _ = el.class_list().toggle_with_force("dark", dark);
        }
        if let Some(storage) = web_sys::window().and_then(|w| w.local_storage().ok().flatten()) {
            let _ = storage.set_item("bmn-theme", if dark { "dark" } else { "light" });
        }
    }
}

fn title_for(path: &str) -> &'static str {
    if path.starts_with("/satker/") {
        return "Rincian Satuan Kerja";
    }
    NAV.iter()
        .flat_map(|(_, butir)| butir.iter())
        .find(|it| it.href == path)
        .map(|it| it.label)
        .unwrap_or("Dasbor BMN")
}
