//! Data contoh (mode demo) agar dasbor tetap dapat dievaluasi tanpa kredensial.
//!
//! Data bersifat **ilustratif**: susunan satuan kerja meniru struktur Kejaksaan
//! (Kejaksaan Agung, Kejaksaan Tinggi, Kejaksaan Negeri), tetapi seluruh angka,
//! kode satuan kerja, dan kondisi aset dibangkitkan secara deterministik dan
//! tidak mencerminkan keadaan sebenarnya. Kondisi aset dibangkitkan dari proses
//! kerusakan dua tahap (baik → rusak ringan → rusak berat) yang bergantung pada
//! umur dan mutu pemeliharaan tiap satuan kerja, supaya analitik prediksi
//! memperlihatkan pola yang masuk akal.

#![cfg(feature = "ssr")]

use crate::catalog;
use serde_json::{json, Value};

/// Provinsi beserta kota/kabupaten kedudukan Kejaksaan Negeri contoh.
const WILAYAH: &[(&str, &[&str])] = &[
    ("ACEH", &["BANDA ACEH", "LHOKSEUMAWE", "LANGSA", "ACEH BESAR"]),
    (
        "SUMATERA UTARA",
        &["MEDAN", "BINJAI", "DELI SERDANG", "PEMATANGSIANTAR", "LABUHANBATU", "ASAHAN"],
    ),
    ("SUMATERA BARAT", &["PADANG", "BUKITTINGGI", "PAYAKUMBUH", "PESISIR SELATAN"]),
    ("RIAU", &["PEKANBARU", "DUMAI", "KAMPAR", "BENGKALIS"]),
    ("KEPULAUAN RIAU", &["BATAM", "TANJUNGPINANG", "KARIMUN"]),
    ("JAMBI", &["JAMBI", "MUARO JAMBI", "BUNGO"]),
    (
        "SUMATERA SELATAN",
        &["PALEMBANG", "LUBUKLINGGAU", "PRABUMULIH", "OGAN KOMERING ILIR", "MUARA ENIM"],
    ),
    ("KEPULAUAN BANGKA BELITUNG", &["PANGKALPINANG", "BANGKA", "BELITUNG"]),
    ("BENGKULU", &["BENGKULU", "REJANG LEBONG", "BENGKULU UTARA"]),
    ("LAMPUNG", &["BANDAR LAMPUNG", "METRO", "LAMPUNG SELATAN", "LAMPUNG TENGAH"]),
    (
        "DKI JAKARTA",
        &["JAKARTA PUSAT", "JAKARTA SELATAN", "JAKARTA TIMUR", "JAKARTA BARAT", "JAKARTA UTARA"],
    ),
    (
        "JAWA BARAT",
        &["BANDUNG", "BOGOR", "BEKASI", "DEPOK", "CIREBON", "TASIKMALAYA", "KARAWANG"],
    ),
    ("BANTEN", &["SERANG", "TANGERANG", "CILEGON", "LEBAK"]),
    (
        "JAWA TENGAH",
        &["SEMARANG", "SURAKARTA", "MAGELANG", "PEKALONGAN", "TEGAL", "BANYUMAS", "KUDUS"],
    ),
    ("DI YOGYAKARTA", &["YOGYAKARTA", "SLEMAN", "BANTUL"]),
    (
        "JAWA TIMUR",
        &["SURABAYA", "MALANG", "KEDIRI", "MADIUN", "JEMBER", "SIDOARJO", "BANYUWANGI"],
    ),
    ("BALI", &["DENPASAR", "BADUNG", "GIANYAR", "BULELENG"]),
    ("NUSA TENGGARA BARAT", &["MATARAM", "LOMBOK TENGAH", "SUMBAWA", "BIMA"]),
    ("NUSA TENGGARA TIMUR", &["KUPANG", "ENDE", "SIKKA", "MANGGARAI"]),
    ("KALIMANTAN BARAT", &["PONTIANAK", "SINGKAWANG", "KETAPANG", "SINTANG"]),
    ("KALIMANTAN TENGAH", &["PALANGKA RAYA", "KOTAWARINGIN TIMUR", "KAPUAS"]),
    ("KALIMANTAN SELATAN", &["BANJARMASIN", "BANJARBARU", "KOTABARU"]),
    ("KALIMANTAN TIMUR", &["SAMARINDA", "BALIKPAPAN", "BONTANG", "KUTAI KARTANEGARA"]),
    ("KALIMANTAN UTARA", &["TARAKAN", "BULUNGAN", "NUNUKAN"]),
    ("SULAWESI UTARA", &["MANADO", "BITUNG", "TOMOHON", "KOTAMOBAGU"]),
    ("GORONTALO", &["GORONTALO", "LIMBOTO", "BONE BOLANGO"]),
    ("SULAWESI TENGAH", &["PALU", "POSO", "BANGGAI"]),
    ("SULAWESI BARAT", &["MAMUJU", "MAJENE", "POLEWALI MANDAR"]),
    (
        "SULAWESI SELATAN",
        &["MAKASSAR", "PAREPARE", "PALOPO", "GOWA", "BONE", "BULUKUMBA"],
    ),
    ("SULAWESI TENGGARA", &["KENDARI", "BAUBAU", "KOLAKA"]),
    ("MALUKU", &["AMBON", "MALUKU TENGAH", "TUAL"]),
    ("MALUKU UTARA", &["TERNATE", "TIDORE KEPULAUAN", "HALMAHERA SELATAN"]),
    ("PAPUA", &["JAYAPURA", "BIAK NUMFOR", "KEPULAUAN YAPEN"]),
    ("PAPUA BARAT", &["MANOKWARI", "FAKFAK", "TELUK BINTUNI"]),
    ("PAPUA BARAT DAYA", &["SORONG", "RAJA AMPAT"]),
    ("PAPUA SELATAN", &["MERAUKE", "MAPPI"]),
    ("PAPUA TENGAH", &["NABIRE", "MIMIKA"]),
    ("PAPUA PEGUNUNGAN", &["JAYAWIJAYA"]),
];

