#![allow(unused)]

use clap::Args;

#[derive(Args, Debug)]
pub struct RbFlag {
    #[clap(long)]
    rollback: bool,
}
#[derive(Args, Debug)]
struct SpecArg {
    specialisation: Option<String>,
}

#[derive(Args, Debug)]
#[allow(clippy::struct_excessive_bools)]
struct FlakeBuildArgs {
    #[clap(long)]
    recreate_lock_file: bool,
    #[clap(long)]
    no_update_lock_file: bool,
    #[clap(long)]
    no_write_lock_file: bool,
    #[clap(long)]
    no_registries: bool,
    #[clap(long)]
    commit_lock_file: bool,
    #[clap(long)]
    update_input: Option<String>,
    #[clap(long)]
    #[arg(num_args(2))]
    override_input: Option<String>,
    #[clap(long)]
    impure: bool,
}
