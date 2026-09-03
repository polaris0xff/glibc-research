use std::{
    env,
    path::{Path, PathBuf},
    io::{Seek, SeekFrom, Write},
    process::{exit, Command, Stdio},
    fs::{create_dir_all, rename, OpenOptions},
};


const BIN_NAME: &str = "uruntime";

type DynError = Box<dyn std::error::Error>;

fn target_for(arch: &str) -> &'static str {
    match arch {
        "x86_64" => "x86_64-unknown-linux-musl",
        "aarch64" => "aarch64-unknown-linux-musl",
        "riscv64" => "riscv64gc-unknown-linux-musl",
        "loongarch64" => "loongarch64-unknown-linux-musl",
        "ppc64" => "powerpc64-unknown-linux-musl",
        "ppc64le" => "powerpc64le-unknown-linux-musl",
        _ => panic!("unknown arch: {}", arch),
    }
}

fn main() {
    if let Err(e) = try_main() {
        eprintln!("{}", e);
        exit(-1);
    }
}

fn try_main() -> Result<(), DynError> {
    let mut all_bins: Vec<String> = Vec::new();
    for arch in ["x86_64", "aarch64", "riscv64", "loongarch64", "ppc64", "ppc64le"] {
        all_bins.append(&mut vec![
            format!("runimage-{arch}"),
            format!("appimage-{arch}"),
            format!("appimage-lite-{arch}"),
        ]);
    }
    let arg = env::args().nth(1).unwrap_or_else(||{
        "".into()
    });
    let arg = arg.as_str();

    if all_bins.iter().any(|bin| bin == arg) {
        build(arg)?;
        return Ok(())
    }

    match arg {
        "all" => {
            for bin in all_bins {
                build(&bin)?
            }
        },
        _ => {
            if all_bins.iter().any(|bin| bin.ends_with(arg)) {
                for bin in all_bins.iter().filter(|bin| bin.ends_with(arg)) {
                    build(&bin)?
                }
            } else {
                print_help()
            }
        },
    }
    Ok(())
}

fn create_dist_dir() -> Result<(), DynError> {
    create_dir_all(dist_dir())?;
    Ok(())
}

