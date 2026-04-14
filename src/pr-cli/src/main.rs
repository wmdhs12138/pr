use clap::{Parser, Subcommand};
use pr_cli::plugin::load_plugins;
use std::path::PathBuf;

const VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Parser)]
#[command(name = "pr-cli", version = VERSION, about = "proot-distro replacement for Android")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Install {
        distro: String,
        #[arg(long)]
        override_alias: Option<String>,
        #[arg(long)]
        override_tarball_url: Option<String>,
        #[arg(long)]
        override_tarball_sha256: Option<String>,
    },
    Login {
        distro: String,
        #[arg(long, default_value = "root")]
        user: String,
        #[arg(long)]
        isolated: bool,
        #[arg(long)]
        no_link2symlink: bool,
        #[arg(long)]
        custom_bind: Vec<String>,
    },
    Remove {
        distro: String,
    },
    List {},
    Backup {
        distro: String,
    },
    Restore {
        distro: String,
    },
    Rename {
        old_alias: String,
        new_alias: String,
    },
    Reset {
        distro: String,
    },
    Copy {
        src_distro: String,
        dst_distro: String,
    },
    #[command(name = "clear-cache")]
    ClearCache {},
}

fn get_plugins_dir() -> PathBuf {
    let prefix = std::env::var("APP_PREFIX")
        .unwrap_or_else(|_| "/data/data/id.or.oo.pr/files/usr".to_string());
    PathBuf::from(prefix).join("etc/proot-distro")
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Install { distro, .. } => {
            eprintln!("install: not yet implemented (distro={})", distro);
            std::process::exit(1);
        }
        Commands::Login { distro, .. } => {
            eprintln!("login: not yet implemented (distro={})", distro);
            std::process::exit(1);
        }
        Commands::Remove { distro } => {
            eprintln!("remove: not yet implemented (distro={})", distro);
            std::process::exit(1);
        }
        Commands::List { .. } => {
            let dir = get_plugins_dir();
            let plugins = load_plugins(&dir);
            if plugins.is_empty() {
                eprintln!("No distributions found in {}", dir.display());
                std::process::exit(1);
            }
            for p in &plugins {
                println!("{}", p);
            }
        }
        Commands::Backup { distro } => {
            eprintln!("backup: not yet implemented (distro={})", distro);
            std::process::exit(1);
        }
        Commands::Restore { distro } => {
            eprintln!("restore: not yet implemented (distro={})", distro);
            std::process::exit(1);
        }
        Commands::Rename {
            old_alias,
            new_alias,
        } => {
            eprintln!(
                "rename: not yet implemented ({} → {})",
                old_alias, new_alias
            );
            std::process::exit(1);
        }
        Commands::Reset { distro } => {
            eprintln!("reset: not yet implemented (distro={})", distro);
            std::process::exit(1);
        }
        Commands::Copy {
            src_distro,
            dst_distro,
        } => {
            eprintln!(
                "copy: not yet implemented ({} → {})",
                src_distro, dst_distro
            );
            std::process::exit(1);
        }
        Commands::ClearCache { .. } => {
            eprintln!("clear-cache: not yet implemented");
            std::process::exit(1);
        }
    }
}
