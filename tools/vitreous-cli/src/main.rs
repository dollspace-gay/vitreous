use clap::{Parser, Subcommand};
use std::path::PathBuf;
use vitreous_hot_reload::{HotReloadServer, ServerConfig, DEFAULT_PORT};

#[derive(Parser)]
#[command(name = "vitreous", about = "vitreous GUI framework CLI")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Create a new vitreous project
    New {
        /// Project name (used as directory name and crate name)
        name: String,
    },
    /// Start the development server with hot reload
    Dev {
        /// Port for the hot reload WebSocket server
        #[arg(short, long, default_value_t = DEFAULT_PORT)]
        port: u16,
        /// Disable automatic `cargo build` on source changes
        #[arg(long)]
        no_build: bool,
        /// Directory to watch (defaults to current directory)
        #[arg(short, long)]
        dir: Option<PathBuf>,
    },
    /// Build the project for release
    Build,
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::New { name } => {
            if let Err(e) = scaffold_project(&name) {
                eprintln!("Error: {e}");
                std::process::exit(1);
            }
        }
        Commands::Dev {
            port,
            no_build,
            dir,
        } => {
            let config = ServerConfig {
                watch_dir: dir.unwrap_or_else(|| PathBuf::from(".")),
                port,
                auto_build: !no_build,
                ..ServerConfig::default()
            };
            let server = HotReloadServer::new(config);
            if let Err(e) = server.run().await {
                eprintln!("Error: {e}");
                std::process::exit(1);
            }
        }
        Commands::Build => {
            let status = std::process::Command::new("cargo")
                .args(["build", "--release"])
                .status();

            match status {
                Ok(s) if s.success() => {}
                Ok(s) => std::process::exit(s.code().unwrap_or(1)),
                Err(e) => {
                    eprintln!("Error: failed to run cargo build: {e}");
                    std::process::exit(1);
                }
            }
        }
    }
}

fn scaffold_project(name: &str) -> Result<(), Box<dyn std::error::Error>> {
    let project_dir = PathBuf::from(name);
    if project_dir.exists() {
        return Err(format!("directory '{name}' already exists").into());
    }

    std::fs::create_dir_all(project_dir.join("src"))?;

    let cargo_toml = format!(
        r#"[package]
name = "{name}"
version = "0.1.0"
edition = "2024"

[dependencies]
vitreous = {{ version = "0.1.0" }}
"#
    );

    let title = to_title_case(name);
    let main_rs = format!(
        r#"use vitreous::{{App, Theme, create_signal, v_stack, h_stack, text, button, spacer, Node}};

fn root() -> Node {{
    let count = create_signal(0i32);

    v_stack((
        text("{title}")
            .font_size(24.0),
        text(move || format!("Count: {{}}", count.get()))
            .font_size(18.0),
        h_stack((
            button("- Decrement")
                .on_click(move || count.set(count.get() - 1)),
            spacer(),
            button("+ Increment")
                .on_click(move || count.set(count.get() + 1)),
        ))
        .gap(8.0),
    ))
    .gap(16.0)
}}

fn main() {{
    App::new()
        .title("{title}")
        .size(400, 300)
        .theme(Theme::light())
        .run(root);
}}
"#
    );

    std::fs::write(project_dir.join("Cargo.toml"), cargo_toml)?;
    std::fs::write(project_dir.join("src").join("main.rs"), main_rs)?;

    eprintln!("Created new vitreous project: {name}");
    eprintln!();
    eprintln!("  cd {name}");
    eprintln!("  cargo run");

    Ok(())
}

fn to_title_case(s: &str) -> String {
    s.split(['-', '_'])
        .filter(|w| !w.is_empty())
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                Some(first) => {
                    let upper: String = first.to_uppercase().collect();
                    format!("{upper}{rest}", rest = chars.as_str())
                }
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn title_case_hyphenated() {
        assert_eq!(to_title_case("my-cool-app"), "My Cool App");
    }

    #[test]
    fn title_case_underscored() {
        assert_eq!(to_title_case("my_app"), "My App");
    }

    #[test]
    fn title_case_single_word() {
        assert_eq!(to_title_case("hello"), "Hello");
    }
}
