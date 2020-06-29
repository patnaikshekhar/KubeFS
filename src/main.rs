mod fs;

use clap::{App, Arg};
use std::ffi::OsStr;
use fs::KubeFS;


fn main() {

    // Parse command line arguments
    let matches = App::new("KubeFS")
        .version("0.0.1")
        .about("Mounts Kubernetes as a filesystem")
        .arg(Arg::with_name("mountpath")
            .help("Set the path where to mount the file system")
            .required(true)
            .index(1)).get_matches();

    let mount_path = matches.value_of("mountpath").expect("Mount path is a required parameter");

    let options = ["-o", "ro", "-o", "fsname=kubefs"]
        .iter()
        .map(|o| o.as_ref())
        .collect::<Vec<&OsStr>>();

    fuse::mount(KubeFS, &mount_path, &options).unwrap();
}
