use clap::{App, Arg};
//use regex::Regex;
use serde_json::Value;
use std::env;
use std::path;
use std::process::Command;
use walkdir::WalkDir;

fn main() -> std::io::Result<()> {
    let exp_dep = App::new("trivy-exp-dep")
        .version("0.1.2")
        .author("Anton Gura <satandyh@yandex.ru>")
        .about("A Trivy plugin that scans the filesystem and skips all packages except for explicitly specified dependencies.")
        .arg(Arg::with_name("path")
            .short('p')
            .long("path")
            //.required(true)
            .takes_value(true)
            .default_value_os(env::current_dir()?.as_os_str())
            .help("Directory where to scan. Current Working dir is default."))
        .arg(Arg::with_name("global")
            .long("global")
            //.last(true)
            .global(true)
            .takes_value(true)
            .multiple(true)
            .allow_hyphen_values(true)
            .required(false)
            .help("Indicate that all flags after will be passed as trivy global/fs options.\nPositional, should be after \"-p/-h/--\" options."))
        .get_matches();

    let project_path = exp_dep.value_of("path").unwrap();
    if !path::Path::new(project_path).is_dir() {
        eprintln!("No such directory to scan {}", project_path);
        std::process::exit(1)
    }
    let global: Vec<&str>;

    // firstscan
    let prescan = path::Path::new(env::temp_dir().as_path()).join("prescan.json");
    let mut firstscan = Command::new("trivy");
    if !exp_dep.value_of("global").is_none() {
        global = exp_dep.values_of("global").unwrap().collect();
        firstscan
            .arg("fs")
            .arg("-q")
            .arg("-f")
            .arg("json")
            .arg("-o")
            .arg(prescan.to_str().unwrap());
        for opt in global {
            firstscan.arg(opt);
        }
        firstscan.arg(project_path);
    } else {
        firstscan.args([
            "fs",
            "-q",
            "-f",
            "json",
            "-o",
            prescan.to_str().unwrap(),
            project_path,
        ]);
    }
    let firstscanres = firstscan.output()?;
    if !firstscanres.status.success() {
        String::from_utf8(firstscanres.stderr)
            .into_iter()
            .for_each(|x| eprintln!("{:#?}", x));
        std::process::exit(1);
    }

    // findfiles
    if !path::Path::new(prescan.to_str().unwrap()).exists() {
        eprintln!(
            "No such file or it's can't be read {}",
            prescan.to_str().unwrap()
        );
        std::process::exit(1)
    }
    let prescan_json = {
        let jsondata = std::fs::read_to_string(prescan.to_str().unwrap()).unwrap();
        serde_json::from_str::<Value>(&jsondata).unwrap()
    };

    let mut prescan_pkg = Vec::new();
    if prescan_json.get("Results") != None {
        for index1 in 0..prescan_json["Results"].as_array().unwrap().len() {
            for index2 in 0..prescan_json["Results"][index1]["Vulnerabilities"]
                .as_array()
                .unwrap()
                .len()
            {
                prescan_pkg.push(
                    prescan_json["Results"][index1]["Vulnerabilities"][index2]["PkgName"]
                        .as_str()
                        .unwrap(),
                );
            }
        }
        prescan_pkg.dedup();
        println!("{}", prescan_pkg.join(" "));
    } else {
        // the same as if prescan_pkg.len() == 0
        std::fs::copy(
            prescan.to_str().unwrap(),
            format!("{}{}", project_path, "/trivy.json".to_string()),
        )?;
        std::fs::remove_file(prescan.to_str().unwrap())?;
        std::process::exit(0);
    }

    // filterfind
    for entry in WalkDir::new(project_path)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        if entry
            .file_name()
            .to_ascii_lowercase()
            .to_string_lossy()
            .ends_with("pipfile")
        {
            println!("{}", entry.path().to_string_lossy());
        }
    }

    Ok(())
}