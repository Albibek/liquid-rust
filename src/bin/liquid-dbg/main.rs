#![doc(html_root_url = "https://cobalt-org.github.io/liquid-rust/")]

// Deny warnings, except in dev mode
#![deny(warnings)]
// #![deny(missing_docs)]
#![cfg_attr(feature="dev", warn(warnings))]

// Allow zero pointers for lazy_static. Otherwise clippy will complain.
#![allow(unknown_lints)]
#![allow(zero_ptr)]

#[macro_use]
extern crate clap;
#[macro_use]
extern crate error_chain;
extern crate liquid;

#[cfg(feature = "serde_yaml")]
extern crate serde_yaml;
#[cfg(feature = "serde_json")]
extern crate serde_json;

use std::ffi;
use std::fs;
use std::io;
use std::io::Write;
use std::path;
use liquid::Renderable;

error_chain! {
    links {
    }

    foreign_links {
        Clap(clap::Error);
        Io(io::Error);
        Liquid(liquid::Error);
        Yaml(serde_yaml::Error) #[cfg(feature = "serde_yaml")];
        Json(serde_json::Error) #[cfg(feature = "serde_json")];
    }

    errors {
    }
}

fn option<'a>(name: &'a str, value: &'a str) -> clap::Arg<'a, 'a> {
    clap::Arg::with_name(name).long(name).value_name(value)
}

#[cfg(feature = "serde_yaml")]
fn load_yaml(path: &path::Path) -> Result<liquid::Value> {
    let f = fs::File::open(path)?;
    serde_yaml::from_reader(f).map_err(|e| e.into())
}

#[cfg(not(feature = "serde_yaml"))]
fn load_yaml(_path: &path::Path) -> Result<liquid::Value> {
    bail!("yaml is unsupported");
}

#[cfg(feature = "serde_json")]
fn load_json(path: &path::Path) -> Result<liquid::Value> {
    let f = fs::File::open(path)?;
    serde_json::from_reader(f).map_err(|e| e.into())
}

#[cfg(not(feature = "serde_json"))]
fn load_json(_path: &path::Path) -> Result<liquid::Value> {
    bail!("json is unsupported");
}

fn build_context(path: &path::Path) -> Result<liquid::Context> {
    let extension = path.extension().unwrap_or_else(|| ffi::OsStr::new(""));
    let value = if extension == ffi::OsStr::new("yaml") {
        load_yaml(path)
    } else if extension == ffi::OsStr::new("yaml") {
        load_json(path)
    } else {
        Err("Unsupported file type".into())
    }?;
    let value = match value {
        liquid::Value::Object(o) => Ok(o),
        _ => Err("File must be an object"),
    }?;
    let data = liquid::Context::with_values(value);

    Ok(data)
}

fn run() -> Result<()> {
    let matches = clap::App::new("liquidate")
        .version(crate_version!())
        .author(crate_authors!())
        .arg(option("input", "LIQUID").required(true))
        .arg(option("output", "TXT"))
        .arg(option("context", "TOML"))
        .arg(option("include-root", "PATH"))
        .get_matches_safe()?;

    let mut options = liquid::LiquidOptions::default();
    let root = matches
        .value_of("include-root")
        .map(path::PathBuf::from)
        .unwrap_or_default();
    options.template_repository = Box::new(liquid::LocalTemplateRepository::new(root));

    let mut data = matches
        .value_of("context")
        .map(|s| {
                 let p = path::PathBuf::from(s);
                 build_context(p.as_path())
             })
        .map_or(Ok(None), |r| r.map(Some))?
        .unwrap_or_else(liquid::Context::new);

    let template_path = matches
        .value_of("input")
        .map(path::PathBuf::from)
        .expect("Parameter was required");
    let template = liquid::parse_file(template_path, options)?;
    let output = template
        .render(&mut data)?
        .unwrap_or_else(|| "".to_string());
    match matches.value_of("output") {
        Some(path) => {
            let mut out = fs::File::create(path::PathBuf::from(path))?;
            out.write_all(output.as_bytes())?;
        }
        None => {
            println!("{}", output);
        }
    }

    Ok(())
}

quick_main!(run);
