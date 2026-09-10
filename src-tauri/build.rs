fn main() {
    // tauri-build only emits rerun-if-changed for tauri.conf.json and capabilities.
    // The icons are compiled into the Windows executable as resource 32512 via a
    // generated resource.rc, but nothing tracks them -- so editing an icon alone
    // leaves Cargo with no reason to relink and the *old* icon stays embedded in
    // the binary. Track the directory explicitly.
    println!("cargo:rerun-if-changed=icons");
    tauri_build::build()
}
