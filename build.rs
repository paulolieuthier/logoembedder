use std::process::Command;
use std::env;

fn main() {
    let out_dir = env::var("OUT_DIR").unwrap();

    Command::new("glib-compile-resources")
        .args(&["--sourcedir", "res"])
        .args(&["--target", &format!("{}/app.gresource", out_dir)])
        .arg("res/app.gresource.xml")
        .status()
        .unwrap();

    println!("cargo:rerun-if-changed=res/app.gresource.xml");
    println!("cargo:rerun-if-changed=res/app.ui");
}
