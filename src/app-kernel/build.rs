use {
    bindgen::EnumVariation,
    chrono::{DateTime, Local},
    compression::prelude::*,
    dotenv::dotenv,
    glob::glob,
    std::{
        env,
        ffi::OsStr,
        fs::{File, create_dir_all, read_to_string},
        io::Write,
        path::PathBuf,
        process::Command
    }
};

fn main() {
    dotenv().ok();

    let now: DateTime<Local> = Local::now();
    println!("cargo::rustc-env=BUILD_TIME={now}");
    println!(
        "cargo::rustc-env=BUILD_PROFILE={}",
        env::var("PROFILE").unwrap()
    );
    println!(
        "cargo::rustc-env=BUILD_FEATURES={}",
        env::var("CARGO_CFG_FEATURE").unwrap()
    );

    //
    // Configuration
    //
    let pkg_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    let src_dir = PathBuf::from_iter([&pkg_dir, "src"]);
    let inc_dir = PathBuf::from_iter([env!("PWD"), "include"]);
    let target_dir = format!("{}/../../..", env::var("OUT_DIR").unwrap());

    if env!("APP_ENV") == "prod" {
        println!("cargo::rerun-if-changed={}", env!("PWD"));
    }
    println!("cargo::rerun-if-env-changed=APP_ENV");
    println!("cargo::rerun-if-changed={}/.env", env!("PWD"));
    println!("cargo::rerun-if-changed={pkg_dir}/build.rs");
    println!("cargo::rerun-if-changed={pkg_dir}/src/lib.rs");
    println!("cargo::rerun-if-changed={pkg_dir}/src/app_c.rs");
    println!("cargo::rerun-if-changed={pkg_dir}/cbindgen.toml");

    //
    // Command help
    //

    let help_file_name = "cmd-help.txt";
    let help_file_path = format!("{target_dir}/{help_file_name}");
    println!("cargo::rustc-env=HELP_FILE={help_file_path}");

    let mut help_buf = String::new();

    if let Ok(buf) = read_to_string(concat!(env!("PWD"), "/doc/{help_file_name}")) {
        help_buf.push_str(&buf);
        help_buf.push('\n');
    }

    let paths = [
        concat!(env!("PWD"), "/*/Cargo.toml"),
        concat!(env!("PWD"), "/crates/*/Cargo.toml"),
        concat!(env!("PWD"), "/vendor/*/crates/*/Cargo.toml")
    ];
    for path in paths {
        for mut file in glob(path).unwrap().flatten() {
            file.pop();
            file.extend(&["doc", help_file_name]);
            if file.exists() {
                let content = read_to_string(file.as_path()).unwrap();
                help_buf.push_str(&content);
                help_buf.push('\n');
            }
        }
    }

    let content = help_buf
        .into_bytes()
        .into_iter()
        .encode(&mut ZlibEncoder::new(), Action::Finish)
        .collect::<Result<Vec<_>, _>>()
        .unwrap();

    let mut help_file = File::create(&help_file_path).unwrap();
    help_file.write_all(&content).unwrap();
    help_file.flush().unwrap();
    drop(help_file);

    //
    // Linking libraries
    //
    println!("cargo::rustc-link-search={target_dir}");

    //
    // Binding C code
    //
    create_dir_all(inc_dir.as_path())
        .expect(&format!("Couldn't create directory: {inc_dir:?}"));

    //println!("cargo::warning={:?} was formatted successfully.", &out_path);

    // Skip bindings if no feature = "bind"
    if env::var("CARGO_CFG_FEATURE").is_ok_and(|s| s.contains("bind")) == false {
        return;
    }

    //
    // Binding Rust code
    //
    let include_file = format!(
        "lib{}.h",
        env::var("CARGO_PKG_NAME").unwrap().replace("-", "_")
    );
    let cbindgens_filename =
        PathBuf::from_iter([inc_dir.as_os_str(), OsStr::new(&include_file)]);

    cbindgen::Builder::new()
        .with_config(cbindgen::Config::from_file("cbindgen.toml").unwrap())
        .with_crate(env::var("CARGO_MANIFEST_DIR").unwrap())
        .with_parse_expand_features(&["bind"])
        .generate()
        .expect("Unable to generate bindings")
        .write_to_file(cbindgens_filename.clone());

    let bindings_file =
        PathBuf::from_iter([src_dir.as_os_str(), OsStr::new("bindings_gen.rs.new")]);

    bindgen::Builder::default()
        .use_core()
        .header(cbindgens_filename.to_string_lossy())
        .default_enum_style(EnumVariation::Rust { non_exhaustive: false })
        .allowlist_item("app_.*")
        .allowlist_item("MOD_.*")
        .allowlist_item("ERR_.*")
        .no_copy("App")
        .generate()
        .expect("Unable to generate bindings")
        .write_to_file(&bindings_file)
        .expect("Couldn't write bindings!");

    let output = Command::new("rustup")
        .args(["run", "nightly", "rustfmt", bindings_file.to_str().unwrap()])
        .output()
        .expect("Could not format binding file.");

    assert!(
        output.status.success(),
        "Unsuccessful status code when running `rustfmt`: {output:?}",
    );
}
