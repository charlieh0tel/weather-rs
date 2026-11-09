//! Checks a command's man pages in `man/` against its command line, for each command's tests.
//! `UPDATE_MAN_PAGES=1 cargo test` rewrites them.

use std::fs;
use std::path::Path;

pub fn check(command: clap::Command) {
    // Without the version, a release doesn't change every page.
    let command = command.version(None::<&str>);
    let generated = tempfile::tempdir().unwrap();
    clap_mangen::generate_to(command, generated.path()).unwrap();

    let man_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("man");
    let update = std::env::var_os("UPDATE_MAN_PAGES").is_some();
    for entry in fs::read_dir(generated.path()).unwrap() {
        let page = entry.unwrap();
        let committed = man_dir.join(page.file_name());
        let contents = fs::read(page.path()).unwrap();
        if update {
            fs::write(&committed, contents).unwrap();
        } else {
            assert!(
                fs::read(&committed).is_ok_and(|committed| committed == contents),
                "{} is out of date; run UPDATE_MAN_PAGES=1 cargo test",
                committed.display()
            );
        }
    }
}
