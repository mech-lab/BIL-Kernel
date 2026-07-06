use anyhow::{Context, Result};
use bil_client::AxleClient;
use clap::{ArgAction, Args, Parser, Subcommand};
use serde::Serialize;
use std::fs;
use std::io::{self, Read};
use std::path::PathBuf;
use std::process::ExitCode;

#[derive(Debug, Parser)]
#[command(name = "bil", about = "BIL Kernel CLI")]
struct Cli {
    #[arg(long, global = true)]
    url: Option<String>,
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    Status(StatusArgs),
    Environments(EnvironmentsArgs),
    Axle(AxleArgs),
}

#[derive(Debug, Args)]
struct StatusArgs {
    #[arg(long, default_value_t = 60.0)]
    timeout_seconds: f64,
}

#[derive(Debug, Args)]
struct EnvironmentsArgs {
    #[arg(long)]
    timeout_seconds: Option<f64>,
}

#[derive(Debug, Args)]
struct AxleArgs {
    #[command(subcommand)]
    command: AxleCommand,
}

#[derive(Debug, Subcommand)]
enum AxleCommand {
    VerifyProof(VerifyProofArgs),
    Check(CheckArgs),
    ExtractDecls(ExtractDeclsArgs),
    Normalize(NormalizeArgs),
}

#[derive(Debug, Args)]
struct VerifyProofArgs {
    #[arg(value_name = "CONTENT")]
    content_path: String,
    #[arg(long)]
    formal_statement: String,
    #[arg(long)]
    environment: String,
    #[arg(long = "permitted-sorry")]
    permitted_sorries: Vec<String>,
    #[arg(long, action = ArgAction::SetTrue)]
    mathlib_options: bool,
    #[arg(long, action = ArgAction::SetTrue)]
    use_def_eq: bool,
    #[arg(long, action = ArgAction::SetTrue)]
    strict: bool,
    #[command(flatten)]
    common: CommonArgs,
}

#[derive(Debug, Args)]
struct CheckArgs {
    #[arg(value_name = "CONTENT")]
    content_path: String,
    #[arg(long)]
    environment: String,
    #[arg(long, action = ArgAction::SetTrue)]
    mathlib_options: bool,
    #[arg(long, action = ArgAction::SetTrue)]
    strict: bool,
    #[command(flatten)]
    common: CommonArgs,
}

#[derive(Debug, Args)]
struct ExtractDeclsArgs {
    #[arg(value_name = "CONTENT")]
    content_path: String,
    #[arg(long)]
    environment: String,
    #[command(flatten)]
    common: CommonArgs,
}

#[derive(Debug, Args)]
struct NormalizeArgs {
    #[arg(value_name = "CONTENT")]
    content_path: String,
    #[arg(long)]
    environment: String,
    #[arg(long = "normalization")]
    normalizations: Vec<String>,
    #[arg(long, action = ArgAction::SetTrue, conflicts_with = "no_failsafe")]
    failsafe: bool,
    #[arg(long = "no-failsafe", action = ArgAction::SetTrue, conflicts_with = "failsafe")]
    no_failsafe: bool,
    #[command(flatten)]
    common: CommonArgs,
}

#[derive(Debug, Args, Default)]
struct CommonArgs {
    #[arg(long, action = ArgAction::SetTrue, conflicts_with = "no_ignore_imports")]
    ignore_imports: bool,
    #[arg(long = "no-ignore-imports", action = ArgAction::SetTrue, conflicts_with = "ignore_imports")]
    no_ignore_imports: bool,
    #[arg(long)]
    timeout_seconds: Option<f64>,
}

impl CommonArgs {
    fn ignore_imports_value(&self) -> Option<bool> {
        if self.ignore_imports {
            Some(true)
        } else if self.no_ignore_imports {
            Some(false)
        } else {
            None
        }
    }
}

#[tokio::main]
async fn main() -> ExitCode {
    match run(Cli::parse()).await {
        Ok(code) => code,
        Err(error) => {
            eprintln!("Error: {error:#}");
            ExitCode::from(1)
        }
    }
}

async fn run(cli: Cli) -> Result<ExitCode> {
    let client = AxleClient::new(cli.url, None, None, None)?;
    match cli.command {
        Command::Status(args) => {
            let status = client.check_status(args.timeout_seconds).await?;
            print_json(&status)?;
            Ok(ExitCode::SUCCESS)
        }
        Command::Environments(args) => {
            let environments = client.environments(args.timeout_seconds).await?;
            print_json(&environments)?;
            Ok(ExitCode::SUCCESS)
        }
        Command::Axle(args) => run_axle_command(&client, args.command).await,
    }
}

async fn run_axle_command(client: &AxleClient, command: AxleCommand) -> Result<ExitCode> {
    match command {
        AxleCommand::VerifyProof(args) => {
            let content = read_file_or_stdin(&args.content_path)?;
            let response = client
                .verify_proof(
                    args.formal_statement,
                    content,
                    args.environment,
                    list_option(args.permitted_sorries),
                    bool_option(args.mathlib_options),
                    bool_option(args.use_def_eq),
                    args.common.ignore_imports_value(),
                    args.common.timeout_seconds,
                )
                .await?;
            let exit = if args.strict && !response.okay {
                ExitCode::from(3)
            } else {
                ExitCode::SUCCESS
            };
            print_json(&response)?;
            Ok(exit)
        }
        AxleCommand::Check(args) => {
            let content = read_file_or_stdin(&args.content_path)?;
            let response = client
                .check(
                    content,
                    args.environment,
                    bool_option(args.mathlib_options),
                    args.common.ignore_imports_value(),
                    args.common.timeout_seconds,
                )
                .await?;
            let exit = if args.strict && !response.okay {
                ExitCode::from(3)
            } else {
                ExitCode::SUCCESS
            };
            print_json(&response)?;
            Ok(exit)
        }
        AxleCommand::ExtractDecls(args) => {
            let content = read_file_or_stdin(&args.content_path)?;
            let response = client
                .extract_decls(
                    content,
                    args.environment,
                    args.common.ignore_imports_value(),
                    args.common.timeout_seconds,
                )
                .await?;
            print_json(&response)?;
            Ok(ExitCode::SUCCESS)
        }
        AxleCommand::Normalize(args) => {
            let content = read_file_or_stdin(&args.content_path)?;
            let failsafe = if args.failsafe {
                Some(true)
            } else if args.no_failsafe {
                Some(false)
            } else {
                None
            };
            let response = client
                .normalize(
                    content,
                    args.environment,
                    list_option(args.normalizations),
                    failsafe,
                    args.common.ignore_imports_value(),
                    args.common.timeout_seconds,
                )
                .await?;
            print_json(&response)?;
            Ok(ExitCode::SUCCESS)
        }
    }
}

fn read_file_or_stdin(path: &str) -> Result<String> {
    if path == "-" {
        let mut buffer = String::new();
        io::stdin()
            .read_to_string(&mut buffer)
            .context("failed to read stdin")?;
        return Ok(buffer);
    }

    fs::read_to_string(PathBuf::from(path))
        .with_context(|| format!("failed to read content from {path}"))
}

fn print_json<T>(value: &T) -> Result<()>
where
    T: Serialize,
{
    println!(
        "{}",
        serde_json::to_string_pretty(value).context("failed to serialize CLI output")?
    );
    Ok(())
}

fn list_option(values: Vec<String>) -> Option<Vec<String>> {
    if values.is_empty() {
        None
    } else {
        Some(values)
    }
}

fn bool_option(value: bool) -> Option<bool> {
    if value { Some(true) } else { None }
}
