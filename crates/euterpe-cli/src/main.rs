use anyhow::{Context, Result, bail};
use serde_json::{Map, Value, json};
use std::{
    env, fs,
    io::{self, Write},
    path::{Path, PathBuf},
    process::Command,
};

fn main() -> Result<()> {
    let mut args = env::args().skip(1);
    match args.next().as_deref() {
        Some("install") | None => install(),
        Some("init") => init(
            args.next()
                .map(PathBuf::from)
                .unwrap_or(env::current_dir()?.join("Euterpe")),
        ),
        Some("uninstall") => uninstall(),
        Some("help") | Some("--help") | Some("-h") => {
            usage();
            Ok(())
        }
        Some(other) => {
            usage();
            bail!("unknown command: {other}")
        }
    }
}

fn usage() {
    println!(
        "Euterpe installer\n\nUsage:\n  euterpe install     Install server and optionally configure coding agents\n  euterpe init [DIR]  Create an Euterpe data project\n  euterpe uninstall   Remove the user installation and service"
    );
}

fn home() -> Result<PathBuf> {
    env::var_os("HOME")
        .map(PathBuf::from)
        .context("HOME is not set")
}
fn prompt(question: &str) -> Result<bool> {
    print!("{question} [y/N] ");
    io::stdout().flush()?;
    let mut answer = String::new();
    io::stdin().read_line(&mut answer)?;
    Ok(matches!(
        answer.trim().to_ascii_lowercase().as_str(),
        "y" | "yes" | "s" | "sim"
    ))
}

fn install() -> Result<()> {
    let home = home()?;
    let current = env::current_exe()?.canonicalize()?;
    let server = locate_server(&current).context(
        "could not find euterpe-mcp next to this CLI; install both release binaries together",
    )?;
    let app_dir = if cfg!(target_os = "macos") {
        home.join("Applications/Euterpe.app/Contents/MacOS")
    } else {
        home.join(".local/lib/euterpe")
    };
    let resources = if cfg!(target_os = "macos") {
        home.join("Applications/Euterpe.app/Contents/Resources")
    } else {
        home.join(".local/share/euterpe")
    };
    fs::create_dir_all(&app_dir)?;
    fs::create_dir_all(resources.join("models"))?;
    let installed_server = app_dir.join("euterpe-mcp");
    copy_executable(&server, &installed_server)?;
    if cfg!(target_os = "macos") {
        fs::write(
            home.join("Applications/Euterpe.app/Contents/Info.plist"),
            plist_info(),
        )?;
        fs::write(
            resources.join("Euterpe.icns"),
            include_bytes!("../../../assets/euterpe.icns"),
        )?;
    }
    if let Some(model) = locate_model(&current) {
        fs::copy(model, resources.join("models/ggml-base.bin")).context("copy Whisper model")?;
    }
    let data = if cfg!(target_os = "macos") {
        home.join("Library/Application Support/Euterpe")
    } else {
        home.join(".local/share/euterpe-data")
    };
    fs::create_dir_all(data.join("operations"))?;
    let roots = env::var("MCP_AUDIO_ALLOWED_ROOTS").unwrap_or_else(|_| home.display().to_string());
    install_service(&home, &installed_server, &data, &resources, &roots)?;
    println!("Installed Euterpe at {}", app_dir.display());
    if prompt("Configure Euterpe for Codex?")? {
        configure_codex(&home, &installed_server, &data, &resources, &roots)?;
    }
    if prompt("Configure Euterpe for Claude?")? {
        configure_claude(&home, &installed_server, &data, &resources, &roots)?;
    }
    println!("Installation complete. Restart the selected coding agents to load Euterpe.");
    Ok(())
}

