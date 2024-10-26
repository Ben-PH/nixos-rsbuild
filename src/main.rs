//! As per description, is a reimpl of nixos-rebuild with room for opinionation.
//!
//! If you are at a "Rust Curious" level, and find this code base esoteric, please raise an issue on
//! github. We aren't here to gate-keep, or make the learning curve of Rust any steeper than it
//! needs to be.
//!
//! Some hints to read through the codebase:
//!
//!  - You'll often find relatively esoteric rust idioms, particularly with the deep functional
//!    mappings, chainings, etc. Read the comments, take your time.
//!  - Use LSP for code navigation. goto definition, reference, type definition etc.
//!  - Think in terms of types. The business logic is intended, as much as possible, to be
//!    encapsulated at the type-level. lines-of-code is lower level impl detail.
//!  - glhf

#![warn(clippy::pedantic)]
#![allow(clippy::uninlined_format_args)]
#![allow(clippy::module_name_repetitions)]
#![allow(unused)]

use std::{
    collections::BTreeMap,
    error::Error,
    io::{self, BufRead, BufReader, Write},
    path::{Path, PathBuf},
    process::{Command as CliCommand, Output, Stdio},
    string::ToString,
};

use camino::{Utf8Path, Utf8PathBuf};
use clap::Parser;
use cmd::{AllArgs, Cli, SubCommand};
use list_generations::GenerationMeta;
use nixos_rsbuild::{
    cmd::{self, BuildSubComms, UtilSubCommand},
    list_generations,
};
use tempdir::TempDir;

fn main() -> Result<(), Box<dyn Error>> {
    let cli = initial_init()?;

    match cli {
        // We only have list-generations util running for now
        SubCommand::Util {
            task: UtilSubCommand::ListGenerations { json },
        } => {
            let gens_iter = GenerationMeta::run_cmd()?;
            println!("{:#?}", gens_iter.collect::<BTreeMap<_, _>>());
            Ok(())
        }
        // I've only tried out build, test, switch, boot.
        SubCommand::Builders { task, arg } => Ok(task.run_build(arg)?),
        SubCommand::Util { task } => unimplemented!("todo: implement {:?}", task),
    }
}

/// Sanatises arg[0]
/// Ensures not run as root
/// Initialises logger
/// Parses cli, returning the subcommand of the result
fn initial_init() -> Result<SubCommand, Box<dyn Error>> {
    if nix::unistd::Uid::current().is_root() {
        // TODO: this pre-empts automation. something to think about
        return Err("This program should not be run as root!".into());
    };

    // sanatise executable name
    let args = std::env::args();
    let mut args = args.peekable();
    let Some(fst) = args.peek() else {
        return Err("No args present in invocation".into());
    };
    if !fst.ends_with("nixos-rsbuild") {
        return Err("Cli args did not begin with a path to file named 'nixos-rsbuild'".into());
    };

    // initialise logger
    env_logger::Builder::new()
        .format(|buf, rec| {
            writeln!(
                buf,
                "{}:{} [{}]\t{}",
                rec.file().unwrap_or("unknown"),
                rec.line().unwrap_or(0),
                rec.level(),
                rec.args()
            )
        })
        .filter_level(log::LevelFilter::Trace)
        .init();

    // parse out cli args into a structured encapsulation
    let cli = {
        let cli = Cli::parse_from(args);
        log::trace!("parsed cli: {:?}", cli);
        // mut_cli.command.try_init_to_default_flake();
        cli
    };
    Ok(cli.command)
}