fn print_help() {
    eprintln!("Tasks:
    x86_64                           build x86_64 RunImage and AppImage uruntime
    runimage-x86_64                  build x86_64 RunImage uruntime
    appimage-x86_64                  build x86_64 AppImage uruntime
    appimage-lite-x86_64             build x86_64 AppImage uruntime (no dwarfsck, mkdwarfs)

    aarch64                          build aarch64 RunImage and AppImage uruntime
    runimage-aarch64                 build aarch64 RunImage uruntime
    appimage-aarch64                 build aarch64 AppImage uruntime
    appimage-lite-aarch64            build aarch64 AppImage uruntime (no dwarfsck, mkdwarfs)

    riscv64                          build riscv64 RunImage and AppImage uruntime
    runimage-riscv64                 build riscv64 RunImage uruntime
    appimage-riscv64                 build riscv64 AppImage uruntime
    appimage-lite-riscv64            build riscv64 AppImage uruntime (no dwarfsck, mkdwarfs)

    loongarch64                      build loongarch64 RunImage and AppImage uruntime
    runimage-loongarch64             build loongarch64 RunImage uruntime
    appimage-loongarch64             build loongarch64 AppImage uruntime
    appimage-lite-loongarch64        build loongarch64 AppImage uruntime (no dwarfsck, mkdwarfs)

    ppc64                             build ppc64 RunImage and AppImage uruntime
    runimage-ppc64                    build ppc64 RunImage uruntime
    appimage-ppc64                    build ppc64 AppImage uruntime
    appimage-lite-ppc64               build ppc64 AppImage uruntime (no dwarfsck, mkdwarfs)

    ppc64le                          build ppc64le RunImage and AppImage uruntime
    runimage-ppc64le                 build ppc64le RunImage uruntime
    appimage-ppc64le                 build ppc64le AppImage uruntime
    appimage-lite-ppc64le            build ppc64le AppImage uruntime (no dwarfsck, mkdwarfs)

    all                              build all of the above")
}

fn strip(path: &PathBuf) -> Result<(), DynError> {
    if Command::new("strip")
        .arg("--version")
        .stdout(Stdio::null())
        .status()
        .is_ok()
    {
        eprint!(" stripping: ");
        let status = Command::new("strip").args([
            "-s", "-R", ".comment", "-R", ".gnu.version",
            "--strip-unneeded"
        ]).arg(path).status()?;
        if !status.success() {
            Err("strip failed")?;
        }
        eprint!("OK");
    } else {
        Err("no `strip` utility found!")?;
    }
    Ok(())
}

fn add_sections(path: &PathBuf) -> Result<(), DynError> {
    if Command::new("llvm-objcopy")
        .arg("--version")
        .stdout(Stdio::null())
        .status()
        .is_ok()
    {
        eprint!(" add sections: ");

        if let Ok(dir) = sections_dir().read_dir() {
            let mut objcopy_args = Vec::new();
            for entry in dir.flatten() {
                let section_file = entry.path();
                if section_file.is_file() {
                    let section_name = format!(".{}", section_file.file_name().unwrap_or_default().to_string_lossy());
                    objcopy_args.append(&mut vec![
                        format!("--add-section={section_name}={}", section_file.display()),
                        format!("--set-section-flags={section_name}=noload,readonly"),
                    ]);
                }
            }
            let status = Command::new("llvm-objcopy")
                .args(objcopy_args).arg(path).status()?;
            if !status.success() {
                Err("failed to add sections")?;
            }
        }
        eprint!("OK");
    } else {
        Err("no `llvm-objcopy` utility found!")?;
    }
    Ok(())
}

fn add_magic(path: &PathBuf, magic: &str) -> Result<(), DynError> {
    eprint!(" embed magic: ");
    let magic = format!("{}\x02", magic);
    let mut file = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(false)
        .open(path)?;
    file.seek(SeekFrom::Start(8))?;
    file.write_all(magic[..3].as_bytes())?;
    eprint!("OK");
    Ok(())
}

fn build(bin: &str) -> Result<(), DynError> {
    create_dist_dir()?;

    let arch = bin.rsplit('-').next().unwrap_or_default();
    let target = target_for(arch);
    // the host `strip` cannot recognise foreign architectures
    let is_strip = arch == "x86_64";

    let mut build_args = Vec::new();
    build_args.append(&mut vec![
        "build", "--release",
        "--target", target
    ]);

    if bin.contains("dwarfs") {
        build_args.append(&mut vec!["--no-default-features", "--features", "dwarfs"]);
    }
    let mut magic = "RI";
    if bin.contains("appimage") {
        magic = "AI";
        build_args.append(&mut vec!["--features", "appimage"])
    }
    if bin.contains("lite") {
        build_args.append(&mut vec!["--features", "lite"]);
    }

    let upx = env::args().nth(2).unwrap_or_default().to_lowercase() == "--upx";
    if upx { build_args.append(&mut vec!["--features", "upx"]) }

    let status = Command::new("cargo")
        .current_dir(project_root())
        .args(build_args)
        .status()?;

    if !status.success() {
        Err("cargo build failed")?;
    }

    let src = project_root()
        .join("target")
        .join(target)
        .join("release")
        .join(BIN_NAME);

    let dst_bin_name = if upx {
        &format!("{BIN_NAME}-{bin}-upx")
    } else {
        &format!("{BIN_NAME}-{bin}")
    };
    let dst = dist_dir().join(dst_bin_name);

    rename(&src, &dst)?;
    eprint!("{dst_bin_name}: OK");

    if is_strip {
        strip(&dst)?;
    }

    add_sections(&dst)?;

    add_magic(&dst, magic)?;

    eprintln!();
    Ok(())
}

fn project_root() -> PathBuf {
    Path::new(&env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(1)
        .unwrap()
        .to_path_buf()
}

fn dist_dir() -> PathBuf {
    project_root().join("dist")
}

fn sections_dir() -> PathBuf {
    project_root().join("sections")
}
