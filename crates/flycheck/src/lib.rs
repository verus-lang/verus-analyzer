//! Flycheck provides the functionality needed to run `cargo check` or
//! another compatible command (f.x. clippy) in a background thread and provide
//! LSP diagnostics based on the output of the command.

#![warn(rust_2018_idioms, unused_lifetimes, semicolon_in_expressions_from_macros)]

use std::{
    fmt, io,
    process::{ChildStderr, ChildStdout, Command, Stdio},
    time::Duration,
};

use crossbeam_channel::{never, select, unbounded, Receiver, Sender};
use paths::AbsPathBuf;
use serde::Deserialize;
use stdx::{process::streaming_output, JodChild};

pub use cargo_metadata::diagnostic::{
    Applicability, Diagnostic, DiagnosticCode, DiagnosticLevel, DiagnosticSpan,
    DiagnosticSpanMacroExpansion,
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FlycheckConfig {
    CargoCommand {
        command: String,
        target_triple: Option<String>,
        all_targets: bool,
        no_default_features: bool,
        all_features: bool,
        features: Vec<String>,
        extra_args: Vec<String>,
    },
    CustomCommand {
        command: String,
        args: Vec<String>,
    },
}

impl fmt::Display for FlycheckConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FlycheckConfig::CargoCommand { command, .. } => write!(f, "cargo {}", command),
            FlycheckConfig::CustomCommand { command, args } => {
                write!(f, "{} {}", command, args.join(" "))
            }
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
    sender: Sender<Restart>,
    _thread: jod_thread::JoinHandle,
    id: usize,
}

impl FlycheckHandle {
    pub fn spawn(
        id: usize,
        sender: Box<dyn Fn(Message) + Send>,
        config: FlycheckConfig,
        workspace_root: AbsPathBuf,
    ) -> FlycheckHandle {
        let actor = FlycheckActor::new(id, sender, config, workspace_root);
        let (sender, receiver) = unbounded::<Restart>();
        let thread = jod_thread::Builder::new()
            .name("Flycheck".to_owned())
            .spawn(move || actor.run(receiver))
            .expect("failed to spawn thread");
        FlycheckHandle { id, sender, _thread: thread }
    }

    /// Schedule a re-start of the cargo check worker.
    pub fn restart(&self, filepath: Option<String>) {
        dbg!("restart for", &self.id);
        match filepath {
            Some(path) => {dbg!("verus"); self.sender.send(Restart::Verus(path)).unwrap()},
            None => {dbg!("not verus"); self.sender.send(Restart::Yes).unwrap();}, // normal cargo check
        };
    }

    /// Stop this cargo check worker.
    pub fn cancel(&self) {
        self.sender.send(Restart::No).unwrap();
    }

    pub fn id(&self) -> usize {
        self.id
    }
}

pub enum Message {
    /// Request adding a diagnostic with fixes included to a file
    AddDiagnostic { id: usize, workspace_root: AbsPathBuf, diagnostic: Diagnostic },

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

enum Restart {
    Yes,
    No,
    Verus(String),
}

struct FlycheckActor {
    id: usize,
    sender: Box<dyn Fn(Message) + Send>,
    config: FlycheckConfig,
    workspace_root: AbsPathBuf,
    /// CargoHandle exists to wrap around the communication needed to be able to
    /// run `cargo check` without blocking. Currently the Rust standard library
    /// doesn't provide a way to read sub-process output without blocking, so we
    /// have to wrap sub-processes output handling in a thread and pass messages
    /// back over a channel.
    cargo_handle: Option<CargoHandle>,

