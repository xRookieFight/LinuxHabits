fn main() {
    println!("cargo:rerun-if-changed=assets/linuxhabits.ico");
    // hmm, build scripts run on the host, so check the target instead of cfg!(windows)
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        let mut res = winresource::WindowsResource::new();
        res.set_icon("assets/linuxhabits.ico");
        res.compile().expect("failed to embed the app icon");
    }
}
