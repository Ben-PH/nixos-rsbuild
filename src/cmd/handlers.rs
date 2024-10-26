use std::io;

use camino::Utf8PathBuf;
use tempdir::TempDir;

use super::AllArgs;

impl super::BuildSubComms {
    /// Builds a config, capturing a sym-link. Follows up with a call to `switch-to-configuration`
    /// as appropriate.
    pub fn run_build(&self, args: AllArgs) -> io::Result<()> {
        log::trace!("Constructing configuration: {:?}", args);
        let (res_dir, use_td) = self.build_configuration(args)?;

        // Execute switch-to-configuration provided by the configuration build.
        // This is where the switch/boot/test/dry-activate component gets carried out
        if matches!(
            self,
            Self::Switch | Self::Boot | Self::Test | Self::DryActivate
        ) {
            let switch_bin = res_dir.join("result/bin/switch-to-configuration");
            let out_link = std::fs::canonicalize(switch_bin).unwrap();

            let local_arch = std::env::var_os("LOCALE_ARCHIVE").unwrap_or_default();
            let task_str = self.to_string();
            let _ = cmd_lib::spawn!(
                sudo nu -c "env -i LOCALE_ARCHIVE=$local_arch $out_link $task_str"
            )?
            .wait();
        }

        // Sanity-check that we are actually cleaning up a tempdir, and not nuking something that
        // shouldn't be. This could justifyably be removed, as the OS GCs the tempdir anyway.
        if use_td {
            log::trace!("Cleaning up tempdir used to link to nix-store: {}", res_dir);
            let sys_td = std::env::temp_dir();
            assert!(std::fs::exists(&sys_td).unwrap());
            assert!(res_dir.starts_with(sys_td));
            assert!(
                res_dir.file_name().unwrap().starts_with("nixrsbuild-"),
                "{}",
                res_dir.file_name().unwrap()
            );
            let _ = std::fs::remove_dir_all(res_dir);
        }

        Ok(())
    }

    /// Builds the configuration, and returns the link to the nix store repo. The `bool` tag
    /// indicates if the link is placed in a temp-dir.
    fn build_configuration(&self, args: AllArgs) -> io::Result<(Utf8PathBuf, bool)> {
        let use_td = args.res_dir.is_none();
        let full_flake = args.flake.init_flake_ref(self)?;
        let res_dir = args.res_dir.unwrap_or(
            Utf8PathBuf::from_path_buf(TempDir::new("nixrsbuild-")?.into_path()).unwrap(),
        );
        log::trace!("Result link directory: {}", res_dir);
        full_flake
            .run_nix_build(res_dir.as_path())
            .map(|_| (res_dir, use_td))
    }
}
