use std::path::PathBuf;

use kira_spatial_3d_viewer::run_viewer;

fn main() {
    match parse_args() {
        Ok((mesh, polylines)) => {
            if let Err(e) = run_viewer(&mesh, polylines.as_deref()) {
                eprintln!("{e}");
                std::process::exit(1);
            }
        }
        Err(msg) => {
            eprintln!("{msg}");
            eprintln!("usage: kira-spatial-3d-viewer --mesh <prefix> [--polylines <path>]");
            std::process::exit(2);
        }
    }
}

fn parse_args() -> Result<(PathBuf, Option<PathBuf>), String> {
    let mut args = std::env::args().skip(1);
    let mut mesh: Option<PathBuf> = None;
    let mut polylines: Option<PathBuf> = None;

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--mesh" => {
                let v = args
                    .next()
                    .ok_or_else(|| "missing value for --mesh".to_string())?;
                mesh = Some(PathBuf::from(v));
            }
            "--polylines" => {
                let v = args
                    .next()
                    .ok_or_else(|| "missing value for --polylines".to_string())?;
                polylines = Some(PathBuf::from(v));
            }
            _ => return Err(format!("unknown argument: {arg}")),
        }
    }

    let mesh = mesh.ok_or_else(|| "--mesh is required".to_string())?;
    Ok((mesh, polylines))
}
