//! 命令行参数与工程目录解析。
use ra3maps::{Result, err, pipeline::build::Options};
use std::{env, fs, path::PathBuf};

#[derive(Default)]
pub(super) struct Args {
    pub(super) root: Option<PathBuf>,
    pub(super) map: Option<String>,
    pub(super) opts: Options,
    pub(super) set: Option<String>,
    pub(super) show: bool,
    pub(super) positional: Vec<String>,
}
pub(super) fn parse(input: &[String]) -> Result<Args> {
    let mut a = Args::default();
    let mut i = 0;
    while i < input.len() {
        let arg = &input[i];
        let mut val = || -> Result<String> {
            i += 1;
            input
                .get(i)
                .cloned()
                .ok_or_else(|| err(format!("{arg} requires a value")))
        };
        match arg.as_str() {
            "--root" => a.root = Some(val()?.into()),
            "--map" => a.map = Some(val()?),
            "--only" => a.opts.only.push(val()?),
            "--skip" => a.opts.skip.push(val()?),
            "--bump" => a.opts.bump = true,
            "--release" => a.opts.release = true,
            "--install" => a.opts.install = true,
            "--no-zip" => a.opts.no_zip = true,
            "--wb" | "--install-wb" => a.opts.wb_install = true,
            "--wb-root" => a.opts.wb_root = Some(val()?.into()),
            "--note" => a.opts.note = val()?,
            "--set" => a.set = Some(val()?),
            "--show" => a.show = true,
            "--" => {
                a.positional.extend_from_slice(&input[i + 1..]);
                break;
            }
            "-" => a.positional.push(arg.clone()),
            s if s.starts_with('-') => return Err(err(format!("unknown option {s}"))),
            _ => a.positional.push(arg.clone()),
        }
        i += 1;
    }
    Ok(a)
}
pub(super) fn find_root(a: &Args) -> Result<PathBuf> {
    if let Some(root) = a
        .root
        .clone()
        .or_else(|| env::var_os("RA3MAP_ROOT").map(PathBuf::from))
    {
        return Ok(fs::canonicalize(root)?);
    }
    let cwd = env::current_dir()?;
    for root in cwd.ancestors() {
        if root.join("Cargo.toml").is_file() && root.join("maps").is_dir() {
            return Ok(root.into());
        }
    }
    if let Ok(exe) = env::current_exe() {
        for root in exe.ancestors() {
            if root.join("maps").is_dir() && root.join("templates").is_dir() {
                return Ok(root.into());
            }
        }
    }
    Err(err("cannot find project root; use --root or RA3MAP_ROOT"))
}