/// Satu jenis barang contoh dalam suatu kategori.
struct Jenis {
    kelompok: &'static str,
    kd: &'static str,
    nama: &'static [&'static str],
    merk: &'static [&'static str],
    /// Rentang harga perolehan (juta rupiah).
    harga: (f64, f64),
    /// Masa manfaat (tahun); 0 bila tidak disusutkan.
    masa: f64,
    bobot: u32,
}

/// Pola pembangkitan satu kategori.
struct Pola {
    /// Rata-rata unit per Kejaksaan Negeri.
    per_satker: f64,
    /// Umur tertua yang masih mungkin tercatat (tahun).
    umur_maks: i32,
    /// Parameter kerusakan: skala dan bentuk Weibull, laju RR→RB per tahun.
    rusak: Option<(f64, f64, f64)>,
    /// Laju penghapusan aset rusak berat per tahun.
    hapus: f64,
    jenis: &'static [Jenis],
}

const TANPA_MERK: &[&str] = &["-"];

fn pola(tabel: &str) -> Pola {
    match tabel {
        "SIMAN2_M_ASET_TANAH" => Pola {
            per_satker: 3.0,
            umur_maks: 55,
            rusak: None,
            hapus: 0.0,
            jenis: &[
                Jenis { kelompok: "Tanah Bangunan Kantor Pemerintah", kd: "2010104001", nama: &["Tanah Bangunan Kantor Pemerintah"], merk: TANPA_MERK, harga: (900.0, 42_000.0), masa: 0.0, bobot: 5 },
                Jenis { kelompok: "Tanah Bangunan Rumah Negara Golongan II", kd: "2010101002", nama: &["Tanah Bangunan Rumah Negara Golongan II"], merk: TANPA_MERK, harga: (250.0, 6_000.0), masa: 0.0, bobot: 4 },
                Jenis { kelompok: "Tanah Bangunan Gudang", kd: "2010103001", nama: &["Tanah Bangunan Gudang Barang Bukti"], merk: TANPA_MERK, harga: (200.0, 3_500.0), masa: 0.0, bobot: 1 },
            ],
        },
        "SIMAN2_M_ASET_GEDUNG_BANGUNAN" => Pola {
            per_satker: 4.0,
            umur_maks: 50,
            rusak: Some((62.0, 2.6, 0.09)),
            hapus: 0.12,
            jenis: &[
                Jenis { kelompok: "Bangunan Gedung Kantor Permanen", kd: "4010101001", nama: &["Bangunan Gedung Kantor Permanen"], merk: TANPA_MERK, harga: (1_800.0, 38_000.0), masa: 50.0, bobot: 4 },
                Jenis { kelompok: "Bangunan Gudang Tertutup Permanen", kd: "4010104001", nama: &["Gedung Barang Bukti", "Bangunan Gudang Arsip"], merk: TANPA_MERK, harga: (250.0, 2_800.0), masa: 50.0, bobot: 2 },
                Jenis { kelompok: "Bangunan Gedung Garasi/Pool Permanen", kd: "4010117001", nama: &["Bangunan Garasi Kendaraan Dinas", "Pos Jaga Permanen"], merk: TANPA_MERK, harga: (90.0, 900.0), masa: 50.0, bobot: 3 },
            ],
        },
        "SIMAN2_M_ASET_RUMAH" => Pola {
            per_satker: 6.0,
            umur_maks: 48,
            rusak: Some((47.0, 2.4, 0.11)),
            hapus: 0.12,
            jenis: &[
                Jenis { kelompok: "Rumah Negara Golongan II Tipe C Permanen", kd: "4010202007", nama: &["Rumah Negara Golongan II Tipe C"], merk: TANPA_MERK, harga: (220.0, 1_100.0), masa: 50.0, bobot: 3 },
                Jenis { kelompok: "Rumah Negara Golongan II Tipe D Permanen", kd: "4010202010", nama: &["Rumah Negara Golongan II Tipe D"], merk: TANPA_MERK, harga: (160.0, 750.0), masa: 50.0, bobot: 4 },
                Jenis { kelompok: "Mess/Wisma/Asrama Permanen", kd: "4010203001", nama: &["Mess Pegawai", "Asrama Jaksa"], merk: TANPA_MERK, harga: (400.0, 3_200.0), masa: 50.0, bobot: 1 },
            ],
        },
        "SIMAN2_M_ASET_ANGKUTAN_BERMOTOR" => Pola {
            per_satker: 12.0,
            umur_maks: 22,
            rusak: Some((14.5, 2.6, 0.26)),
            hapus: 0.55,
            jenis: &[
                Jenis { kelompok: "Mini Bus (Penumpang 14 Orang Kebawah)", kd: "3020102003", nama: &["Mini Bus Operasional", "Kendaraan Dinas Operasional"], merk: &["Toyota Innova", "Toyota Avanza", "Mitsubishi Xpander", "Suzuki Ertiga", "Daihatsu Terios"], harga: (190.0, 520.0), masa: 7.0, bobot: 4 },
                Jenis { kelompok: "Kendaraan Tahanan", kd: "3020105011", nama: &["Mobil Tahanan"], merk: &["Isuzu Elf", "Mitsubishi Colt Diesel", "Toyota Hiace"], harga: (380.0, 760.0), masa: 7.0, bobot: 2 },
                Jenis { kelompok: "Sedan", kd: "3020101001", nama: &["Kendaraan Dinas Jabatan"], merk: &["Toyota Camry", "Toyota Altis", "Honda Civic"], harga: (420.0, 890.0), masa: 7.0, bobot: 1 },
                Jenis { kelompok: "Sepeda Motor", kd: "3020104001", nama: &["Sepeda Motor Operasional"], merk: &["Honda Vario", "Honda Supra X", "Yamaha NMAX", "Honda Beat"], harga: (16.0, 38.0), masa: 7.0, bobot: 6 },
            ],
        },
        "SIMAN2_M_ASET_ALAT_BESAR" => Pola {
            per_satker: 0.35,
            umur_maks: 25,
            rusak: Some((16.0, 2.4, 0.22)),
            hapus: 0.35,
            jenis: &[
                Jenis { kelompok: "Electric Generating Set", kd: "3010303001", nama: &["Genset 100 KVA", "Genset 250 KVA"], merk: &["Perkins", "Cummins", "Caterpillar"], harga: (180.0, 950.0), masa: 10.0, bobot: 4 },
                Jenis { kelompok: "Pompa", kd: "3010304001", nama: &["Pompa Pemadam Kebakaran"], merk: &["Ebara", "Grundfos"], harga: (60.0, 240.0), masa: 10.0, bobot: 1 },
            ],
        },
        "SIMAN2_M_ASET_ALAT_PERSENJATAAN" => Pola {
            per_satker: 4.0,
            umur_maks: 30,
            rusak: Some((30.0, 2.2, 0.14)),
            hapus: 0.25,
            jenis: &[
                Jenis { kelompok: "Senjata Genggam", kd: "3090101001", nama: &["Pistol Dinas"], merk: &["Pindad G2", "Pindad P3A"], harga: (12.0, 32.0), masa: 10.0, bobot: 3 },
                Jenis { kelompok: "Alat Keamanan dan Perlindungan", kd: "3090402001", nama: &["Borgol", "Rompi Pengaman", "Metal Detector"], merk: &["Pindad", "Garrett", "-"], harga: (0.6, 9.0), masa: 5.0, bobot: 4 },
            ],
        },
        "SIMAN2_M_ASET_JALAN_DAN_JEMBATAN" => Pola {
            per_satker: 0.6,
            umur_maks: 30,
            rusak: Some((18.0, 2.2, 0.18)),
            hapus: 0.20,
            jenis: &[
                Jenis { kelompok: "Jalan Khusus Kompleks", kd: "5010107001", nama: &["Jalan Lingkungan Kantor", "Pelataran Parkir"], merk: TANPA_MERK, harga: (80.0, 900.0), masa: 10.0, bobot: 1 },
            ],
        },
        "SIMAN2_M_ASET_BANGUNAN_AIR" => Pola {
            per_satker: 0.3,
            umur_maks: 35,
            rusak: Some((32.0, 2.2, 0.11)),
            hapus: 0.15,
            jenis: &[
                Jenis { kelompok: "Bangunan Pembuang Pengaman Sungai", kd: "5020403001", nama: &["Saluran Drainase Kompleks", "Talud Pengaman"], merk: TANPA_MERK, harga: (60.0, 650.0), masa: 30.0, bobot: 1 },
            ],
        },
        "SIMAN2_M_ASET_INSTALASI_JARINGAN" => Pola {
            per_satker: 3.0,
            umur_maks: 30,
            rusak: Some((24.0, 2.2, 0.16)),
            hapus: 0.28,
            jenis: &[
                Jenis { kelompok: "Instalasi Gardu Listrik Distribusi", kd: "5030601001", nama: &["Instalasi Listrik Gedung Kantor"], merk: TANPA_MERK, harga: (45.0, 520.0), masa: 20.0, bobot: 3 },
                Jenis { kelompok: "Jaringan Telepon Diatas Tanah", kd: "5040301001", nama: &["Jaringan LAN Kantor", "Jaringan Telepon/PABX"], merk: TANPA_MERK, harga: (25.0, 380.0), masa: 20.0, bobot: 3 },
                Jenis { kelompok: "Instalasi Air Bersih/Air Baku", kd: "5030101001", nama: &["Instalasi Air Bersih"], merk: TANPA_MERK, harga: (20.0, 190.0), masa: 20.0, bobot: 2 },
            ],
        },
        "SIMAN2_M_ASET_KHUSUS_TIK" => Pola {
            per_satker: 22.0,
            umur_maks: 16,
            rusak: Some((8.6, 2.4, 0.36)),
            hapus: 0.70,
            jenis: &[
                Jenis { kelompok: "P.C Unit", kd: "3100102001", nama: &["PC Unit"], merk: &["Lenovo ThinkCentre", "HP ProDesk", "Dell OptiPlex", "Acer Veriton"], harga: (9.0, 19.0), masa: 4.0, bobot: 8 },
                Jenis { kelompok: "Lap Top", kd: "3100102002", nama: &["Laptop"], merk: &["Lenovo ThinkPad", "HP ProBook", "Dell Latitude", "Asus ExpertBook"], harga: (11.0, 26.0), masa: 4.0, bobot: 6 },
                Jenis { kelompok: "Printer (Peralatan Personal Komputer)", kd: "3100203003", nama: &["Printer"], merk: &["Epson L-Series", "HP LaserJet", "Canon Pixma"], harga: (2.5, 9.0), masa: 4.0, bobot: 5 },
                Jenis { kelompok: "Scanner (Peralatan Personal Komputer)", kd: "3100203004", nama: &["Scanner Dokumen"], merk: &["Fujitsu ScanSnap", "Epson DS", "Canon DR"], harga: (4.0, 22.0), masa: 4.0, bobot: 2 },
                Jenis { kelompok: "Server", kd: "3100204001", nama: &["Server Rak"], merk: &["Dell PowerEdge", "HPE ProLiant", "Lenovo ThinkSystem"], harga: (70.0, 380.0), masa: 4.0, bobot: 1 },
            ],
        },
        "SIMAN2_M_ASET_NON_TIK" => Pola {
            per_satker: 44.0,
            umur_maks: 24,
            rusak: Some((14.0, 2.0, 0.22)),
            hapus: 0.55,
            jenis: &[
                Jenis { kelompok: "Meja Kerja Kayu", kd: "3050201002", nama: &["Meja Kerja Pejabat", "Meja Kerja Staf"], merk: &["Olympic", "Ligna", "Lokal"], harga: (1.4, 6.5), masa: 5.0, bobot: 8 },
                Jenis { kelompok: "Kursi Besi/Metal", kd: "3050201003", nama: &["Kursi Kerja", "Kursi Tamu", "Kursi Rapat"], merk: &["Chitose", "Indachi", "Brother"], harga: (0.7, 4.5), masa: 5.0, bobot: 10 },
                Jenis { kelompok: "Lemari Besi/Metal", kd: "3050104001", nama: &["Lemari Arsip", "Filing Cabinet", "Brankas"], merk: &["Lion", "Brother", "Daiko"], harga: (2.0, 14.0), masa: 5.0, bobot: 5 },
                Jenis { kelompok: "A.C. Split", kd: "3050204004", nama: &["AC Split 1 PK", "AC Split 2 PK"], merk: &["Daikin", "Panasonic", "Sharp", "LG"], harga: (4.5, 13.0), masa: 5.0, bobot: 6 },
                Jenis { kelompok: "Televisi", kd: "3050206002", nama: &["Televisi LED"], merk: &["Samsung", "LG", "Sharp"], harga: (3.0, 16.0), masa: 5.0, bobot: 2 },
                Jenis { kelompok: "Camera Video", kd: "3060102128", nama: &["CCTV Set", "Kamera Sidang"], merk: &["Hikvision", "Dahua", "Sony"], harga: (6.0, 85.0), masa: 5.0, bobot: 2 },
                Jenis { kelompok: "Sound System", kd: "3050206046", nama: &["Sound System Ruang Rapat"], merk: &["TOA", "Yamaha", "BMB"], harga: (5.0, 60.0), masa: 5.0, bobot: 1 },
            ],
        },
        "SIMAN2_M_ASET_ASET_TAK_BERWUJUD" => Pola {
            per_satker: 1.5,
            umur_maks: 14,
            rusak: None,
            hapus: 0.0,
            jenis: &[
                Jenis { kelompok: "Software Komputer", kd: "8010101001", nama: &["Aplikasi Manajemen Perkara", "Aplikasi Persuratan", "Lisensi Perangkat Lunak"], merk: TANPA_MERK, harga: (15.0, 900.0), masa: 4.0, bobot: 1 },
            ],
        },
        "SIMAN2_M_ASET_ASET_TETAP_LAINNYA" => Pola {
            per_satker: 3.0,
            umur_maks: 35,
            rusak: None,
            hapus: 0.0,
            jenis: &[
                Jenis { kelompok: "Monografi", kd: "6010101001", nama: &["Buku Perpustakaan Hukum"], merk: TANPA_MERK, harga: (0.1, 3.0), masa: 0.0, bobot: 5 },
                Jenis { kelompok: "Lukisan Cat Minyak", kd: "6020201001", nama: &["Lukisan", "Barang Bercorak Kesenian"], merk: TANPA_MERK, harga: (1.0, 40.0), masa: 0.0, bobot: 1 },
            ],
        },
        "SIMAN2_M_ASET_ASET_TETAP_RENOVASI" => Pola {
            per_satker: 0.6,
            umur_maks: 15,
            rusak: None,
            hapus: 0.0,
            jenis: &[
                Jenis { kelompok: "Gedung dan Bangunan Dalam Renovasi", kd: "6070201001", nama: &["Renovasi Gedung Kantor Pinjam Pakai"], merk: TANPA_MERK, harga: (80.0, 1_800.0), masa: 10.0, bobot: 1 },
            ],
        },
        _ => Pola {
            per_satker: 0.28,
            umur_maks: 7,
            rusak: None,
            hapus: 0.0,
            jenis: &[
                Jenis { kelompok: "Gedung dan Bangunan Dalam Pengerjaan", kd: "7010301001", nama: &["KDP Pembangunan Gedung Kantor", "KDP Rehabilitasi Rumah Negara"], merk: TANPA_MERK, harga: (300.0, 14_000.0), masa: 0.0, bobot: 1 },
            ],
        },
    }
}

