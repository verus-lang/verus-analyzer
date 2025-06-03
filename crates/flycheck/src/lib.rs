//! Flycheck provides the functionality needed to run `cargo check` or
//! another compatible command (f.x. clippy) in a background thread and provide
//! LSP diagnostics based on the output of the command.

// FIXME: This crate now handles running `cargo test` needed in the test explorer in
// addition to `cargo check`. Either split it into 3 crates (one for test, one for check
// and one common utilities) or change its name and docs to reflect the current state.

#![warn(rust_2018_idioms, unused_lifetimes)]

use std::{fmt, io, path::Path, process::Command, time::Duration};

use crossbeam_channel::{never, select, unbounded, Receiver, Sender};
use paths::{AbsPath, AbsPathBuf, Utf8PathBuf};
use rustc_hash::FxHashMap;
use serde::Deserialize;

pub use cargo_metadata::diagnostic::{
    Applicability, Diagnostic, DiagnosticCode, DiagnosticLevel, DiagnosticSpan,
    DiagnosticSpanMacroExpansion,
};
use toolchain::Tool;

mod command;
mod test_runner;

use command::{CommandHandle, ParseFromLine};
pub use test_runner::{CargoTestHandle, CargoTestMessage, TestState};

#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
pub enum InvocationStrategy {
    Once,
    #[default]
    PerWorkspace,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum InvocationLocation {
    Root(AbsPathBuf),
    #[default]
    Workspace,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CargoOptions {
    pub target_triples: Vec<String>,
    pub all_targets: bool,
    pub no_default_features: bool,
    pub all_features: bool,
    pub features: Vec<String>,
    pub extra_args: Vec<String>,
    pub extra_env: FxHashMap<String, String>,
    pub target_dir: Option<Utf8PathBuf>,
}

impl CargoOptions {
    fn apply_on_command(&self, cmd: &mut Command) {
        for target in &self.target_triples {
            cmd.args(["--target", target.as_str()]);
        }
        if self.all_targets {
            cmd.arg("--all-targets");
        }
        if self.all_features {
            cmd.arg("--all-features");
        } else {
            if self.no_default_features {
                cmd.arg("--no-default-features");
            }
            if !self.features.is_empty() {
                cmd.arg("--features");
                cmd.arg(self.features.join(" "));
            }
        }
        if let Some(target_dir) = &self.target_dir {
            cmd.arg("--target-dir").arg(target_dir);
        }
        cmd.envs(&self.extra_env);
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FlycheckConfig {
    CargoCommand {
        command: String,
        options: CargoOptions,
        ansi_color_output: bool,
    },
    CustomCommand {
        command: String,
        args: Vec<String>,
        extra_env: FxHashMap<String, String>,
        invocation_strategy: InvocationStrategy,
        invocation_location: InvocationLocation,
    },
    VerusCommand {
        args: Vec<String>,
    },
}

impl fmt::Display for FlycheckConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FlycheckConfig::CargoCommand { command, .. } => write!(f, "cargo {command}"),
            FlycheckConfig::CustomCommand { command, args, .. } => {
                write!(f, "{command} {}", args.join(" "))
            }
            FlycheckConfig::VerusCommand { args } => write!(f, "verus {}", args.join(" ")),
        }
    }
}

/// Flycheck wraps the shared state and communication machinery used for
/// running `cargo check` (or other compatible command) and providing
/// diagnostics based on the output.
/// The spawned thread is shut down when this struct is dropped.
#[derive(Debug)]
pub struct FlycheckHandle {
    // XXX: drop order is significant
    sender: Sender<StateChange>,
    _thread: stdx::thread::JoinHandle,
    id: usize,
}

impl FlycheckHandle {
    pub fn spawn(
        id: usize,
        sender: Box<dyn Fn(Message) + Send>,
        config: FlycheckConfig,
        sysroot_root: Option<AbsPathBuf>,
        workspace_root: AbsPathBuf,
        manifest_path: Option<AbsPathBuf>,
    ) -> FlycheckHandle {
        let actor =
            FlycheckActor::new(id, sender, config, sysroot_root, workspace_root, manifest_path);
        let (sender, receiver) = unbounded::<StateChange>();
        let thread = stdx::thread::Builder::new(stdx::thread::ThreadIntent::Worker)
            .name("Flycheck".to_owned())
            .spawn(move || actor.run(receiver))
            .expect("failed to spawn thread");
        FlycheckHandle { id, sender, _thread: thread }
    }

