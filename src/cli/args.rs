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
    pub(super) planning: PlanningArgs,
}
#[derive(Default)]
pub(super) struct PlanningArgs {
    pub(super) budget: Option<u64>,
    pub(super) time_limit: Option<u64>,
    pub(super) seed: Option<u64>,
    pub(super) output: Option<PathBuf>,
    pub(super) resume: bool,
    pub(super) candidate_limit: Option<usize>,
    pub(super) radius: Option<f64>,
}
impl PlanningArgs {
    pub(super) fn specified(&self) -> bool {
        self.budget.is_some()
            || self.time_limit.is_some()
            || self.seed.is_some()
            || self.output.is_some()
            || self.resume
            || self.candidate_limit.is_some()
            || self.radius.is_some()
    }
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
            "--budget" => a.planning.budget = Some(val()?.parse()?),
            "--time-limit" => a.planning.time_limit = Some(val()?.parse()?),
            "--seed" => a.planning.seed = Some(val()?.parse()?),
            "--output" => a.planning.output = Some(val()?.into()),
            "--resume" => a.planning.resume = true,
            "--candidate-limit" => a.planning.candidate_limit = Some(val()?.parse()?),
            "--radius" => a.planning.radius = Some(val()?.parse()?),
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

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn planning_options_are_explicit_and_missing_values_fail() {
        let values = [
            "--budget",
            "40",
            "--time-limit",
            "2",
            "--seed",
            "7",
            "--resume",
            "--candidate-limit",
            "12",
            "--radius",
            "24",
            "--output",
            "candidate.json",
        ]
        .map(str::to_owned);
        let args = parse(&values).unwrap();
        assert_eq!(args.planning.budget, Some(40));
        assert_eq!(args.planning.seed, Some(7));
        assert!(args.planning.resume);
        assert!(parse(&["--budget".into()]).is_err());
        assert!(parse(&["--radius".into(), "invalid".into()]).is_err());
    }
}