fn locate_server(cli: &Path) -> Option<PathBuf> {
    let name = if cfg!(windows) {
        "euterpe-mcp.exe"
    } else {
        "euterpe-mcp"
    };
    [
        cli.parent()?.join(name),
        cli.parent()?.parent()?.join("euterpe-mcp"),
    ]
    .into_iter()
    .find(|p| p.is_file())
}
fn locate_model(cli: &Path) -> Option<PathBuf> {
    let release_dir = cli.parent()?;
    let target_dir = release_dir.parent()?;
    let project_dir = target_dir.parent()?;
    [
        project_dir.join("models/ggml-base.bin"),
        release_dir.join("models/ggml-base.bin"),
    ]
    .into_iter()
    .find(|p| p.is_file())
}
fn copy_executable(src: &Path, dst: &Path) -> Result<()> {
    fs::copy(src, dst)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(dst, fs::Permissions::from_mode(0o755))?;
    }
    Ok(())
}
fn service_env(data: &Path, resources: &Path, roots: &str) -> Vec<(String, String)> {
    vec![
        ("MCP_AUDIO_DATA_DIR".into(), data.display().to_string()),
        (
            "MCP_AUDIO_MODEL".into(),
            resources.join("models/ggml-base.bin").display().to_string(),
        ),
        ("MCP_AUDIO_ALLOWED_ROOTS".into(), roots.into()),
    ]
}
fn install_service(
    home: &Path,
    server: &Path,
    data: &Path,
    resources: &Path,
    roots: &str,
) -> Result<()> {
    if cfg!(target_os = "macos") {
        let agents = home.join("Library/LaunchAgents");
        fs::create_dir_all(&agents)?;
        let env_xml = service_env(data, resources, roots)
            .iter()
            .map(|(k, v)| format!("<key>{}</key><string>{}</string>", xml(k), xml(v)))
            .collect::<String>();
        let plist = format!(
            "<?xml version=\"1.0\" encoding=\"UTF-8\"?><!DOCTYPE plist PUBLIC \"-//Apple//DTD PLIST 1.0//EN\" \"http://www.apple.com/DTDs/PropertyList-1.0.dtd\"><plist version=\"1.0\"><dict><key>Label</key><string>com.euterpe.mcp</string><key>ProgramArguments</key><array><string>{}</string></array><key>EnvironmentVariables</key><dict>{}</dict><key>RunAtLoad</key><true/><key>KeepAlive</key><false/><key>StandardErrorPath</key><string>{}</string></dict></plist>",
            xml(&server.display().to_string()),
            env_xml,
            xml(&data.join("euterpe.log").display().to_string())
        );
        fs::write(agents.join("com.euterpe.mcp.plist"), plist)?;
        let _ = Command::new("launchctl")
            .args([
                "bootstrap",
                &format!("gui/{}", unsafe { libc_uid() }),
                agents.join("com.euterpe.mcp.plist").to_str().unwrap_or(""),
            ])
            .status();
    }
    Ok(())
}
#[cfg(unix)]
fn libc_uid() -> u32 {
    unsafe { getuid() }
}
#[cfg(unix)]
unsafe extern "C" {
    fn getuid() -> u32;
}
#[cfg(not(unix))]
fn libc_uid() -> u32 {
    0
}
fn xml(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}
fn plist_info() -> &'static str {
    "<?xml version=\"1.0\" encoding=\"UTF-8\"?><plist version=\"1.0\"><dict><key>CFBundleName</key><string>Euterpe</string><key>CFBundleIdentifier</key><string>com.euterpe.mcp</string><key>CFBundlePackageType</key><string>APPL</string><key>CFBundleVersion</key><string>0.1.0</string><key>CFBundleIconFile</key><string>Euterpe</string></dict></plist>"
}

fn init(dir: PathBuf) -> Result<()> {
    fs::create_dir_all(dir.join("models"))?;
    fs::create_dir_all(dir.join("runtime/operations"))?;
    fs::write(
        dir.join("README.md"),
        "# Euterpe project\n\nPlace media files in this directory and configure it as an allowed input root. Put the Whisper GGML model in `models/ggml-base.bin`.\n",
    )?;
    println!("Created Euterpe project at {}", dir.display());
    Ok(())
}