/// RNG deterministik (SplitMix64) agar data contoh stabil antar-build.
pub struct Rng(pub u64);

impl Rng {
    pub fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
    /// Pecahan seragam pada (0, 1).
    pub fn f(&mut self) -> f64 {
        ((self.next() >> 11) as f64 + 0.5) / (1u64 << 53) as f64
    }
    pub fn rentang(&mut self, lo: f64, hi: f64) -> f64 {
        lo + self.f() * (hi - lo)
    }
    pub fn pilih<'a, T>(&mut self, xs: &'a [T]) -> &'a T {
        &xs[(self.next() % xs.len() as u64) as usize]
    }
    /// Hampiran normal baku (jumlah empat seragam).
    pub fn normal(&mut self) -> f64 {
        (self.f() + self.f() + self.f() + self.f() - 2.0) * 1.732
    }
}

/// Satu satuan kerja contoh.
pub struct SatkerContoh {
    pub kode: String,
    pub nama: String,
    pub prov: &'static str,
    pub kota: String,
    /// Pengali jumlah aset.
    pub skala: f64,
    /// Mutu pemeliharaan: >1 berarti aset lebih cepat rusak.
    pub kerapuhan: f64,
    pub p_psp: f64,
    pub p_sertifikat: f64,
    pub p_idle: f64,
}