    /// Schedule a re-start of the cargo check worker to do a workspace wide check.
    pub fn restart_workspace(&self, saved_file: Option<AbsPathBuf>) {
        self.sender.send(StateChange::Restart { package: None, saved_file }).unwrap();
    }

    /// Schedule a re-start of the cargo check worker to do a package wide check.
    pub fn restart_for_package(&self, package: String) {
        self.sender
            .send(StateChange::Restart { package: Some(package), saved_file: None })
            .unwrap();
    }

    /// Schedule a re-start of the cargo check worker.
    pub fn restart_verus(&self, file: String) {
        tracing::debug!("restart verus for {:?}", file);
        self.sender.send(StateChange::RestartVerus(file)).unwrap();
    }

    /// Stop this cargo check worker.
    pub fn cancel(&self) {
        self.sender.send(StateChange::Cancel).unwrap();
    }

    pub fn id(&self) -> usize {
        self.id
    }
}

pub enum Message {
    /// Request adding a diagnostic with fixes included to a file
    AddDiagnostic { id: usize, workspace_root: AbsPathBuf, diagnostic: Diagnostic },

    /// Request clearing all previous diagnostics
    ClearDiagnostics { id: usize },

    /// Request check progress notification to client
    Progress {
        /// Flycheck instance ID
        id: usize,
        progress: Progress,
    },
}

impl fmt::Debug for Message {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Message::AddDiagnostic { id, workspace_root, diagnostic } => f
                .debug_struct("AddDiagnostic")
                .field("id", id)
                .field("workspace_root", workspace_root)
                .field("diagnostic_code", &diagnostic.code.as_ref().map(|it| &it.code))
                .finish(),
            Message::ClearDiagnostics { id } => {
                f.debug_struct("ClearDiagnostics").field("id", id).finish()
            }
            Message::Progress { id, progress } => {
                f.debug_struct("Progress").field("id", id).field("progress", progress).finish()
            }
        }
    }
}

#[derive(Debug)]
pub enum Progress {
    DidStart,
    DidCheckCrate(String),
    DidFinish(io::Result<()>),
    DidCancel,
    DidFailToRestart(String),
    VerusResult(String),
}

enum StateChange {
    Restart { package: Option<String>, saved_file: Option<AbsPathBuf> },
    Cancel,
    RestartVerus(String),
}

/// A [`FlycheckActor`] is a single check instance of a workspace.
struct FlycheckActor {
    /// The workspace id of this flycheck instance.
    id: usize,
    sender: Box<dyn Fn(Message) + Send>,
    config: FlycheckConfig,
    manifest_path: Option<AbsPathBuf>,
    /// Either the workspace root of the workspace we are flychecking,
    /// or the project root of the project.
    root: AbsPathBuf,
    sysroot_root: Option<AbsPathBuf>,
    /// CargoHandle exists to wrap around the communication needed to be able to
    /// run `cargo check` without blocking. Currently the Rust standard library
    /// doesn't provide a way to read sub-process output without blocking, so we
    /// have to wrap sub-processes output handling in a thread and pass messages
    /// back over a channel.
    command_handle: Option<CommandHandle<CargoCheckMessage>>,
    /// The receiver side of the channel mentioned above.
    command_receiver: Option<Receiver<CargoCheckMessage>>,

    status: FlycheckStatus,
}

enum Event {
    RequestStateChange(StateChange),
    CheckEvent(Option<CargoCheckMessage>),
}

#[derive(PartialEq)]
enum FlycheckStatus {
    Started,
    DiagnosticSent,
    Finished,
}

const SAVED_FILE_PLACEHOLDER: &str = "$saved_file";

