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
    io::{self, Write},
    path::{Path, PathBuf},
    process::{Command as CliCommand, Output, Stdio},
    string::ToString,
};

use camino::{Utf8Path, Utf8PathBuf};
use clap::Parser;
use cmd::{AllArgs, Cli, SubCommand};
use list_generations::GenerationMeta;
use nixos_rsbuild::{
    cmd::{self, BuildSubComms},
    list_generations,
};
use tempdir::TempDir;

fn main() -> Result<(), Box<dyn Error>> {
    let cli = initial_init()?;

    // list generations
    if let Some(gen_meta) = GenerationMeta::dispatch_cmd(&cli) {
        let gens_iter = gen_meta?;
        println!("{:#?}", gens_iter.collect::<BTreeMap<_, _>>());
        return Ok(());
    }

    // plain flake build
    if let SubCommand::Builder {
        comm,
        arg,
        // all: AllArgs {
        //     flake,
        //     no_flake: false,
        //     ..
        // },
        // ..
    } = cli
    {
        log::trace!("getting full flake");
        let use_td = arg.res_dir.is_none();
        let full_flake = arg.flake.init_flake_ref()?;
        let path = arg.res_dir.unwrap_or(
            Utf8PathBuf::from_path_buf(TempDir::new("nixrsbuild-")?.into_path()).unwrap(),
        );

        let mut build_command = full_flake.build_cmd(path.as_path())?;
        log::trace!("bcom: {:?}", build_command);
        let comm_output = build_command.output()?;
        log::trace!("com: {:?}", comm_output);
        let out_link = path.join("result/bin/switch-to-configuration");

        let local_arch = std::env::var_os("LOCALE_ARCHIVE").unwrap_or_default();
        let task_str = comm.to_string();
        cmd_lib::run_cmd!(
            sudo bash -c "env -i LOCALE_ARCHIVE=$local_arch $out_link $task_str";
        )?;

        return Ok(());
    }

    // TODO: wrap and impl drop. The referenced impl does an ssh drop
    let tmpdir = tempfile::TempDir::with_prefix("nixos-rsbuild")?;
    log::trace!("using tmpdir: {}", tmpdir.path().display());

    // TODO: check for re-exec
    // let reexec_env = std::env::var("_NIXOS_REBUILD_REEXEC").unwrap_or_default();
    // if reexec_env.is_empty()
    //     && cli.can_run()
    //     && matches!(cli.inner_args(), Some(AllArgs { fast: false, .. }))
    // {
    //     todo!("handle the reexec context")
    // }

    Ok(())
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

/// Simple wrapper to trace-log commands that get run, and the renults
fn run_cmd(cmd: &mut CliCommand) -> io::Result<Output> {
    log::trace!("RUN: {:?}", cmd);
    let res = cmd.spawn().expect("failed to start").wait_with_output();
    log::trace!("RES: {:?}", res);
    res
}
