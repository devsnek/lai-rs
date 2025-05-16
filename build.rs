use std::error::Error;
use std::path::Path;
use std::process::Command;

// lai/meson.build - sources
const SOURCES: &[&str] = &[
    "core/error.c",
    "core/eval.c",
    "core/exec.c",
    "core/exec-operand.c",
    "core/libc.c",
    "core/ns.c",
    "core/object.c",
    "core/opregion.c",
    "core/os_methods.c",
    "core/variable.c",
    "core/vsnprintf.c",
    "helpers/pc-bios.c",
    "helpers/pci.c",
    "helpers/resource.c",
    "helpers/sci.c",
    "helpers/pm.c",
    "drivers/ec.c",
    "drivers/timer.c",
];

const LAI_GITHUB_URL: &str = "https://github.com/managarm/lai";

fn main() -> Result<(), Box<dyn Error>> {
    let out_dir = std::env::var_os("OUT_DIR").unwrap();
    let lai_path = Path::new(&out_dir).join("bundled").join("lai");
    let lai_path_str = lai_path.to_string_lossy();

    if !lai_path.exists() {
        Command::new("git")
            .args(["clone", LAI_GITHUB_URL, &lai_path_str])
            .status()
            .unwrap();
    }

    let sources = SOURCES
        .iter()
        .map(|file| format!("{lai_path_str}/{file}"))
        .collect::<Vec<_>>();

    // NOTE: The build is configured for kernel enviornments.
    cc::Build::new()
        .files(sources)
        .include(format!("{lai_path_str}/include"))
        .flag("-fno-stack-protector")
        // The mmx and sse features determine support for SIMD instructions, which can often speed up
        // programs significantly. However, using the large SIMD registers in OS kernels leads to
        // performance problems.
        .flag("-mno-sse")
        .flag("-mno-mmx")
        // Inform the compiler that the target does not have floating-point hardware.
        .flag("-msoft-float")
        // Disable stack pointer optimization called the “red zone”, because it would cause
        // stack corruptions otherwise.
        .flag("-mno-red-zone")
        .flag("-fno-builtin")
        .flag("-nostdlib")
        .flag("-ffreestanding")
        .flag("-fpic")
        .compile("lai");

    println!("cargo:rerun-if-changed=src/wrapper.h");

    bindgen::Builder::default()
        .use_core()
        .clang_arg(format!("-I{lai_path_str}/include"))
        .header("src/wrapper.h")
        .parse_callbacks(Box::new(bindgen::CargoCallbacks))
        .generate()
        .unwrap()
        .write_to_file(Path::new(&out_dir).join("lai_sys.rs"))
        .unwrap();

    Ok(())
}
