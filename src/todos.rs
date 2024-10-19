
fn get_config(sub_cmd: &SubCommands) -> io::Result<Utf8PathBuf> {
    if !sub_cmd.rollback() {
        println!("building the system configuration...");
        match sub_cmd {
            SubCommands::Switch { all, .. } | SubCommands::Boot { all, .. } => {
                let nix_env_set = if !all.building_attribute() {
                    // nixBuild $buildFile -A "${attr:+$attr.}config.system.build.toplevel" "${extraBuildFlags[@]}"
                    todo!()
                } else if all.flake.is_none() {
                    // nixBuild '<nixpkgs/nixos>' --no-out-link -A system "${extraBuildFlags[@]}"
                    todo!()
                } else {
                    // nixFlakeBuild "$flake#$flakeAttr.config.system.build.toplevel" "${extraBuildFlags[@]}" "${lockFlags[@]}"
                    flake_build::flake_build_config(sub_cmd, &[])
                }?;
                // TODO: support target/host shenanigans
                // targetHostSudoCmd nix-env -p "$profile" --set "$pathToConfig"
                let mut cmd = CliCommand::new("nix-env");
                cmd.args([
                    "-p",
                    all.profile_path.as_str(),
                    "--set",
                    nix_env_set.as_str(),
                ]);
                let _ = run_cmd(&mut cmd)?;
                Ok(nix_env_set)
            }
            SubCommands::Test { all, .. }
            | SubCommands::Build { all, .. }
            | SubCommands::DryActivate { all }
            | SubCommands::DryBuild { all } => {
                todo!("drybuild");
                if !all.building_attribute() {
                    // pathToConfig="$(nixBuild $buildFile -A "${attr:+$attr.}config.system.build.vm" "${extraBuildFlags[@]}")"
                    todo!()
                } else if all.flake.is_none() {
                    // pathToConfig="$(nixBuild '<nixpkgs/nixos>' -A vm -k "${extraBuildFlags[@]}")"
                    todo!()
                } else {
                    // pathToConfig="$(nixFlakeBuild "$flake#$flakeAttr.config.system.build.vm" "${extraBuildFlags[@]}" "${lockFlags[@]}")"
                    todo!()
                }
            }
            SubCommands::Edit { .. } => todo!(),
            SubCommands::Repl { .. } => todo!(),
            SubCommands::BuildVm { all } => {
                todo!("build vm");
                if !all.building_attribute() {
                    // pathToConfig="$(nixBuild $buildFile -A "${attr:+$attr.}config.system.build.vmWithBootLoader" "${extraBuildFlags[@]}")"
                    todo!()
                } else if all.flake.is_none() {
                    // pathToConfig="$(nixBuild '<nixpkgs/nixos>' -A vmWithBootLoader -k "${extraBuildFlags[@]}")"
                    todo!()
                } else {
                    // pathToConfig="$(nixFlakeBuild "$flake#$flakeAttr.config.system.build.vmWithBootLoader" "${extraBuildFlags[@]}" "${lockFlags[@]}")"
                    todo!()
                }
            }
            SubCommands::BuildVmWithBootloader { all } => {
                todo!("build vm w/ bootloader");
                if !all.building_attribute() {
                    // pathToConfig="$(nixBuild $buildFile -A "${attr:+$attr.}config.system.build.vm" "${extraBuildFlags[@]}")"
                    todo!()
                } else if all.flake.is_none() {
                    // pathToConfig="$(nixBuild '<nixpkgs/nixos>' -A vm -k "${extraBuildFlags[@]}")"
                    todo!()
                } else {
                    // pathToConfig="$(nixFlakeBuild "$flake#$flakeAttr.config.system.build.vm" "${extraBuildFlags[@]}" "${lockFlags[@]}")"
                    todo!()
                }
            }
            SubCommands::ListGenerations { .. } => {
                panic!("internal error, list generations matched in irrelevant context")
            }
        }
        // copy_to_target(path_to_config)
    } else {
        // doing rollback
        match sub_cmd {
            SubCommands::Switch { all, .. } | SubCommands::Boot { all, .. } => {
                // targetHostSudoCmd nix-env --rollback -p "$profile"
                Ok(all.profile_path.clone())
            }
            SubCommands::Test { all, .. } | SubCommands::Build { all, .. } => {
                // systemNumber=$(
                //     targetHostCmd nix-env -p "$profile" --list-generations |
                //     sed -n '/current/ {g; p;}; s/ *\([0-9]*\).*/\1/; h'
                // )
                // pathToConfig="$profile"-${systemNumber}-link
                // if [ -z "$targetHost" ]; then
                //     ln -sT "$pathToConfig" ./result
                // fi
                //
                todo!()
            }
            _ => panic!("internal error, subcommand matched in irrelevant context"),
        }
    }
}

fn switch_to_bin_subpath(command: &SubCommands) -> Utf8PathBuf {
    let tail = ["bin", "switch-to-configuration"];
    let mut buf = Utf8PathBuf::new();
    match command {
        SubCommands::Switch {
            specialisation: Some(sp),
            ..
        }
        | SubCommands::Test {
            specialisation: Some(sp),
            ..
        } => {
            buf.push("specialisation");
            buf.push(sp);
        }
        _ => {}
    }
    for p in tail {
        buf.push(p);
    }
    buf
}
fn run_switch_to_config(command: SubCommands, cfg_path: &Utf8Path) {
    // TODO: check if systemd-run can run `yes` under `target_host_sudo_cmd`
    // irrelevant for now, as we are just going 100% local

    let switch_to_path = switch_to_bin_subpath(&command);
    let switch_to_path = Utf8PathBuf::from(cfg_path).join(switch_to_path);
    let mut switch_cmd = CliCommand::new("sudo");
    switch_cmd.env_clear();
    if let Some(loc) = std::env::var_os("LOCALE_ARCHIVE") {
        switch_cmd.env("LOCALE_ARCHIVE", loc);
    }
    if let Some(AllArgs {
        install_bootloader: true,
        ..
    }) = command.inner_args()
    {
        switch_cmd.env("NIXOS_INSTALL_BOOTLOADER", "1");
    }
    switch_cmd.arg(switch_to_path);
}