/// Daftar satuan kerja contoh (deterministik).
pub fn satker_contoh() -> Vec<SatkerContoh> {
    let mut out = Vec::new();
    let mut tambah = |nama: String, prov: &'static str, kota: String, skala: f64| {
        let urut = out.len() as u64 + 1;
        let mut rng = Rng(urut.wrapping_mul(0xA24B_AED4_963E_E407));
        let kerapuhan = (0.42 * rng.normal()).exp().clamp(0.45, 2.6);
        out.push(SatkerContoh {
            kode: format!("D{urut:05}"),
            nama,
            prov,
            kota,
            skala,
            kerapuhan,
            p_psp: 0.52 + 0.46 * rng.f(),
            p_sertifikat: 0.55 + 0.44 * rng.f(),
            p_idle: 0.006 + 0.05 * rng.f() * rng.f(),
        });
    };
    tambah(
        "KEJAKSAAN AGUNG RI".into(),
        "DKI JAKARTA",
        "JAKARTA SELATAN".into(),
        10.0,
    );
    tambah(
        "BADAN PENDIDIKAN DAN PELATIHAN KEJAKSAAN RI".into(),
        "DKI JAKARTA",
        "JAKARTA SELATAN".into(),
        3.0,
    );
    for (prov, kota) in WILAYAH {
        tambah(
            format!("KEJAKSAAN TINGGI {prov}"),
            prov,
            kota[0].to_string(),
            3.0,
        );
        for k in kota.iter() {
            tambah(format!("KEJAKSAAN NEGERI {k}"), prov, k.to_string(), 1.0);
        }
    }
    out
}