impl FlycheckActor {
    fn new(
        id: usize,
        sender: Box<dyn Fn(Message) + Send>,
        config: FlycheckConfig,
        sysroot_root: Option<AbsPathBuf>,
        workspace_root: AbsPathBuf,
        manifest_path: Option<AbsPathBuf>,
    ) -> FlycheckActor {
        tracing::info!(%id, ?workspace_root, "Spawning flycheck");
        FlycheckActor {
            id,
            sender,
            config,
            sysroot_root,
            root: workspace_root,
            manifest_path,
            command_handle: None,
            command_receiver: None,
            status: FlycheckStatus::Finished,
        }
    }

    fn report_progress(&self, progress: Progress) {
        self.send(Message::Progress { id: self.id, progress });
    }

    fn next_event(&self, inbox: &Receiver<StateChange>) -> Option<Event> {
        if let Ok(msg) = inbox.try_recv() {
            // give restarts a preference so check outputs don't block a restart or stop
            return Some(Event::RequestStateChange(msg));
        }
        select! {
            recv(inbox) -> msg => msg.ok().map(Event::RequestStateChange),
            recv(self.command_receiver.as_ref().unwrap_or(&never())) -> msg => Some(Event::CheckEvent(msg.ok())),
        }
    }

    fn run(mut self, inbox: Receiver<StateChange>) {
        'event: while let Some(event) = self.next_event(&inbox) {
            match event {
                Event::RequestStateChange(StateChange::Cancel) => {
                    tracing::debug!(flycheck_id = self.id, "flycheck cancelled");
                    self.cancel_check_process();
                }
                Event::RequestStateChange(StateChange::Restart { package, saved_file }) => {
                    // Cancel the previously spawned process
                    self.cancel_check_process();
                    while let Ok(restart) = inbox.recv_timeout(Duration::from_millis(50)) {
                        // restart chained with a stop, so just cancel
                        if let StateChange::Cancel = restart {
                            continue 'event;
                        }
                    }

                    let command =
                        match self.check_command(package.as_deref(), saved_file.as_deref()) {
                            Some(c) => c,
                            None => continue,
                        };
                    let formatted_command = format!("{command:?}");

                    tracing::debug!(?command, "will restart flycheck");
                    let (sender, receiver) = unbounded();
                    match CommandHandle::spawn(command, sender) {
                        Ok(command_handle) => {
                            tracing::debug!(command = formatted_command, "did restart flycheck");
                            self.command_handle = Some(command_handle);
                            self.command_receiver = Some(receiver);
                            self.report_progress(Progress::DidStart);
                            self.status = FlycheckStatus::Started;
                        }
                        Err(error) => {
                            self.report_progress(Progress::DidFailToRestart(format!(
                                "Failed to run the following command: {formatted_command} error={error}"
                            )));
                            self.status = FlycheckStatus::Finished;
                        }
                    }
                }
                Event::RequestStateChange(StateChange::RestartVerus(filename)) => {
                    // verus: copied from above `Event::RequestStateChange(StateChange::Restart)`
                    // Cancel the previously spawned process
                    self.cancel_check_process();
                    while let Ok(restart) = inbox.recv_timeout(Duration::from_millis(50)) {
                        // restart chained with a stop, so just cancel
                        if let StateChange::Cancel = restart {
                            continue 'event;
                        }
                    }

                    let command = self.run_verus(filename.clone());
                    let formatted_command = format!("{command:?}");
                    tracing::info!(?command, "will restart flycheck");
                    let (sender, receiver) = unbounded();
                    match CommandHandle::spawn(command, sender) {
                        Ok(command_handle) => {
                            self.command_handle = Some(command_handle);
                            self.command_receiver = Some(receiver);
                            // self.report_progress(Progress::VerusResult(format!(
                            //     //"Started running the following Verus command: {:?}",
                            //     "Running Verus...",
                            //     //&formatted_command,
                            // )));
                            self.report_progress(Progress::DidStart); // this is important -- otherwise, previous diagnostic does not disappear
                            self.status = FlycheckStatus::Started;
                        }
                        Err(error) => {
                            self.report_progress(Progress::DidFailToRestart(format!(
                                "Failed to run the following command: {formatted_command} error={error}"
                            )));
                            self.status = FlycheckStatus::Finished;
                        }
                    }
                }
                Event::CheckEvent(None) => {
                    tracing::debug!(flycheck_id = self.id, "flycheck finished");

                    // Watcher finished
                    let command_handle = self.command_handle.take().unwrap();
                    self.command_receiver.take();
                    let formatted_handle = format!("{command_handle:?}");

                    let res = command_handle.join();
                    if let Err(error) = &res {
                        tracing::error!(
                            "Flycheck failed to run the following command: {}, error={}",
                            formatted_handle,
                            error
                        );
                    }
                    if self.status == FlycheckStatus::Started {
                        self.send(Message::ClearDiagnostics { id: self.id });
                    }
                    self.report_progress(Progress::DidFinish(res));
                    self.status = FlycheckStatus::Finished;
                }
                Event::CheckEvent(Some(message)) => match message {
                    CargoCheckMessage::CompilerArtifact(msg) => {
                        tracing::trace!(
                            flycheck_id = self.id,
                            artifact = msg.target.name,
                            "artifact received"
                        );
                        self.report_progress(Progress::DidCheckCrate(msg.target.name));
                    }

                    CargoCheckMessage::Diagnostic(msg) => {
                        tracing::trace!(
                            flycheck_id = self.id,
                            message = msg.message,
                            "diagnostic received"
                        );
                        if self.status == FlycheckStatus::Started {
                            self.send(Message::ClearDiagnostics { id: self.id });
                        }
                        self.send(Message::AddDiagnostic {
                            id: self.id,
                            workspace_root: self.root.clone(),
                            diagnostic: msg,
                        });
                        self.status = FlycheckStatus::DiagnosticSent;
                    }
                    CargoCheckMessage::VerusResult(res) => {
                        self.report_progress(Progress::VerusResult(res));
                    }
                },
            }
        }
        // If we rerun the thread, we need to discard the previous check results first
        self.cancel_check_process();
    }

    fn cancel_check_process(&mut self) {
        if let Some(command_handle) = self.command_handle.take() {
            tracing::debug!(
                command = ?command_handle,
                "did  cancel flycheck"
            );
            command_handle.cancel();
            self.command_receiver.take();
            self.report_progress(Progress::DidCancel);
            self.status = FlycheckStatus::Finished;
        }
    }

    /// Construct a `Command` object for checking the user's code. If the user
    /// has specified a custom command with placeholders that we cannot fill,
    /// return None.
    fn check_command(
        &self,
        package: Option<&str>,
        saved_file: Option<&AbsPath>,
    ) -> Option<Command> {
        let (mut cmd, args) = match &self.config {
            FlycheckConfig::CargoCommand { command, options, ansi_color_output } => {
                let mut cmd = Command::new(Tool::Cargo.path());
                if let Some(sysroot_root) = &self.sysroot_root {
                    cmd.env("RUSTUP_TOOLCHAIN", AsRef::<std::path::Path>::as_ref(sysroot_root));
                }
                cmd.arg(command);
                cmd.current_dir(&self.root);

                match package {
                    Some(pkg) => cmd.arg("-p").arg(pkg),
                    None => cmd.arg("--workspace"),
                };

                cmd.arg(if *ansi_color_output {
                    "--message-format=json-diagnostic-rendered-ansi"
                } else {
                    "--message-format=json"
                });

                if let Some(manifest_path) = &self.manifest_path {
                    cmd.arg("--manifest-path");
                    cmd.arg(manifest_path);
                    if manifest_path.extension().map_or(false, |ext| ext == "rs") {
                        cmd.arg("-Zscript");
                    }
                }

                options.apply_on_command(&mut cmd);
                (cmd, options.extra_args.clone())
            }
            FlycheckConfig::CustomCommand {
                command,
                args,
                extra_env,
                invocation_strategy,
                invocation_location,
            } => {
                let mut cmd = Command::new(command);
                cmd.envs(extra_env);

                match invocation_location {
                    InvocationLocation::Workspace => {
                        match invocation_strategy {
                            InvocationStrategy::Once => {
                                cmd.current_dir(&self.root);
                            }
                            InvocationStrategy::PerWorkspace => {
                                // FIXME: cmd.current_dir(&affected_workspace);
                                cmd.current_dir(&self.root);
                            }
                        }
                    }
                    InvocationLocation::Root(root) => {
                        cmd.current_dir(root);
                    }
                }

                if args.contains(&SAVED_FILE_PLACEHOLDER.to_owned()) {
                    // If the custom command has a $saved_file placeholder, and
                    // we're saving a file, replace the placeholder in the arguments.
                    if let Some(saved_file) = saved_file {
                        let args = args
                            .iter()
                            .map(|arg| {
                                if arg == SAVED_FILE_PLACEHOLDER {
                                    saved_file.to_string()
                                } else {
                                    arg.clone()
                                }
                            })
                            .collect();
                        (cmd, args)
                    } else {
                        // The custom command has a $saved_file placeholder,
                        // but we had an IDE event that wasn't a file save. Do nothing.
                        return None;
                    }
                } else {
                    (cmd, args.clone())
                }
            }
            FlycheckConfig::VerusCommand { args: _ } => {
                return None;
            } // Verus doesn't have a check mode (yet)
        };

        cmd.args(args);
        Some(cmd)
    }

    // copied from above check_command
    fn run_verus(&self, file: String) -> Command {
        let (mut cmd, args) = match &self.config {
            FlycheckConfig::CargoCommand { .. } => {
                panic!("verus analyzer does not yet support cargo commands")
            }
            FlycheckConfig::CustomCommand { .. } => {
                panic!("verus analyzer does not yet support custom commands")
            }
            FlycheckConfig::VerusCommand { args } => {
                let verus_binary_str = match std::env::var("VERUS_BINARY_PATH") {
                    Ok(path) => path,
                    Err(_) => {
                        tracing::warn!("VERUS_BINARY_PATH was not set!");
                        "verus".to_string() // Hope that it's in the PATH
                    }
                };
                dbg!(&verus_binary_str);
                tracing::info!("Using Verus binary: {}", &verus_binary_str);

                let verus_exec_path = Path::new(&verus_binary_str)
                    .canonicalize()
                    .expect("We expect to succeed with canonicalizing the Verus binary path");
                let mut cmd = Command::new(verus_exec_path);

                // Try to locate a Cargo.toml file that might contain custom Verus arguments
                let file = Path::new(&file);
                let mut toml_dir: Option<std::path::PathBuf> = None;
                let mut extra_args_from_toml = Vec::new();
                for ans in file.ancestors() {
                    if ans.join("Cargo.toml").exists() {
                        let toml = std::fs::read_to_string(ans.join("Cargo.toml")).unwrap();
                        let mut found_verus_settings = false;
                        for line in toml.lines() {
                            if found_verus_settings {
                                if line.contains("extra_args") {
                                    let start = "extra_args".len() + 1;
                                    let mut arguments =
                                        line[start..line.len() - 1].trim().to_string();
                                    if arguments.starts_with("=") {
                                        arguments.remove(0);
                                        arguments = arguments.trim().to_string();
                                    }
                                    if arguments.starts_with("\"") {
                                        arguments.remove(0);
                                    }
                                    if arguments.ends_with("\"") {
                                        arguments.remove(arguments.len() - 1);
                                    }

                                    let arguments_vec = arguments
                                        .split(" ")
                                        .map(|it| it.to_string())
                                        .collect::<Vec<_>>();
                                    extra_args_from_toml.extend(arguments_vec);
                                }
                                break;
                            }
                            if line.contains("[package.metadata.verus.ide]") {
                                found_verus_settings = true;
                            }
                        }
                        toml_dir = Some(ans.to_path_buf());
                        break;
                    }
                }

                // We may need to add additional arguments
                let mut args = args.to_vec();
                match toml_dir {
                    None => {
                        // This file doesn't appear to be part of a larger project
                        // Try to invoke Verus on it directly, but try to avoid
                        // complaints about missing `fn main()`
                        args.push("--crate-type".to_string());
                        args.push("lib".to_string());
                    }
                    Some(toml_dir) => {
                        // This file appears to be part of a Rust project.
                        // If it's not the root file, then we need to
                        // invoke Verus on the root file and then filter for results in the current file
                        let root_file = if toml_dir.join("src").join("main.rs").exists() {
                            Some(toml_dir.join("src").join("main.rs"))
                        } else if toml_dir.join("src").join("lib.rs").exists() {
                            args.push("--crate-type".to_string());
                            args.push("lib".to_string());
                            Some(toml_dir.join("src").join("lib.rs"))
                        } else {
                            None
                        };

                        match root_file {
                            Some(root_file) => {
                                let file_as_module = file
                                    .strip_prefix(toml_dir.join("src"))
                                    .unwrap()
                                    .to_str()
                                    .unwrap()
                                    .replace(std::path::MAIN_SEPARATOR_STR, "::")
                                    .replace(".rs", "")
                                    // Trimming `::mod` instead of trimming `mod` and conditionally
                                    // checking for a `::` before it. This works because a `mod.rs`
                                    // file at the source root can define a module called `mod`. 
                                    .trim_end_matches("::mod").to_string()
                                ;

                                args.insert(0, root_file.to_str().unwrap().to_string());
                                if file == root_file {
                                    tracing::info!("file == root_file");
                                } else {
                                    tracing::info!(?root_file, "root_file");
                                    args.insert(1, "--verify-module".to_string());
                                    args.insert(2, file_as_module);
                                }
                            }
                            None => {
                                // Puzzling -- we found a Cargo.toml but no root file.
                                // Do our best by trying to run directly on the file supplied
                                args.insert(0, file.to_str().unwrap().to_string());
                                args.push("--crate-type".to_string());
                                args.push("lib".to_string());
                            }
                        }
                    }
                }

                args.append(&mut extra_args_from_toml);
                args.push("--".to_string());
                args.push("--error-format=json".to_string());

                cmd.current_dir(&self.root);
                (cmd, args)
            }
        };

        cmd.args(args);
        cmd
    }

    fn send(&self, check_task: Message) {
        (self.sender)(check_task);
    }
}

