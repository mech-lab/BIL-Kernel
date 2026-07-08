use anyhow::{Context, Result, bail};
use bil_bundle::{BundleBuilder, BundleInspectOptions, BundleReader};
use bil_client::AxleClient;
use bil_core::{AxleArtifactKind, BundleKind, DigestSet, ReceiptMode, SignatureAlgorithm};
use bil_hash::{canonical_json_slice, digest_bytes};
use bil_receipt::{ReceiptIssueOptions, ReceiptIssuer};
use bil_report::{
    AuditReviewReport, RegulatoryReviewReport, render_audit_markdown, render_regulatory_markdown,
    render_verification_markdown, render_verification_sarif,
};
use clap::{ArgAction, Args, Parser, Subcommand, ValueEnum};
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
    Hash(HashArgs),
    Report(ReportArgs),
    Bundle(BundleArgs),
    Receipt(ReceiptArgs),
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
struct HashArgs {
    #[arg(value_name = "PATH")]
    path: String,
    #[arg(long, action = ArgAction::SetTrue)]
    canonical_json: bool,
}

#[derive(Debug, Args)]
struct AxleArgs {
    #[command(subcommand)]
    command: AxleCommand,
}

#[derive(Debug, Args)]
struct BundleArgs {
    #[command(subcommand)]
    command: BundleCommand,
}

#[derive(Debug, Args)]
struct ReceiptArgs {
    #[command(subcommand)]
    command: ReceiptCommand,
}

#[derive(Debug, Args)]
struct ReportArgs {
    #[arg(value_name = "DIR")]
    bundle_path: String,
    #[arg(long, value_enum, default_value_t = ReportKind::Verification)]
    kind: ReportKind,
    #[arg(long, value_enum, default_value_t = ReportFormat::Json)]
    format: ReportFormat,
    #[arg(long, value_name = "FILE")]
    receipt: Option<String>,
    #[arg(long = "trust-key", value_name = "FILE")]
    trust_keys: Vec<String>,
    #[arg(long, action = ArgAction::SetTrue)]
    require_receipt: bool,
    #[arg(long, action = ArgAction::SetTrue)]
    require_trust: bool,
}

#[derive(Debug, Subcommand)]
enum BundleCommand {
    Create(BundleCreateArgs),
    Institutionalize(BundleInstitutionalizeArgs),
    Inspect(BundleInspectArgs),
}