/// Bobot relatif pengadaan pada suatu tahun (meniru gelombang anggaran).
fn bobot_tahun(tahun: i32, tahun_kini: i32, bulan_kini: u32, bangunan: bool) -> f64 {
    let dasar = if bangunan {
        // Bangunan dan tanah: perolehan tersebar panjang.
        if tahun < 1985 {
            0.5
        } else {
            1.0 + 0.02 * (tahun - 1985) as f64
        }
    } else {
        1.07_f64.powi(tahun - 2008)
    };
    let gelombang = match tahun {
        2015..=2019 => 1.25,
        2020 => 0.60,
        2021 => 0.72,
        2022 => 0.95,
        2023 => 1.08,
        2024 => 1.18,
        2025 => 0.88,
        _ => 1.0,
    };
    let parsial = if tahun == tahun_kini {
        0.85 * bulan_kini as f64 / 12.0
    } else {
        1.0
    };
    dasar * gelombang * parsial
}

fn pilih_berbobot(rng: &mut Rng, bobot: &[f64]) -> usize {
    let total: f64 = bobot.iter().sum();
    let mut x = rng.f() * total;
    for (i, b) in bobot.iter().enumerate() {
        if x < *b {
            return i;
        }
        x -= b;
    }
    bobot.len() - 1
}