    // verus
    // running Verus for all file takes too much time
    // only run Verus for the saved file 
    // pub recent_saved_file: uri;
}

enum Event {
    Restart(Restart),
    CheckEvent(Option<CargoMessage>),
}

impl FlycheckActor {
    fn new(
        id: usize,
        sender: Box<dyn Fn(Message) + Send>,
        config: FlycheckConfig,
        workspace_root: AbsPathBuf,
    ) -> FlycheckActor {
        tracing::info!(%id, ?workspace_root, "Spawning flycheck");
        FlycheckActor { id, sender, config, workspace_root, cargo_handle: None }
    }
    fn progress(&self, progress: Progress) {
        self.send(Message::Progress { id: self.id, progress });
    }
    fn next_event(&self, inbox: &Receiver<Restart>) -> Option<Event> {
        let check_chan = self.cargo_handle.as_ref().map(|cargo| &cargo.receiver);
        select! {
            recv(inbox) -> msg => msg.ok().map(Event::Restart),
            recv(check_chan.unwrap_or(&never())) -> msg => Some(Event::CheckEvent(msg.ok())),
        }
    }
    fn run(mut self, inbox: Receiver<Restart>) {
        while let Some(event) = self.next_event(&inbox) {
            // dbg!("flychecker run:");
            match event {
                Event::Restart(Restart::No) => {
                    self.cancel_check_process();
                }
                Event::Restart(Restart::Yes) => {
                    // Cancel the previously spawned process
                    self.cancel_check_process();
                    while let Ok(_) = inbox.recv_timeout(Duration::from_millis(50)) {}

                    let command = self.check_command(None);
                    // insert file name here
                    tracing::debug!(?command, "will restart flycheck");
                    match CargoHandle::spawn(command) {
                        Ok(cargo_handle) => {
                            tracing::debug!(
                                command = ?self.check_command(None),
                                "did  restart flycheck"
                            );
                            // dbg!("cargo handle spawn success");
                            self.cargo_handle = Some(cargo_handle);
                            self.progress(Progress::DidStart);
                        }
                        Err(error) => {
                            // dbg!("cargo handle spwan failed");
                            self.progress(Progress::DidFailToRestart(format!(
                                "Failed to run the following command: {:?} error={}",
                                self.check_command(None),
                                error
                            )));
                        }
                    }
                }
                Event::Restart(Restart::Verus(path)) => {
                    // Cancel the previously spawned process
                    dbg!("restart verus");
                    self.cancel_check_process();
                    while let Ok(_) = inbox.recv_timeout(Duration::from_millis(50)) {}



                    let command = self.check_command(Some(path.clone()));

                    // insert file name here
                    tracing::debug!(?command, "will restart flycheck");
                    match CargoHandle::spawn(command) {
                        Ok(cargo_handle) => {
                            tracing::debug!(
                                command = ?self.check_command(Some(path.clone())),
                                "did  restart flycheck"
                            );
                            // dbg!("cargo handle spawn success");
                            self.cargo_handle = Some(cargo_handle);
                            self.progress(Progress::DidStart);
                        }
                        Err(error) => {
                            // dbg!("cargo handle spwan failed");
                            self.progress(Progress::DidFailToRestart(format!(
                                "Failed to run the following command: {:?} error={}",
                                self.check_command(Some(path.clone())),
                                error
                            )));
                        }
                    }
                }
                
                Event::CheckEvent(None) => {
                    // dbg!("FlycheckActor fun checkevent");
                    tracing::debug!(flycheck_id = self.id, "flycheck finished");

                    // Watcher finished
                    let cargo_handle = self.cargo_handle.take().unwrap();
                    let res = cargo_handle.join();
                    if res.is_err() {
                        tracing::error!(
                            "Flycheck failed to run the following command: {:?}",
                            self.check_command(None)
                        );
                    }
                    self.progress(Progress::DidFinish(res));
                }
                Event::CheckEvent(Some(message)) => match message {
                    CargoMessage::CompilerArtifact(msg) => {
                        // dbg!("FlycheckActor fun checkevent compilerartifact");
                        self.progress(Progress::DidCheckCrate(msg.target.name));
                    }

                    CargoMessage::Diagnostic(msg) => {
                        // dbg!("FlycheckActor fun checkevent diagnostic");
                        self.send(Message::AddDiagnostic {
                            id: self.id,
                            workspace_root: self.workspace_root.clone(),
                            diagnostic: msg,
                        });
                    }
                    CargoMessage::VerusResult(msg) => {
                        self.progress(Progress::VerusResult(msg));
                    }
                },
            }
        }
        // If we rerun the thread, we need to discard the previous check results first
        self.cancel_check_process();
    }

    fn cancel_check_process(&mut self) {
        if let Some(cargo_handle) = self.cargo_handle.take() {
            tracing::debug!(
                command = ?self.check_command(None),
                "did  cancel flycheck"
            );
            cargo_handle.cancel();
            self.progress(Progress::DidCancel);
        }
    }