#[derive(Debug, Subcommand)]
enum ReceiptCommand {
    Issue(ReceiptIssueArgs),
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

#[derive(Debug, Args)]
struct BundleCreateArgs {
    #[arg(long, value_name = "FILE")]
    axle: String,
    #[arg(long)]
    axle_kind: String,
    #[arg(long, value_name = "DIR")]
    out: String,
}

#[derive(Debug, Args)]
struct BundleInspectArgs {
    #[arg(value_name = "DIR")]
    bundle_path: String,
    #[arg(long, value_name = "FILE")]
    receipt: Option<String>,
    #[arg(long, value_enum, default_value_t = InspectOutputFormat::Json)]
    format: InspectOutputFormat,
    #[arg(long = "trust-key", value_name = "FILE")]
    trust_keys: Vec<String>,
    #[arg(long, action = ArgAction::SetTrue)]
    require_receipt: bool,
    #[arg(long, action = ArgAction::SetTrue)]
    require_trust: bool,
}

#[derive(Debug, Args)]
struct BundleInstitutionalizeArgs {
    #[arg(value_name = "DIR")]
    bundle_path: String,
    #[arg(long, value_name = "FILE")]
    institutional: String,
    #[arg(long, value_name = "FILE")]
    risk: String,
    #[arg(long, value_name = "FILE")]
    controls: String,
}

#[derive(Debug, Args)]
struct ReceiptIssueArgs {
    #[arg(value_name = "DIR")]
    bundle_path: String,
    #[arg(long, value_enum)]
    mode: ReceiptModeArg,
    #[arg(long, value_enum)]
    algorithm: SignatureAlgorithmArg,
    #[arg(long, value_name = "FILE")]
    private_key: String,
    #[arg(long)]
    issued_at: Option<String>,
    #[arg(long, value_name = "FILE")]
    out: Option<String>,
}

#[derive(Debug, Args, Default)]
struct CommonArgs {
    #[arg(long, action = ArgAction::SetTrue, conflicts_with = "no_ignore_imports")]
    ignore_imports: bool,
    #[arg(
        long = "no-ignore-imports",
        action = ArgAction::SetTrue,
        conflicts_with = "ignore_imports"
    )]
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
enum InspectOutputFormat {
    Json,
    Markdown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
#[value(rename_all = "kebab-case")]
enum ReportKind {
    Verification,
    Audit,
    Regulatory,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
#[value(rename_all = "kebab-case")]
enum ReportFormat {
    Json,
    Markdown,
    Sarif,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
#[value(rename_all = "kebab-case")]
enum ReceiptModeArg {
    Embedded,
    Detached,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
#[value(rename_all = "kebab-case")]
enum SignatureAlgorithmArg {
    Ed25519,
    EcdsaP256Sha256,
    RsaPssSha256,
}

impl From<ReceiptModeArg> for ReceiptMode {
    fn from(value: ReceiptModeArg) -> Self {
        match value {
            ReceiptModeArg::Embedded => ReceiptMode::Embedded,
            ReceiptModeArg::Detached => ReceiptMode::Detached,
        }
    }
}

impl From<SignatureAlgorithmArg> for SignatureAlgorithm {
    fn from(value: SignatureAlgorithmArg) -> Self {
        match value {
            SignatureAlgorithmArg::Ed25519 => SignatureAlgorithm::Ed25519,
            SignatureAlgorithmArg::EcdsaP256Sha256 => SignatureAlgorithm::EcdsaP256Sha256,
            SignatureAlgorithmArg::RsaPssSha256 => SignatureAlgorithm::RsaPssSha256,
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
    match cli.command {
        Command::Status(args) => {
            let client = AxleClient::new(cli.url, None, None, None)?;
            let status = client.check_status(args.timeout_seconds).await?;
            print_json(&status)?;
            Ok(ExitCode::SUCCESS)
        }
        Command::Environments(args) => {
            let client = AxleClient::new(cli.url, None, None, None)?;
            let environments = client.environments(args.timeout_seconds).await?;
            print_json(&environments)?;
            Ok(ExitCode::SUCCESS)
        }
        Command::Axle(args) => {
            let client = AxleClient::new(cli.url, None, None, None)?;
            run_axle_command(&client, args.command).await
        }
        Command::Hash(args) => {
            let bytes = read_bytes_or_stdin(&args.path)?;
            let effective = if args.canonical_json {
                canonical_json_slice(&bytes)?
            } else {
                bytes
            };
            let output = HashOutput {
                path: args.path,
                canonical_json: args.canonical_json,
                byte_length: effective.len() as u64,
                digests: digest_bytes(&effective),
            };
            print_json(&output)?;
            Ok(ExitCode::SUCCESS)
        }
        Command::Report(args) => run_report_command(args),
        Command::Bundle(args) => run_bundle_command(args.command),
        Command::Receipt(args) => run_receipt_command(args.command),
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

fn run_bundle_command(command: BundleCommand) -> Result<ExitCode> {
    match command {
        BundleCommand::Create(args) => {
            let axle_bytes = fs::read(&args.axle)
                .with_context(|| format!("failed to read AXLE payload from {}", args.axle))?;
            let axle_kind: AxleArtifactKind = args.axle_kind.parse()?;
            let materialized = BundleBuilder::new().create_axle_bundle(
                axle_kind,
                &axle_bytes,
                PathBuf::from(&args.out),
            )?;
            let output = BundleCreateOutput {
                output_dir: materialized.output_dir.display().to_string(),
                bundle_kind: materialized.descriptor.bundle_kind,
                bundle_id: materialized.descriptor.bundle_id,
                payload_count: materialized.manifest.entries.len(),
                merkle_roots: DigestSet {
                    sha256: materialized.merkle.trees.sha256.root,
                    blake3: materialized.merkle.trees.blake3.root,
                },
            };
            print_json(&output)?;
            Ok(ExitCode::SUCCESS)
        }
        BundleCommand::Institutionalize(args) => {
            let institutional_bytes = fs::read(&args.institutional).with_context(|| {
                format!(
                    "failed to read institutional profile input from {}",
                    args.institutional
                )
            })?;
            let risk_bytes = fs::read(&args.risk).with_context(|| {
                format!("failed to read risk registry input from {}", args.risk)
            })?;
            let controls_bytes = fs::read(&args.controls).with_context(|| {
                format!(
                    "failed to read controls registry input from {}",
                    args.controls
                )
            })?;
            let materialized = BundleBuilder::new().institutionalize(
                PathBuf::from(&args.bundle_path),
                &institutional_bytes,
                &risk_bytes,
                &controls_bytes,
            )?;
            let output = BundleInstitutionalizeOutput {
                output_dir: materialized.output_dir.display().to_string(),
                previous_bundle_id: materialized.previous_bundle_id,
                bundle_id: materialized.descriptor.bundle_id,
                institutional_kind: materialized.descriptor.institutional_kind,
                institutional_profile_version: materialized
                    .descriptor
                    .institutional_profile_version,
                payload_count: materialized.manifest.entries.len(),
                merkle_roots: DigestSet {
                    sha256: materialized.merkle.trees.sha256.root,
                    blake3: materialized.merkle.trees.blake3.root,
                },
                external_receipt_notice: materialized.external_receipt_notice,
            };
            print_json(&output)?;
            Ok(ExitCode::SUCCESS)
        }
        BundleCommand::Inspect(args) => {
            let inspection = BundleReader::open(PathBuf::from(&args.bundle_path))?
                .inspect_with_options(&BundleInspectOptions {
                    receipt_path: args.receipt.map(PathBuf::from),
                    trust_key_paths: args.trust_keys.into_iter().map(PathBuf::from).collect(),
                    require_receipt: args.require_receipt,
                    require_trust: args.require_trust,
                })?;
            match args.format {
                InspectOutputFormat::Json => print_json(&inspection)?,
                InspectOutputFormat::Markdown => {
                    print_markdown(&render_verification_markdown(&inspection))
                }
            }
            if inspection.overall_verified {
                Ok(ExitCode::SUCCESS)
            } else {
                Ok(ExitCode::from(2))
            }
        }
    }
}

fn run_report_command(args: ReportArgs) -> Result<ExitCode> {
    if args.kind != ReportKind::Verification && args.format == ReportFormat::Sarif {
        bail!("SARIF output is only supported for verification reports");
    }

    let context = BundleReader::open(PathBuf::from(&args.bundle_path))?
        .report_context_with_options(&BundleInspectOptions {
            receipt_path: args.receipt.map(PathBuf::from),
            trust_key_paths: args.trust_keys.into_iter().map(PathBuf::from).collect(),
            require_receipt: args.require_receipt,
            require_trust: args.require_trust,
        })?;

    let exit = match args.kind {
        ReportKind::Verification => {
            match args.format {
                ReportFormat::Json => print_json(&context.verification)?,
                ReportFormat::Markdown => {
                    print_markdown(&render_verification_markdown(&context.verification))
                }
                ReportFormat::Sarif => {
                    print_json(&render_verification_sarif(&context.verification))?
                }
            }
            if context.verification.overall_verified {
                ExitCode::SUCCESS
            } else {
                ExitCode::from(2)
            }
        }
        ReportKind::Audit => {
            let report = AuditReviewReport::from_context(&context);
            match args.format {
                ReportFormat::Json => print_json(&report)?,
                ReportFormat::Markdown => print_markdown(&render_audit_markdown(&report)),
                ReportFormat::Sarif => unreachable!("validated above"),
            }
            if audit_or_regulatory_success(&context) {
                ExitCode::SUCCESS
            } else {
                ExitCode::from(2)
            }
        }
        ReportKind::Regulatory => {
            let report = RegulatoryReviewReport::from_context(&context);
            match args.format {
                ReportFormat::Json => print_json(&report)?,
                ReportFormat::Markdown => print_markdown(&render_regulatory_markdown(&report)),
                ReportFormat::Sarif => unreachable!("validated above"),
            }
            if audit_or_regulatory_success(&context) {
                ExitCode::SUCCESS
            } else {
                ExitCode::from(2)
            }
        }
    };

    Ok(exit)
}

fn audit_or_regulatory_success(context: &bil_bundle::BundleInspectionContext) -> bool {
    let report = &context.verification;
    report.overall_verified
        && report.institutional_layer_present
        && report.banking_profile_verified
        && report.insurance_profile_verified
        && report.legal_governance_profile_verified
        && report.ai_assurance_profile_verified
        && report.risk_registry_verified
        && report.controls_registry_verified
        && report.cross_profile_consistency_verified
}

fn run_receipt_command(command: ReceiptCommand) -> Result<ExitCode> {
    match command {
        ReceiptCommand::Issue(args) => {
            let private_key_der = fs::read(&args.private_key).with_context(|| {
                format!("failed to read private key DER from {}", args.private_key)
            })?;
            let materialized = ReceiptIssuer::new().issue(
                PathBuf::from(&args.bundle_path),
                &private_key_der,
                ReceiptIssueOptions {
                    mode: args.mode.into(),
                    algorithm: args.algorithm.into(),
                    issued_at: args.issued_at,
                    out: args.out.map(PathBuf::from),
                },
            )?;
            let output = ReceiptIssueOutput {
                receipt_path: materialized.receipt_path.display().to_string(),
                receipt_mode: materialized.document.claims.receipt_mode,
                algorithm: materialized.document.signature.algorithm,
                bundle_id: materialized.document.claims.bundle_id,
                key_id: materialized.document.signature.key_id,
                covered_file_count: materialized.document.claims.covered_files.len(),
            };
            print_json(&output)?;
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

fn read_bytes_or_stdin(path: &str) -> Result<Vec<u8>> {
    if path == "-" {
        let mut buffer = Vec::new();
        io::stdin()
            .read_to_end(&mut buffer)
            .context("failed to read stdin")?;
        return Ok(buffer);
    }

    fs::read(PathBuf::from(path)).with_context(|| format!("failed to read bytes from {path}"))
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

fn print_markdown(markdown: &str) {
    println!("{markdown}");
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

#[derive(Debug, Serialize)]
struct HashOutput {
    path: String,
    canonical_json: bool,
    byte_length: u64,
    digests: DigestSet,
}

#[derive(Debug, Serialize)]
struct BundleCreateOutput {
    output_dir: String,
    bundle_kind: BundleKind,
    bundle_id: String,
    payload_count: usize,
    merkle_roots: DigestSet,
}

#[derive(Debug, Serialize)]
struct BundleInstitutionalizeOutput {
    output_dir: String,
    previous_bundle_id: String,
    bundle_id: String,
    institutional_kind: Option<String>,
    institutional_profile_version: Option<String>,
    payload_count: usize,
    merkle_roots: DigestSet,
    external_receipt_notice: String,
}

#[derive(Debug, Serialize)]
struct ReceiptIssueOutput {
    receipt_path: String,
    receipt_mode: ReceiptMode,
    algorithm: SignatureAlgorithm,
    bundle_id: String,
    key_id: String,
    covered_file_count: usize,
}