#[allow(clippy::large_enum_variant)]
enum CargoCheckMessage {
    CompilerArtifact(cargo_metadata::Artifact),
    Diagnostic(Diagnostic),
    VerusResult(String),
}

impl ParseFromLine for CargoCheckMessage {
    fn from_line(line: &str, error: &mut String) -> Option<Self> {
        let mut deserializer = serde_json::Deserializer::from_str(line);
        deserializer.disable_recursion_limit();
        if let Ok(message) = JsonMessage::deserialize(&mut deserializer) {
            return match message {
                // Skip certain kinds of messages to only spend time on what's useful
                JsonMessage::Cargo(message) => match message {
                    cargo_metadata::Message::CompilerArtifact(artifact) if !artifact.fresh => {
                        Some(CargoCheckMessage::CompilerArtifact(artifact))
                    }
                    cargo_metadata::Message::CompilerMessage(msg) => {
                        Some(CargoCheckMessage::Diagnostic(msg.message))
                    }
                    _ => None,
                },
                JsonMessage::Rustc(message) => Some(CargoCheckMessage::Diagnostic(message)),
            };
        } else {
            // verus
            // forward verification result if present
            // TODO: We should ask Verus for json output and then parse it properly here
            if line.contains("verification results::") {
                Some(CargoCheckMessage::VerusResult(line.to_string()));
            } else {
                tracing::error!("deserialize error: {:?}", line);
            }
        }

        error.push_str(line);
        error.push('\n');
        None
    }

    fn from_eof() -> Option<Self> {
        None
    }
}

#[derive(Deserialize)]
#[serde(untagged)]
enum JsonMessage {
    Cargo(cargo_metadata::Message),
    Rustc(Diagnostic),
}
