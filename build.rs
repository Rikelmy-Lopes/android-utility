//! Embeds the application icon and version metadata into Windows executables.

fn main() {
    println!("cargo:rerun-if-changed=icon/icon.ico");

    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        return;
    }

    let mut resource = winresource::WindowsResource::new();
    resource
        .set_icon("icon/icon.ico")
        .set("ProductName", "Android Utility")
        .set(
            "FileDescription",
            "Inspect and analyze Android devices through ADB",
        );

    if let Err(error) = resource.compile() {
        panic!("could not compile Windows resources: {error}");
    }
}
