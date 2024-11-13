use std::env;
use std::path::PathBuf;

fn main() {
    let res = pkg_config::Config::new()
        .atleast_version("6.4.1")
        .statik(true)
        .probe("clips")
        .unwrap();

    let include_paths = res
        .include_paths
        .into_iter()
        .map(|p| format!("-I{}", p.to_str().unwrap()));

    let bindings = bindgen::Builder::default()
        .header("wrapper.h")
        .clang_args(include_paths)
        .derive_debug(true)
        .impl_debug(true)
        .derive_default(true)
        .parse_callbacks(Box::new(bindgen::CargoCallbacks::new()))
        .generate()
        .expect("Unable to generate bindings");

    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    bindings
        .write_to_file(out_path.join("bindings.rs"))
        .expect("Couldn't write bindings!");
}
