use clap::Args;
use maggraph::{run_ui_server, Result, UiServerOptions};

use maggraph::ResolvedConfig;

#[derive(Debug, Args)]
pub struct UiArgs {
    /// Host to bind (must be loopback: 127.0.0.1 or ::1)
    #[arg(long, default_value = "127.0.0.1")]
    pub host: String,

    /// TCP port for the dashboard
    #[arg(long, default_value_t = 8787)]
    pub port: u16,

    /// Print the URL and exit without starting the server
    #[arg(long)]
    pub dry_run: bool,
}

#[tracing::instrument(skip_all, fields(host = %args.host, port = args.port))]
pub fn run(resolved: &ResolvedConfig, args: &UiArgs) -> Result<()> {
    let options = UiServerOptions::new(&args.host, args.port, resolved.clone())?;

    if args.dry_run {
        let url = format!("http://{}", options.bind);
        println!("{url}");
        return Ok(());
    }

    let rt = tokio::runtime::Runtime::new().map_err(|e| {
        maggraph::MagGraphError::Index(format!("failed to start async runtime: {e}"))
    })?;

    rt.block_on(run_ui_server(options))
}
