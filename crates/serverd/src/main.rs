//! The serverd binary: open the workspace, serve the doors.

use std::path::PathBuf;
use std::sync::Arc;

use glossql_catalog::Lake;
use glossql_glossary::Store;
use glossql_scripts::RhaiRuntime;
use glossql_serverd::{DoorConfig, Plane, router};

const USAGE: &str = "usage: serverd --workspace <dir> [--functions <dir>] \
[--addr <ip:port>] [--agent <id>] [--human <id>] [--row-cap <n>]";

struct Args {
    workspace: PathBuf,
    functions: Option<PathBuf>,
    addr: String,
    doors: DoorConfig,
}

fn parse(mut argv: std::env::Args) -> Result<Args, String> {
    argv.next();
    let mut workspace = None;
    let mut functions = None;
    let mut addr = "127.0.0.1:8080".to_string();
    let mut doors = DoorConfig::default();
    while let Some(flag) = argv.next() {
        let mut value = || argv.next().ok_or(format!("{flag} needs a value"));
        match flag.as_str() {
            "--workspace" => workspace = Some(PathBuf::from(value()?)),
            "--functions" => functions = Some(PathBuf::from(value()?)),
            "--addr" => addr = value()?,
            "--agent" => doors.agent = value()?,
            "--human" => doors.human = value()?,
            "--row-cap" => {
                doors.row_cap = value()?
                    .parse()
                    .map_err(|e| format!("--row-cap: {e}"))?;
            }
            other => return Err(format!("unknown flag {other}")),
        }
    }
    Ok(Args {
        workspace: workspace.ok_or("--workspace is required")?,
        functions,
        addr,
        doors,
    })
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = parse(std::env::args()).map_err(|e| format!("{e}\n{USAGE}"))?;
    let warehouse = args.workspace.join("warehouse");
    std::fs::create_dir_all(&warehouse)?;

    let store_url = format!(
        "sqlite://{}?mode=rwc",
        args.workspace.join("glossary.sqlite").display()
    );
    let store = Store::open(&store_url).await?;
    let lake = Lake::open(&args.workspace.join("catalog.sqlite"), &warehouse).await?;
    let functions = args
        .functions
        .unwrap_or_else(|| args.workspace.join("functions"));
    let runtime = Arc::new(RhaiRuntime::new(functions));

    let plane = Arc::new(Plane::new(store, Some(lake), runtime));
    let app = router(plane, args.doors);
    let listener = tokio::net::TcpListener::bind(&args.addr).await?;
    println!(
        "serverd on {} — /mcp (agent door), /query (arrow door)",
        args.addr
    );
    axum::serve(listener, app).await?;
    Ok(())
}
