fn main() {
    cxx_build::bridge("src/api.rs")
        .file("cpp/api.cpp")
        .file("cpp/api_shim.cpp")
        .include("cpp")
        .flag_if_supported("-std=c++14")
        .flag_if_supported("-fvisibility=default")
        .compile("open-bambu-net");

    // Tell linker to export all symbols listed in exports.txt
    println!("cargo:rustc-link-arg=-Wl,-exported_symbols_list,{}/exports.txt",
        std::env::var("CARGO_MANIFEST_DIR").unwrap());

    println!("cargo:rerun-if-changed=src/api.rs");
    println!("cargo:rerun-if-changed=cpp/api.cpp");
    println!("cargo:rerun-if-changed=cpp/api.hpp");
    println!("cargo:rerun-if-changed=cpp/api_shim.cpp");
    println!("cargo:rerun-if-changed=cpp/api_shim.hpp");
    println!("cargo:rerun-if-changed=exports.txt");
}