    fn check_command(&self, filepath:Option<String>) -> Command {
        let mut cmd = match &self.config {
            FlycheckConfig::CargoCommand {
                command,
                target_triple,
                no_default_features,
                all_targets,
                all_features,
                extra_args,
                features,
            } => {
                // dbg!("normal command");
                
                let mut cmd = Command::new(toolchain::cargo());
                cmd.arg(command);
                cmd.current_dir(&self.workspace_root);
                cmd.args(&["--workspace", "--message-format=json", "--manifest-path"])
                    .arg(self.workspace_root.join("Cargo.toml").as_os_str());

                if let Some(target) = target_triple {
                    cmd.args(&["--target", target.as_str()]);
                }
                if *all_targets {
                    cmd.arg("--all-targets");
                }
                if *all_features {
                    cmd.arg("--all-features");
                } else {
                    if *no_default_features {
                        cmd.arg("--no-default-features");
                    }
                    if !features.is_empty() {
                        cmd.arg("--features");
                        cmd.arg(features.join(" "));
                    }
                }
                cmd.args(extra_args);
                cmd
            }
            FlycheckConfig::CustomCommand { command, args } => {
                dbg!(&command);
                dbg!(&args);
                match filepath.as_ref() {
                    Some(path) => {
                        //  insert current-saved filename here
                        let mut args = args.to_vec();
                        args = args.iter().map(|x| {if x == "${file}"  {path.clone()} else {x.clone()} }).collect();
                        let mut cmd = Command::new(command);
                        cmd.args(args);
                        cmd
                    },
                    None => {
                        // lets do simple cargo check
                        let mut cmd = Command::new(toolchain::cargo());
                        cmd.arg("check");
                        cmd.current_dir(&self.workspace_root);
                        cmd.args(&["--workspace", "--message-format=json", "--manifest-path"])
                            .arg(self.workspace_root.join("Cargo.toml").as_os_str());
                        cmd
                    }
                }
            }
        };
        cmd.current_dir(&self.workspace_root);
        dbg!(&cmd);
        cmd
    }

    fn send(&self, check_task: Message) {
        (self.sender)(check_task);
    }
}

/// A handle to a cargo process used for fly-checking.
struct CargoHandle {
    /// The handle to the actual cargo process. As we cannot cancel directly from with
    /// a read syscall dropping and therefor terminating the process is our best option.
    child: JodChild,
    thread: jod_thread::JoinHandle<io::Result<(bool, String)>>,
    receiver: Receiver<CargoMessage>,
}

impl CargoHandle {
    fn spawn(mut command: Command) -> std::io::Result<CargoHandle> {
        command.stdout(Stdio::piped()).stderr(Stdio::piped()).stdin(Stdio::null());
        let mut child = JodChild::spawn(command)?;

        let stdout = child.stdout.take().unwrap();
        let stderr = child.stderr.take().unwrap();

        let (sender, receiver) = unbounded();
        let actor = CargoActor::new(sender, stdout, stderr);
        let thread = jod_thread::Builder::new()
            .name("CargoHandle".to_owned())
            .spawn(move || actor.run())
            .expect("failed to spawn thread");
        Ok(CargoHandle { child, thread, receiver })
    }

    fn cancel(mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }

    fn join(mut self) -> io::Result<()> {
        let _ = self.child.kill();
        let exit_status = self.child.wait()?;
        let (read_at_least_one_message, error) = self.thread.join()?;
 
        // when Verus verification fails, it terminates with compiler error
        if read_at_least_one_message {//|| exit_status.success() 
            Ok(())
        } else {
            Err(io::Error::new(io::ErrorKind::Other, format!(
                "Cargo watcher failed, the command produced no valid metadata (exit code: {:?}):\n{}",
                exit_status, error
            )))
        }
    }
}

struct CargoActor {
    sender: Sender<CargoMessage>,
    stdout: ChildStdout,
    stderr: ChildStderr,
}

impl CargoActor {
    fn new(sender: Sender<CargoMessage>, stdout: ChildStdout, stderr: ChildStderr) -> CargoActor {
        CargoActor { sender, stdout, stderr }
    }

