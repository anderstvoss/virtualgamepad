use serde_json::{Value, json};
use std::{
    path::{Path, PathBuf},
    process::{Command, ExitCode},
};
use xtask::{Config, Fake, Monotonic, run};
fn output(command: &str, args: &[&str], root: &Path) -> Option<String> {
    let result = Command::new(command)
        .args(args)
        .current_dir(root)
        .output()
        .ok()?;
    result
        .status
        .success()
        .then(|| String::from_utf8_lossy(&result.stdout).trim().to_owned())
}
fn environment(root: &Path, backend: &str) -> Value {
    json!({"backend":backend,"synthetic":backend=="fake","git_sha":output("git",&["rev-parse","HEAD"],root),
        "dirty":output("git",&["status","--porcelain","--untracked-files=no"],root).map(|s|!s.is_empty()),
        "research_revision":output("git",&["rev-parse","HEAD:protocol-research"],root),
        "kernel":output("uname",&["-r"],root),"cpu_arch":std::env::consts::ARCH,
        "cpu_count":std::thread::available_parallelism().ok().map(std::num::NonZeroUsize::get),
        "rustc":output("rustc",&["--version"],root),"sdl_version":output("pkg-config",&["--modversion","sdl3"],root),
        "probe_source_sha256":output("sha256sum",&["scripts/sdl3-gamepad-probe.c"],root).and_then(|s|s.split_whitespace().next().map(str::to_owned)),
        "sdl_backend":{"value":null,"reason":"not observed"},"sdl_hints":{"value":null,"reason":"not captured; never assume defaults"},
        "host_label":"current-local-host","wall_clock_utc":output("date",&["-u","+%Y-%m-%dT%H:%M:%SZ"],root)})
}
fn diff(args: &[String], root: &Path) -> Result<(), String> {
    if args.len() != 3 {
        return Err("acceptance-diff <run.json> <reference.json>".into());
    }
    let runs: Vec<Value> = args[1..]
        .iter()
        .map(|p| {
            std::fs::read_to_string(p)
                .map_err(|e| e.to_string())
                .and_then(|s| serde_json::from_str(&s).map_err(|e| e.to_string()))
        })
        .collect::<Result<_, _>>()?;
    if runs
        .iter()
        .all(|r| r["schema_version"] == 1 && r["kind"] == "virtual_host_run")
    {
        println!(
            "{}",
            json!({"run":runs[0],"reference":runs[1],"verdict":null,"reason":"timing comparison has no acceptance budget"})
        );
        return Ok(());
    }
    let status = Command::new("python3")
        .arg(root.join("scripts/compare-sdl3-observations.py"))
        .args(&args[1..])
        .status()
        .map_err(|e| e.to_string())?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("SDL comparator exited {status}"))
    }
}
fn execute() -> Result<(), String> {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .to_path_buf();
    let args: Vec<_> = std::env::args().skip(1).collect();
    if args.first().map(String::as_str) == Some("acceptance-diff") {
        return diff(&args, &root);
    }
    if args.first().map(String::as_str) != Some("acceptance") {
        return Err("expected acceptance or acceptance-diff".into());
    }
    let mut config = Config {
        controller: String::new(),
        realization: String::new(),
        cycles: 50,
        concurrency: 1,
    };
    let mut backend = "linux".to_owned();
    let mut out = None;
    let mut reference = None;
    let mut sdl = false;
    let mut i = 1;
    while i < args.len() {
        if args[i] == "--sdl" {
            sdl = true;
            i += 1;
            continue;
        }
        let value = args.get(i + 1).ok_or("missing option value")?;
        match args[i].as_str() {
            "--controller" => config.controller.clone_from(value),
            "--realization" => config.realization.clone_from(value),
            "--backend" => backend.clone_from(value),
            "--cycles" => config.cycles = value.parse().map_err(|_| "invalid cycles")?,
            "--concurrency" => {
                config.concurrency = value.parse().map_err(|_| "invalid concurrency")?;
            }
            "--out" => out = Some(value.clone()),
            "--reference" => reference = Some(value.clone()),
            _ => return Err(format!("unknown option {}", args[i])),
        }
        i += 2;
    }
    config.family()?;
    let env = environment(&root, &backend);
    let mut clock = Monotonic::default();
    let result = match backend.as_str() {
        "fake" => run(&mut Fake::default(), &mut clock, &config, &env)?,
        "linux" => {
            #[cfg(target_os = "linux")]
            {
                run(
                    &mut xtask::linux::Linux { sdl, root },
                    &mut clock,
                    &config,
                    &env,
                )?
            }
            #[cfg(not(target_os = "linux"))]
            {
                let _ = (sdl, root);
                return Err("live acceptance runs need Linux".into());
            }
        }
        _ => return Err("backend must be fake or linux".into()),
    };
    let result = if let Some(reference) = reference {
        let text = std::fs::read_to_string(reference).map_err(|e| e.to_string())?;
        let reference: Value = serde_json::from_str(&text).map_err(|e| e.to_string())?;
        json!({"run":result,"reference":reference,"verdict":null,"reason":"no acceptance budgets"})
    } else {
        result
    };
    let text = serde_json::to_string_pretty(&result).map_err(|e| e.to_string())?;
    if let Some(path) = out {
        std::fs::write(path, text + "\n").map_err(|e| e.to_string())?;
    } else {
        println!("{text}");
    }
    Ok(())
}
fn main() -> ExitCode {
    match execute() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::from(2)
        }
    }
}
