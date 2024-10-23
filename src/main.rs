#![warn(clippy::pedantic)]
#![allow(clippy::uninlined_format_args)]
#![allow(clippy::module_name_repetitions)]
#![allow(unused)]
use clap::Parser;
use cmd::{AllArgs, Cli, SubCommand};
use list_generations::GenerationMeta;
use std::{
    collections::BTreeMap,
    error::Error,
    io::{self, Write},
    process::{Command as CliCommand, Output},
};
mod cmd;
mod flake;
mod list_generations;
pub mod utils;

fn main() -> Result<(), Box<dyn Error>> {
    let cli = initial_init()?;

    // list generations
    if let Some(gen_meta) = GenerationMeta::dispatch_cmd(&cli) {
        let gens_iter = gen_meta?;
        println!("{:#?}", gens_iter.collect::<BTreeMap<_, _>>());
        return Ok(());
    }

    // plain flake build
    if let SubCommand::Build {
        all: AllArgs {
            flake,
            no_flake: false,
            ..
        },
        ..
    } = cli
    {
        log::trace!("getting full flake");
        let full_flake = flake.init_flake_ref()?;
        let out_link = full_flake.build(None)?;
        log::trace!("outlink: {}", out_link);
        return Ok(());
    }

    // TODO: wrap and impl drop. The referenced impl does an ssh drop
    let tmpdir = tempfile::TempDir::with_prefix("nixos-rsbuild")?;
    log::trace!("using tmpdir: {}", tmpdir.path().display());

    // TODO: check for re-exec
    let reexec_env = std::env::var("_NIXOS_REBUILD_REEXEC").unwrap_or_default();
    if reexec_env.is_empty()
        && cli.can_run()
        && matches!(cli.inner_args(), Some(AllArgs { fast: false, .. }))
    {
        todo!("handle the reexec context")
    }

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
        .filter_level(log::LevelFilter::Error)
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
