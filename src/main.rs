mod fs;
mod inode;
mod kube_client;

use clap::{App, Arg};
use fs::KubeFS;
use kube_client::KubeClient;
use std::ffi::OsStr;

fn main() {
    let kube = KubeClient::new();

    // Parse command line arguments
    let matches = App::new("KubeFS")
        .version("0.0.1")
        .about("Mounts Kubernetes as a filesystem")
        .arg(
            Arg::with_name("mountpath")
                .help("Set the path where to mount the file system")
                .required(true)
                .index(1),
        )
        .get_matches();

    let mount_path = matches
        .value_of("mountpath")
        .expect("Mount path is a required parameter");

    let options = ["-o", "wro", "-o", "fsname=kubefs", "-o", "auto_unmount"]
        .iter()
        .map(|o| o.as_ref())
        .collect::<Vec<&OsStr>>();

    println!("Mounting to location {}", mount_path);

    let fs = KubeFS::new(kube);

    fuse::mount(fs, &mount_path, &options).unwrap();
}
