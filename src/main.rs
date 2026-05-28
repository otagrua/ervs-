use clap::Parser;
use std::{
    env::home_dir,
    ffi::OsStr,
    io::Write,
    path::{Path, PathBuf},
};

macro_rules! err {
    ($x:expr) => {
        eprintln!("Err: \x1b[1m\x1b[31m{}\x1b[0m", $x)
    };
}
macro_rules! ok {
    ($x:expr) => {
        match $x {
            Ok(x) => x,
            Err(x) => {
                err!(x);
                return;
            }
        }
    };
}

#[derive(clap::Parser)]
/// Manages user services. Wraps the systemctl command
enum Commands {
    Start { service: String },
    Stop { service: String },
    Restart { service: String },
    Reload { service: String },
    Status { service: String },
    New { service: String },
    Destroy { service: String },
    List,
}

fn run<I, S>(args: I)
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    let x = ok!(ok!(std::process::Command::new("systemctl")
        .arg("--user")
        .args(args)
        .spawn())
    .wait());

    println!(
        "\x1b[1m{}{x}\x1b[0m",
        if x.success() { "\x1b[32m" } else { "\x1b[31m" }
    );
}

fn service_parent() -> Option<PathBuf> {
    let Some(home) = home_dir() else {
        err!("No home path");
        return None;
    };

    Some(home.join(".config").join("systemd").join("user"))
}
fn service_path(service: &str) -> Option<PathBuf> {
    let path = service_parent()?.join(service);

    let path = if path.extension() == Some(OsStr::new("service")) {
        path
    } else {
        path.with_added_extension("service")
    };

    Some(path)
}

fn new(service: &str) {
    let Some(path) = service_path(service) else {
        return;
    };

    println!("\x1b[2m{}\x1b[0m", path.display());

    let mut service_file = ok!(std::fs::File::create(path));

    let mut permissions = ok!(service_file.metadata()).permissions();
    permissions.set_readonly(true);
    ok!(service_file.set_permissions(permissions));

    let cwd = Path::new("%h").join(service);
    let start = cwd.join(service);
    ok!(service_file.write_all(
        format!(
            "[Unit]
Description={service} (generated)
After=network.target
Wants=network-online.target

[Service]
Restart=on-failure
Type=simple
ExecStart={}
WorkingDirectory={}
StandardOutput=journal
StandardError=journal

[Install]
WantedBy=default.target",
            start.display(),
            cwd.display()
        )
        .as_bytes()
    ));

    run(["enable", "--now", service]);
    run(["status", service]);
}

fn destroy(service: &str) {
    let Some(path) = service_path(service) else {
        return;
    };

    println!("\x1b[2m{}\x1b[0m", path.display());

    run(["disable", "--now", service]);
    ok!(std::fs::remove_file(path));
    run(["daemon-reload"]);
    run(["status", service]);
}

fn list() {
    let Some(path) = service_parent() else {
        return;
    };

    for file in ok!(std::fs::read_dir(path)) {
        let file = ok!(file);
        if !ok!(file.metadata()).is_file() {
            continue;
        }
        println!("{}", file.file_name().display());
    }
}

fn main() {
    let command = Commands::parse();
    match command {
        Commands::Start { service } => run(["start", &service]),
        Commands::Stop { service } => run(["stop", &service]),
        Commands::Restart { service } => run(["restart", &service]),
        Commands::Reload { service } => run(["reload", &service]),
        Commands::Status { service } => run(["status", &service]),
        Commands::New { service } => new(&service),
        Commands::Destroy { service } => destroy(&service),
        Commands::List => list(),
    }
}