fn configure_codex(
    home: &Path,
    server: &Path,
    data: &Path,
    resources: &Path,
    roots: &str,
) -> Result<()> {
    let config = home.join(".codex/config.toml");
    fs::create_dir_all(config.parent().unwrap())?;
    let old = fs::read_to_string(&config).unwrap_or_default();
    let block = format!(
        "[mcp_servers.euterpe]\ncommand = {}\nargs = []\n\n[mcp_servers.euterpe.env]\nMCP_AUDIO_DATA_DIR = {}\nMCP_AUDIO_MODEL = {}\nMCP_AUDIO_ALLOWED_ROOTS = {}\n",
        toml_string(&server.display().to_string()),
        toml_string(&data.display().to_string()),
        toml_string(&resources.join("models/ggml-base.bin").display().to_string()),
        toml_string(roots)
    );
    let updated = replace_toml_table(&old, "mcp_servers.euterpe", &block);
    fs::write(config, updated)?;
    println!("Updated Codex config.");
    Ok(())
}
fn toml_string(s: &str) -> String {
    format!(
        "\"{}\"",
        s.replace('\\', "\\\\")
            .replace('"', "\\\"")
            .replace('\n', "\\n")
    )
}
fn replace_toml_table(old: &str, key: &str, replacement: &str) -> String {
    let mut out = String::new();
    let mut skipping = false;
    let mut replaced = false;
    for line in old.lines() {
        if line.trim_start().starts_with('[') {
            if skipping {
                skipping = false;
            }
            let current = line.trim().trim_start_matches('[').trim_end_matches(']');
            if current == key || current.starts_with(&format!("{key}.")) {
                if !replaced {
                    out.push_str(replacement);
                    replaced = true;
                }
                skipping = true;
                continue;
            }
        }
        if !skipping {
            out.push_str(line);
            out.push('\n');
        }
    }
    if !replaced {
        if !out.trim().is_empty() {
            out.push('\n');
        }
        out.push_str(replacement);
    }
    out
}
fn configure_claude(
    home: &Path,
    server: &Path,
    data: &Path,
    resources: &Path,
    roots: &str,
) -> Result<()> {
    // Claude Code stores user-scoped MCP servers in ~/.claude.json across platforms.
    let config = home.join(".claude.json");
    fs::create_dir_all(config.parent().unwrap())?;
    let mut root: Value = if config.exists() {
        serde_json::from_slice(&fs::read(&config)?).context("parse Claude config JSON")?
    } else {
        json!({})
    };
    if !root.is_object() {
        root = json!({});
    }
    let root = root.as_object_mut().unwrap();
    let servers = root.entry("mcpServers").or_insert_with(|| json!({}));
    if !servers.is_object() {
        *servers = json!({});
    }
    let mut env_map = Map::new();
    for (k, v) in service_env(data, resources, roots) {
        env_map.insert(k, Value::String(v));
    }
    servers.as_object_mut().unwrap().insert(
        "euterpe".into(),
        json!({"command": server, "args": [], "env": env_map}),
    );
    fs::write(config, serde_json::to_vec_pretty(&root)?)?;
    println!("Updated Claude config.");
    Ok(())
}
fn uninstall() -> Result<()> {
    let home = home()?;
    if cfg!(target_os = "macos") {
        let plist = home.join("Library/LaunchAgents/com.euterpe.mcp.plist");
        let _ = Command::new("launchctl")
            .args([
                "bootout",
                &format!("gui/{}", unsafe { libc_uid() }),
                plist.to_str().unwrap_or(""),
            ])
            .status();
        let _ = fs::remove_file(plist);
        let _ = fs::remove_dir_all(home.join("Applications/Euterpe.app"));
    } else {
        let _ = fs::remove_dir_all(home.join(".local/lib/euterpe"));
    }
    println!(
        "Removed Euterpe application and user service. Agent configuration and user data were preserved."
    );
    Ok(())
}
