use clap::{Parser, Subcommand};
use pr_cli::cmd_test::command_test;
use pr_cli::color::*;
use pr_cli::commands_extra;
use pr_cli::install::command_install;
use pr_cli::login::command_login;
use pr_cli::plugin::load_plugins;
use pr_cli::shared;
use std::path::PathBuf;

const VERSION: &str = shared::VERSION;

#[derive(Parser)]
#[command(
    name = "pr-cli",
    version = VERSION,
    about = "proot-distro replacement for Android"
)]
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
    List {
        #[arg(short, long)]
        verbose: bool,
    },
    Backup {
        distro: String,
        #[arg(long)]
        output: Option<String>,
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
    Test {
        distro: String,
        #[arg(short, long)]
        suite: Option<String>,
        #[arg(short, long)]
        verbose: bool,
    },
    #[command(name = "clear-cache")]
    ClearCache {},
}

fn get_prefix() -> PathBuf {
    PathBuf::from(shared::get_prefix())
}

fn get_plugins_dir() -> PathBuf {
    PathBuf::from(shared::get_plugins_dir())
}

fn get_installed_rootfs_dir() -> PathBuf {
    PathBuf::from(shared::get_installed_rootfs_dir())
}

fn is_installed(alias: &str) -> bool {
    get_installed_rootfs_dir().join(alias).is_dir()
}

fn command_list(verbose: bool) {
    let dir = get_plugins_dir();
    let plugins = load_plugins(&dir);

    println!();

    if plugins.is_empty() {
        println!("{}No distribution plug-ins found.{}", YELLOW, RESET);
        println!();
        println!(
            "{}Please check the directory '{}' and create at least one distribution plug-in.{}",
            YELLOW,
            dir.display(),
            RESET
        );
        println!();
        return;
    }

    if verbose {
        println!("{}Supported distributions:{}", CYAN, RESET);
    } else {
        println!(
            "{}Supported distributions (format: name < alias >):{}",
            CYAN, RESET
        );
        println!();
    }

    for p in &plugins {
        if verbose {
            println!();
            println!("  {}* {}{}{}", CYAN, YELLOW, p.name, RESET);
            println!();

            println!("    {}Alias: {}{}{}", CYAN, GREEN, p.alias, RESET);

            if is_installed(&p.alias) {
                println!("    {}Installed: {}yes{}", CYAN, GREEN, RESET);
            } else {
                println!("    {}Installed: {}no{}", CYAN, RED, RESET);
            }

            if let Some(ref comment) = p.comment {
                println!("    {}Comment: {}{}{}", CYAN, RESET, comment, RESET);
            }

            let archs = p.supported_architectures().join(", ");
            println!("    {}Architectures: {}{}{}", CYAN, RESET, archs, RESET);
        } else {
            println!(
                "  {}* {}{} {}< {}{}>{}",
                CYAN, YELLOW, p.name, GREEN, p.alias, RESET, ""
            );
        }
    }

    println!();
    println!(
        "{}Install selected one with: {}pr-cli install <alias>{}",
        CYAN, GREEN, RESET
    );
    println!();
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Install {
            distro,
            override_alias,
            override_tarball_url,
            override_tarball_sha256,
        } => {
            if let Err(e) = command_install(
                &distro,
                override_alias.as_deref(),
                override_tarball_url.as_deref(),
                override_tarball_sha256.as_deref(),
            ) {
                eprintln!("Error: {}", e);
                std::process::exit(1);
            }
        }
        Commands::Login {
            distro,
            user,
            isolated,
            no_link2symlink,
            custom_bind,
        } => {
            if let Err(e) =
                command_login(&distro, &user, isolated, no_link2symlink, &custom_bind, &[])
            {
                eprintln!("Error: {}", e);
                std::process::exit(1);
            }
        }
        Commands::Remove { distro } => {
            if let Err(e) = commands_extra::command_remove(&distro, false) {
                eprintln!("Error: {}", e);
                std::process::exit(1);
            }
        }
        Commands::List { verbose } => {
            command_list(verbose);
        }
        Commands::Backup { distro, output } => {
            if let Err(e) = commands_extra::command_backup(&distro, output.as_deref()) {
                eprintln!("Error: {}", e);
                std::process::exit(1);
            }
        }
        Commands::Restore { distro } => {
            if let Err(e) = commands_extra::command_restore(&distro) {
                eprintln!("Error: {}", e);
                std::process::exit(1);
            }
        }
        Commands::Rename {
            old_alias,
            new_alias,
        } => {
            if let Err(e) = commands_extra::command_rename(&old_alias, &new_alias) {
                eprintln!("Error: {}", e);
                std::process::exit(1);
            }
        }
        Commands::Reset { distro } => {
            if let Err(e) = commands_extra::command_reset(&distro) {
                eprintln!("Error: {}", e);
                std::process::exit(1);
            }
        }
        Commands::Copy {
            src_distro,
            dst_distro,
        } => {
            if let Err(e) = commands_extra::command_copy(&src_distro, &dst_distro) {
                eprintln!("Error: {}", e);
                std::process::exit(1);
            }
        }
        Commands::Test {
            distro,
            suite,
            verbose,
        } => {
            if let Err(e) = command_test(&distro, suite.as_deref(), verbose) {
                eprintln!("Error: {}", e);
                std::process::exit(1);
            }
        }
        Commands::ClearCache { .. } => {
            if let Err(e) = commands_extra::command_clear_cache() {
                eprintln!("Error: {}", e);
                std::process::exit(1);
            }
        }
    }
}