    fn run(self) -> io::Result<(bool, String)> {
        // We manually read a line at a time, instead of using serde's
        // stream deserializers, because the deserializer cannot recover
        // from an error, resulting in it getting stuck, because we try to
        // be resilient against failures.
        //
        // Because cargo only outputs one JSON object per line, we can
        // simply skip a line if it doesn't parse, which just ignores any
        // erroneous output.

        let error = String::new(); //FIXME proper error 
        // let mut error = String::new();
        let mut read_at_least_one_stdout = false;
        let mut read_at_least_one_stderr = false;
        let mut reported_verus_result_stdout = false;
        let mut reported_verus_result_stderr = false;
        let output = streaming_output(
            self.stdout,
            self.stderr,
            &mut |line| {
                read_at_least_one_stdout = true;
                // dbg!("stdout line");
                // dbg!(&line);

                // Try to deserialize a message from Cargo or Rustc.
                let mut deserializer = serde_json::Deserializer::from_str(line);
                deserializer.disable_recursion_limit();
                if let Ok(message) = JsonMessage::deserialize(&mut deserializer) {
                    match message {
                        // Skip certain kinds of messages to only spend time on what's useful
                        JsonMessage::Cargo(message) => match message {
                            cargo_metadata::Message::CompilerArtifact(artifact)
                                if !artifact.fresh =>
                            {
                                // dbg!(&artifact);
                                self.sender.send(CargoMessage::CompilerArtifact(artifact)).unwrap();
                            }
                            cargo_metadata::Message::CompilerMessage(msg) => {
                                // dbg!(&msg);
                                self.sender.send(CargoMessage::Diagnostic(msg.message)).unwrap();
                            }
                            _ => (),
                        },
                        JsonMessage::Rustc(message) => {
                            // dbg!(&message);
                            self.sender.send(CargoMessage::Diagnostic(message)).unwrap();
                        }
                    }
                } else {
                    // dbg!("json deserialize error");
                    // report verification result
                    if line.starts_with("verification results::") {
                        dbg!("generating verus result stdout", &line);
                        self.sender.send(CargoMessage::VerusResult(line.to_string())).unwrap();
                        reported_verus_result_stdout = true;
                    }
                }
            },
            // FIXME: below should be removed given that  original r-a properly gets complier error from stdout
            &mut |line| {
                read_at_least_one_stderr = true;
                // dbg!("stderr line");
                // dbg!(&line);

                // Try to deserialize a message from Cargo or Rustc.
                let mut deserializer = serde_json::Deserializer::from_str(line);
                deserializer.disable_recursion_limit();
                if let Ok(message) = JsonMessage::deserialize(&mut deserializer) {
                    match message {
                        // Skip certain kinds of messages to only spend time on what's useful
                        JsonMessage::Cargo(message) => match message {
                            cargo_metadata::Message::CompilerArtifact(artifact)
                                if !artifact.fresh =>
                            {
                                // dbg!(&artifact);
                                self.sender.send(CargoMessage::CompilerArtifact(artifact)).unwrap();
                            }
                            cargo_metadata::Message::CompilerMessage(msg) => {
                                // dbg!(&msg);
                                self.sender.send(CargoMessage::Diagnostic(msg.message)).unwrap();
                            }
                            _ => (),
                        },
                        JsonMessage::Rustc(message) => {
                            // dbg!(&message);
                            self.sender.send(CargoMessage::Diagnostic(message)).unwrap();
                        }
                    }
                } else {
                    // dbg!("json deserialize error");
                    // report verification result
                    if line.starts_with("verification results::") {
                        dbg!("generating verus result stderr", &line);
                        self.sender.send(CargoMessage::VerusResult(line.to_string())).unwrap();
                        reported_verus_result_stderr = true;
                    }
                }
            },
            // &mut |line| {
            //     dbg!("error line");
            //     dbg!(&line);
            //     error.push_str(line);
            //     error.push('\n');
            // },
        );
        let reported_verus_result = reported_verus_result_stdout || reported_verus_result_stderr;
        let read_at_least_one_message = read_at_least_one_stdout || read_at_least_one_stderr;
        dbg!(&reported_verus_result ,&read_at_least_one_message);

        if !reported_verus_result{
            dbg!("generating missing verus result");
            self.sender.send(CargoMessage::VerusResult("Verus Failed Before Producing Verification Result".to_string())).unwrap();
        }
            
        match output {
            Ok(_) => {dbg!("cargo actor output OK"); Ok((read_at_least_one_message, error))},
            Err(e) =>{dbg!("cargo actor output Err"); Err(io::Error::new(e.kind(), format!("{:?}: {}", e, error)))},
        }
    }
}

enum CargoMessage {
    CompilerArtifact(cargo_metadata::Artifact),
    Diagnostic(Diagnostic),
    VerusResult(String),
}

#[derive(Deserialize)]
#[serde(untagged)]
enum JsonMessage {
    Cargo(cargo_metadata::Message),
    Rustc(Diagnostic),
}