/// Bangkitkan baris contoh satu tabel untuk seluruh satuan kerja.
pub fn baris_tabel(tabel: &str, tahun_kini: i32, bulan_kini: u32) -> Vec<Value> {
    let Some(ti) = catalog::indeks(tabel) else {
        return vec![];
    };
    let def = &catalog::TABLES[ti];
    let p = pola(tabel);
    let bangunan = matches!(
        tabel,
        "SIMAN2_M_ASET_TANAH" | "SIMAN2_M_ASET_GEDUNG_BANGUNAN" | "SIMAN2_M_ASET_RUMAH"
    );
    let kini = tahun_kini as f64 + (bulan_kini as f64 - 0.5) / 12.0;
    let tahun_awal = tahun_kini - p.umur_maks;
    let tahun: Vec<i32> = (tahun_awal..=tahun_kini).collect();
    let bobot: Vec<f64> = tahun
        .iter()
        .map(|y| bobot_tahun(*y, tahun_kini, bulan_kini, bangunan))
        .collect();
    let bobot_jenis: Vec<f64> = p.jenis.iter().map(|j| j.bobot as f64).collect();

    let mut out = Vec::new();
    let mut urut = 0u64;
    for (si, s) in satker_contoh().iter().enumerate() {
        let mut rng = Rng((si as u64 + 1) * 1_000_003 + (ti as u64 + 1) * 7_919);
        // Jumlah calon aset; sebagian gugur karena sudah dihapus.
        let calon = p.per_satker * s.skala * rng.rentang(0.75, 1.3) * 1.18;
        let mut n = calon.floor() as usize;
        if rng.f() < calon.fract() {
            n += 1;
        }
        for _ in 0..n {
            let j = &p.jenis[pilih_berbobot(&mut rng, &bobot_jenis)];
            let th = tahun[pilih_berbobot(&mut rng, &bobot)];
            let bln = if th == tahun_kini {
                1 + (rng.next() % bulan_kini.max(1) as u64) as u32
            } else {
                1 + (rng.next() % 12) as u32
            };
            let hari = 1 + (rng.next() % 28) as u32;
            let umur = (kini - (th as f64 + (bln as f64 - 0.5) / 12.0)).max(0.0);

            // Kondisi: proses dua tahap bergantung umur dan mutu pemeliharaan satker.
            let mut kondisi = "Baik";
            let mut lama_rb = 0.0;
            if let Some((eta, beta, mu)) = p.rusak {
                let t1 = eta * (-(rng.f().ln()) / s.kerapuhan).powf(1.0 / beta);
                if t1 < umur {
                    let t2 = -(rng.f().ln()) / mu;
                    if t1 + t2 < umur {
                        kondisi = "Rusak Berat";
                        lama_rb = umur - t1 - t2;
                    } else {
                        kondisi = "Rusak Ringan";
                    }
                }
            }
            // Aset rusak berat lambat laun dihapus dari pembukuan.
            if kondisi == "Rusak Berat" && rng.f() > (-p.hapus * lama_rb).exp() {
                continue;
            }
            urut += 1;

            let harga = (j.harga.0 * (j.harga.1 / j.harga.0).powf(rng.f()) * 1_000_000.0
                / 1000.0)
                .round()
                * 1000.0;
            // Sebagian kecil tercatat Rp1 (belum dinilai) sebagai anomali data.
            let nilai = if rng.f() < 0.006 { 1.0 } else { harga };
            let (susut, sisa): (f64, Option<f64>) = if j.masa > 0.0 {
                let porsi = (umur / j.masa).min(1.0);
                // Penyusutan dihitung per semester.
                let sisa = ((j.masa - umur).max(0.0) * 2.0).ceil() / 2.0;
                ((nilai * porsi).round(), Some(sisa))
            } else {
                (0.0, None)
            };
            let buku = (nilai - susut).max(0.0);

            let hilang = def.fisik
                && !bangunan
                && matches!(
                    j.kelompok,
                    "Lap Top" | "Sepeda Motor" | "Senjata Genggam" | "Camera Video"
                )
                && rng.f() < 0.012 * s.kerapuhan;
            let p_idle = s.p_idle
                * if bangunan { 2.4 } else { 1.0 }
                * if kondisi == "Rusak Berat" { 3.0 } else { 1.0 };
            let idle = rng.f() < p_idle;
            let dihentikan = kondisi == "Rusak Berat" && rng.f() < 0.55;
            let psp = rng.f() < s.p_psp * if umur < 1.0 { 0.45 } else { 1.0 };
            let tanah = tabel == "SIMAN2_M_ASET_TANAH";
            let sertifikat = tanah && rng.f() < s.p_sertifikat;
            let sengketa = tanah && rng.f() < 0.02;
            // Sedikit isian kosong agar pemeriksaan mutu data punya bahan.
            let kondisi_isi = if def.fisik && rng.f() < 0.012 { "" } else { kondisi };
            let tgl = if rng.f() < 0.004 {
                "1900-01-01 00:00:00".to_string()
            } else {
                format!("{th}-{bln:02}-{hari:02} 00:00:00")
            };
            let nama_barang = *rng.pilih(j.nama);
            let merk = *rng.pilih(j.merk);
            let lat = -8.0 + 12.0 * rng.f();
            let lon = 96.0 + 44.0 * rng.f();
            let punya_gps = bangunan && rng.f() < 0.62;

            out.push(json!({
                "kd_jns_bmn": ti as i64 + 1,
                "kd_brg": j.kd,
                "no_aset": urut,
                "tercatat": if bangunan { "KIB" } else { "DBR" },
                "merk": merk,
                "tipe": if merk == "-" { "-" } else { "Standar" },
                "rph_aset": nilai,
                "rph_susut": susut,
                "rph_mutasi": 0,
                "rph_perolehan": nilai,
                "rph_buku": buku,
                "status_bmn_yn": "Y",
                "jml_photo": if rng.f() < 0.7 { 1 + (rng.next() % 3) as i64 } else { 0 },
                "ur_sskel": j.kelompok,
                "alamat": format!("Jl. Contoh No. {} {}", 1 + rng.next() % 180, s.kota),
                "ur_kab": s.kota,
                "ur_prov": s.prov,
                "asl_perlh": *rng.pilih(&["Pembelian", "Pembelian", "Pembelian", "Transfer Masuk", "Hibah Masuk"]),
                "cara_perlh": if bangunan { "Pembangunan" } else { "Pembelian" },
                "tgl_perlh": tgl,
                "tgl_buku_pertama": format!("{th}-{bln:02}-{hari:02} 00:00:00"),
                "ur_sumber_dana": *rng.pilih(&["APBN", "APBN", "APBN", "APBN", "PNBP", "Hibah"]),
                "status_bmn_idle": if idle { "Y" } else { "N" },
                "no_psp": if psp { format!("KEP-{}/KN/{}", 100 + rng.next() % 900, (th + 1).min(tahun_kini)) } else { String::new() },
                "luas": if bangunan { (60.0 + 2400.0 * rng.f() * rng.f()).round() } else { 0.0 },
                "no_perkara_hukum": if sengketa { format!("{}/Pdt.G/{}/PN", 10 + rng.next() % 300, tahun_kini - 1 - (rng.next() % 4) as i32) } else { String::new() },
                "no_dok_bukti_kepemilikan": if sertifikat { format!("SHP No. {}", 100 + rng.next() % 9000) } else { String::new() },
                "jns_sertifikat": if sertifikat { "Hak Pakai" } else { "" },
                "dihentikan_yn": if dihentikan { "Y" } else { "N" },
                "gps_latitude": if punya_gps { (lat * 1e5).round() / 1e5 } else { 0.0 },
                "gps_longitude": if punya_gps { (lon * 1e5).round() / 1e5 } else { 0.0 },
                "brg_hilang_yn": if hilang { "Y" } else { "N" },
                "brg_rusak_yn": if kondisi == "Baik" { "N" } else { "Y" },
                "umur_sisa": sisa,
                "jns_aset": def.label,
                "kode_satker": s.kode,
                "nama_satker": s.nama,
                "ur_kondisi": kondisi_isi,
                "nm_penghuni": if tabel == "SIMAN2_M_ASET_RUMAH" && rng.f() < 0.82 { "Pegawai (contoh)" } else { "" },
                "no_polisi": if tabel == "SIMAN2_M_ASET_ANGKUTAN_BERMOTOR" { format!("B {} XX", 1000 + rng.next() % 8999) } else { String::new() },
                "ur_kanwil": format!("KANWIL DJKN {}", s.prov),
                "ur_kl": "KEJAKSAAN REPUBLIK INDONESIA",
                "ur_eselon1": "KEJAKSAAN REPUBLIK INDONESIA",
                "kode_register": format!("CONTOH-{:02}-{:07}", ti + 1, urut),
                "intra_extra": if nilai < 1_000_000.0 && !bangunan { "Ekstra" } else { "Intra" },
                "ur_status": if dihentikan { "Dihentikan dari penggunaan" } else { "Aktif" },
                "nama": nama_barang,
                "lokasi_ruang": if bangunan { "" } else { *rng.pilih(&["Ruang Kepala", "Ruang Pidum", "Ruang Pidsus", "Ruang Datun", "Ruang Intelijen", "Ruang Pembinaan", "Gudang"]) },
                "last_update": format!("{tahun_kini}-{bulan_kini:02}-01 00:00:00"),
            }));
        }
    }
    out
}

