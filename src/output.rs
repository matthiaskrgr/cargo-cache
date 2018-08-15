use humansize::{file_size_opts, FileSize};

use crate::library::*;

impl DirSizes {
    pub(crate) fn print_pretty(&self, ccd: &CargoCachePaths) -> String {
        // create a string and concatenate all the things we want to print with it
        // and only print it in the end, this should save a few syscalls and be faster than
        // printing every line one by one

        // @TODO use format_args!() ?
        let mut s = String::with_capacity(470);

        s.push_str(&format!(
            "Cargo cache '{}/':\n\n",
            &ccd.cargo_home.display()
        ));

        s.push_str(&format!(
            "Total size: {: >35}\n",
            self.total_size.file_size(file_size_opts::DECIMAL).unwrap()
        ));

        // the nested format!()s are a hack to get nice alignment of the numbers
        // any ideas on how to not uses nested format here is appreciate...
        s.push_str(&format!(
            "{: <41} {}\n",
            &format!("Size of {} installed binaries:", self.numb_bins,),
            self.total_bin_size
                .file_size(file_size_opts::DECIMAL)
                .unwrap()
        ));

        s.push_str(&format!(
            "Size of registry: {: >33}\n",
            self.total_reg_size
                .file_size(file_size_opts::DECIMAL)
                .unwrap()
        ));

        s.push_str(&format!(
            "{: <44}{}\n",
            &format!("Size of {} crate archives:", self.numb_reg_cache_entries),
            self.total_reg_cache_size
                .file_size(file_size_opts::DECIMAL)
                .unwrap()
        ));

        s.push_str(&format!(
            "{: <43} {}\n",
            &format!(
                "Size of {} crate source checkouts:",
                self.numb_reg_src_checkouts
            ),
            self.total_reg_src_size
                .file_size(file_size_opts::DECIMAL)
                .unwrap()
        ));

        s.push_str(&format!(
            "Size of git db: {: >35}\n",
            self.total_git_db_size
                .file_size(file_size_opts::DECIMAL)
                .unwrap()
        ));

        s.push_str(&format!(
            "{: <43} {}\n",
            &format!("Size of {} bare git repos:", self.numb_git_repos_bare_repos),
            self.total_git_repos_bare_size
                .file_size(file_size_opts::DECIMAL)
                .unwrap()
        ));

        s.push_str(&format!(
            "{: <43} {}", /* final println already introduces \n */
            &format!("Size of {} git repo checkouts:", self.numb_git_checkouts),
            self.total_git_chk_size
                .file_size(file_size_opts::DECIMAL)
                .unwrap()
        ));

        s
    }
}

impl CargoCachePaths {
    pub(crate) fn get_dir_paths(&self) -> String {
        let mut s = String::with_capacity(500);
        s.push_str("\n");
        s.push_str(&format!(
            "cargo home:                 {}\n",
            &self.cargo_home.display()
        ));

        s.push_str(&format!(
            "binaries directory:         {}\n",
            &self.bin_dir.display()
        ));
        s.push_str(&format!(
            "registry directory:         {}\n",
            &self.registry.display()
        ));
        s.push_str(&format!(
            "registry index:             {}\n",
            &self.registry_index.display()
        ));
        s.push_str(&format!(
            "crate source archives:      {}\n",
            &self.registry_cache.display()
        ));
        s.push_str(&format!(
            "unpacked crate sources:     {}\n",
            &self.registry_sources.display()
        ));
        s.push_str(&format!(
            "bare git repos:             {}\n",
            &self.git_repos_bare.display()
        ));
        s.push_str(&format!(
            "git repo checkouts:         {}\n",
            &self.git_checkouts.display()
        ));
        s
    }
}

pub(crate) fn get_info(c: &CargoCachePaths, s: &DirSizes) -> String {
    let mut strn = String::with_capacity(1020);
    strn.push_str("Found CARGO_HOME / cargo cache base dir\n");
    strn.push_str(&format!(
        "\t\t\t'{}' of size: {}\n",
        &c.cargo_home.display(),
        s.total_size.file_size(file_size_opts::DECIMAL).unwrap()
    ));

    strn.push_str(&format!("Found {} binaries installed in\n", s.numb_bins));
    strn.push_str(&format!(
        "\t\t\t'{}', size: {}\n",
        &c.bin_dir.display(),
        s.total_bin_size.file_size(file_size_opts::DECIMAL).unwrap()
    ));
    strn.push_str("\t\t\tNote: use 'cargo uninstall' to remove binaries, if needed.\n");

    strn.push_str("Found registry base dir:\n");
    strn.push_str(&format!(
        "\t\t\t'{}', size: {}\n",
        &c.registry.display(),
        s.total_reg_size.file_size(file_size_opts::DECIMAL).unwrap()
    ));
    strn.push_str("Found registry crate source cache:\n");
    strn.push_str(&format!(
        "\t\t\t'{}', size: {}\n",
        &c.registry_cache.display(),
        s.total_reg_cache_size
            .file_size(file_size_opts::DECIMAL)
            .unwrap()
    ));
    strn.push_str("\t\t\tNote: removed crate sources will be redownloaded if necessary\n");
    strn.push_str("Found registry unpacked sources\n");
    strn.push_str(&format!(
        "\t\t\t'{}', size: {}\n",
        &c.registry_sources.display(),
        s.total_reg_src_size
            .file_size(file_size_opts::DECIMAL)
            .unwrap()
    ));
    strn.push_str("\t\t\tNote: removed unpacked sources will be reextracted from local cache (no net access needed).\n");

    strn.push_str("Found git repo database:\n");
    strn.push_str(&format!(
        "\t\t\t'{}', size: {}\n",
        &c.git_repos_bare.display(),
        s.total_git_repos_bare_size
            .file_size(file_size_opts::DECIMAL)
            .unwrap()
    ));
    strn.push_str("\t\t\tNote: removed git repositories will be recloned if necessary\n");
    strn.push_str("Found git repo checkouts:\n");
    strn.push_str(&format!(
        "\t\t\t'{}', size: {}\n",
        &c.git_checkouts.display(),
        s.total_git_chk_size
            .file_size(file_size_opts::DECIMAL)
            .unwrap()
    ));
    strn.push_str(
        "\t\t\tNote: removed git checkouts will be rechecked-out from repo database if necessary (no net access needed, if repos are up-to-date).\n"
    );
    strn
}
