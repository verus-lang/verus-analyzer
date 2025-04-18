//! Run Verus and return the verification result

use crate::AssistContext;
use std::{fs::File, io::Write, process::Command, time::Instant};
use syntax::ast::{self, vst, HasModuleItem, HasName};

impl<'a> AssistContext<'a> {
    // for now, assume one file only
    // 1) copy the file to a temporary file
    // 2) replace out the function with this VST Fn
    // 3) run verus on the temporary file
    // run Verus on the `vst::Fn` node
    // assume running verus inside vs-code
    // TODO: pass the whole project to verus, instead of this single file
    // TODO: projects with multiple file/module -- `verify-module` flag --verify-function flag
    // output: None -> compile error
    /// We only replace the function in the input
    /// we use the remaining codebase when invoking Verus
    /// Output None when Verus fails to start (e.g., compile error on the modified function)
    pub(crate) fn try_verus(
        &self,
        vst_fn: &vst::Fn, // only replace this function and run
    ) -> Option<VerifResult> {
        let source_file = &self.source_file;
        let verus_exec_path = std::env::var("VERUS_BINARY_PATH")
            .expect("please set VERUS_BINARY_PATH environment variable");
        let mut text_string = String::new();
        // in VST, we should also be able to "print" and verify
        // display for VST should be correct modulo whitespace
        for it in source_file.items() {
            match it {
                ast::Item::Fn(f) => {
                    text_string += "\nverus!{\n";
                    if f.name()?.to_string().trim() == vst_fn.name.to_string().trim() {
                        text_string += &vst_fn.to_string();
                    } else {
                        // review: f.cst.to_string?
                        text_string += &f.to_string();
                    }
                    text_string += "\n}\n";
                }
                ast::Item::Enum(e) => {
                    text_string += "\nverus!{\n";
                    text_string += &e.to_string();
                    text_string += "\n}\n";
                }
                ast::Item::Struct(e) => {
                    text_string += "\nverus!{\n";
                    // review: it.cst.to_string?  for now, No -- see is_failing
                    text_string += &e.to_string();
                    text_string += "\n}\n";
                }
                ast::Item::Impl(e) => {
                    text_string += "\nverus!{\n";
                    text_string += &e.to_string();
                    text_string += "\n}\n";
                }
                _ => {
                    text_string += &it.to_string();
                    text_string += "\n";
                }
            }
        }
        //dbg!(&text_string);

        // let verify_func_flag = "--verify-function";
        // let verify_root_flag = "--verify-root"; // TODO: figure out the surrounding module of `token`
        // let func_name = vst_fn.name.to_string();

        // REIVEW: instead of writing to a file in the tmp directory, consider using `memfd_create` for an anonymous file
        // refer to `man memfd_create` or `dev/shm`
        // REVIEW: Is this true? In linux, set env TMPDIR to set the tmp directory. Otherwise, it fails
        let tmp_dir = tempfile::TempDir::new().ok()?;
        let file_path = tmp_dir.path().join("verus_proof_action_scratch_file.rs");
        //dbg!(&file_path);
        let display = file_path.display();

        // Open a file in write-only mode, returns `io::Result<File>`
        let mut file = match File::create(&file_path) {
            Err(why) => {
                dbg!("couldn't create {}: {}", display, why);
                return None;
            }
            Ok(file) => file,
        };

        // Write the modified verus program to `file`, returns `io::Result<()>`
        match file.write_all(text_string.as_bytes()) {
            Err(why) => {
                dbg!("couldn't write to {}: {}", display, why);
                return None;
            }
            Ok(_) => (),//dbg!("successfully wrote to {}", display),
        };

        let now = Instant::now();
        let output = Command::new(verus_exec_path)
            .arg(file_path)
            .arg("--multiple-errors")
            .arg("10") // we want many errors as proof-action reads this. By default, Verus gives a couple of errors as a human reads those.
            .output();
        let elapsed = now.elapsed().as_secs();

        let output = output.ok()?;
        // dbg!(&output);
        if output.status.success() {
            return Some(VerifResult::mk_success(elapsed));
        } else {
            // disambiguate verification failure     VS    compile error etc
            match std::str::from_utf8(&output.stdout) {
                Ok(out) => {
                    //dbg!(out);
                    if out.contains("verification results:: verified: 0 errors: 0") {
                        // failure from other errors. (e.g. compile error)
                        return None;
                    } else {
                        // verification failure
                        match std::str::from_utf8(&output.stderr) {
                            Ok(err_msg) => {
                                return Some(VerifResult::mk_failure(
                                    out.into(),
                                    err_msg.into(),
                                    elapsed,
                                ));
                            }
                            Err(_) => return None,
                        }
                    }
                }
                Err(_) => return None,
            }
        }
    }
}

#[derive(Debug)]
pub(crate) struct VerifResult {
    pub(crate) is_success: bool,
    // FIXME: properly parse json using serde and store the list of assertion/ensures/requires
    #[allow(dead_code)]
    pub(crate) stdout: String,
    pub(crate) stderr: String,
    pub(crate) time: u64,
}

impl VerifResult {
    pub(crate) fn mk_success(time: u64) -> Self {
        VerifResult { is_success: true, stdout: String::new(), stderr: String::new(), time }
    }

    pub(crate) fn mk_failure(stdout: String, stderr: String, time: u64) -> Self {
        VerifResult { is_success: false, stdout, stderr, time }
    }

    pub(crate) fn is_failing(&self, assertion: &vst::AssertExpr) -> bool {
        if self.is_success {
            return false;
        }
        self.stderr.contains(&assertion.to_string())
    }
}