static KUNCI_SEMAI: std::sync::Mutex<()> = std::sync::Mutex::new(());

/// Isi basis data dengan data contoh untuk seluruh tabel katalog.
///
/// Aman dipanggil bersamaan: pemanggil kedua menunggu lalu mendapati data sudah ada.
pub fn seed() {
    let _k = KUNCI_SEMAI.lock().unwrap_or_else(|e| e.into_inner());
    if !crate::store::sync_statuses().is_empty() {
        return;
    }
    let (tahun, bulan) = crate::tanggal_kini();
    let now = crate::now_string();
    for t in catalog::TABLES.iter() {
        let rows = baris_tabel(t.table, tahun, bulan);
        let _ = crate::store::ganti_tabel(t.table, rows, &now);
    }
    let _ = crate::store::set_meta("mode", "demo");
}

/// Apakah basis data saat ini berisi data contoh.
pub fn is_demo() -> bool {
    crate::store::is_demo()
}

/// Pastikan ada data untuk ditampilkan: bila kredensial belum diisi dan basis
/// data kosong, isi dengan data contoh.
pub fn pastikan() {
    if !crate::config::get().is_configured() && crate::store::sync_statuses().is_empty() {
        seed();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn data_contoh_deterministik() {
        let a = baris_tabel("SIMAN2_M_ASET_ANGKUTAN_BERMOTOR", 2026, 11);
        let b = baris_tabel("SIMAN2_M_ASET_ANGKUTAN_BERMOTOR", 2026, 11);
        assert_eq!(a.len(), b.len());
        assert_eq!(a[0], b[0]);
        assert_eq!(a[a.len() - 1], b[b.len() - 1]);
        assert!(a.len() > 1500, "angkutan contoh: {}", a.len());
    }

    #[test]
    fn struktur_satker_contoh() {
        let s = satker_contoh();
        assert!(s.len() > 150);
        assert_eq!(s[0].nama, "KEJAKSAAN AGUNG RI");
        let mut kode: Vec<&str> = s.iter().map(|x| x.kode.as_str()).collect();
        kode.sort();
        kode.dedup();
        assert_eq!(kode.len(), s.len(), "kode satker contoh harus unik");
        assert!(s.iter().all(|x| x.kode.starts_with('D')));
    }

    #[test]
    fn kondisi_memburuk_seiring_umur() {
        let rows = baris_tabel("SIMAN2_M_ASET_KHUSUS_TIK", 2026, 11);
        let hitung = |muda: bool| {
            let (mut baik, mut n) = (0.0, 0.0);
            for r in &rows {
                let th: i32 = r["tgl_perlh"].as_str().unwrap()[..4].parse().unwrap();
                if th < 1950 || r["ur_kondisi"].as_str().unwrap().is_empty() {
                    continue;
                }
                if (th >= 2023) == muda {
                    n += 1.0;
                    if r["ur_kondisi"] == "Baik" {
                        baik += 1.0;
                    }
                }
            }
            baik / n
        };
        assert!(hitung(true) > 0.85, "aset muda umumnya baik");
        assert!(hitung(false) < hitung(true) - 0.2, "aset tua lebih banyak rusak");
    }

    #[test]
    fn penyusutan_konsisten_dengan_sisa_umur() {
        let rows = baris_tabel("SIMAN2_M_ASET_NON_TIK", 2026, 11);
        for r in rows.iter().take(400) {
            let nilai = r["rph_aset"].as_f64().unwrap();
            let buku = r["rph_buku"].as_f64().unwrap();
            let sisa = r["umur_sisa"].as_f64().unwrap();
            assert!(buku <= nilai);
            if sisa == 0.0 {
                assert_eq!(buku, 0.0, "habis masa manfaat berarti nilai buku nol");
            }
        }
        // Tanah tidak disusutkan dan tidak punya sisa umur.
        let tanah = baris_tabel("SIMAN2_M_ASET_TANAH", 2026, 11);
        assert!(tanah.iter().all(|r| r["umur_sisa"].is_null()));
        assert!(tanah.iter().all(|r| r["rph_susut"] == 0.0));
    }
}
