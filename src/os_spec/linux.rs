use crate::fslog;
use log::info;
use pakkly_error::FormattedError;
use std::io::{self, Write};
use std::process::Command;
use tempfile::TempDir;

struct SudoCommand {
    command: String,
    args: Vec<String>,
    _file_to_clear: Option<Box<TempDir>>,
}
pub struct LinuxSudo {
    commands_buffer: Vec<SudoCommand>,
}
impl LinuxSudo {
    pub fn new() -> Self {
        return Self { commands_buffer: vec![] };
    }
    pub fn write_file<C>(&mut self, final_path: String, contents: C) -> Result<(), FormattedError>
    where
        C: AsRef<[u8]>,
    {
        let tmpdir = tempfile::tempdir()?;
        let filepath = tmpdir.path().join("file");
        fslog::write(&filepath, contents)?;

        self.cmd(
            "cp".to_string(),
            [filepath.to_str().unwrap().to_string(), final_path.to_string()].to_vec(),
            Some(Box::from(tmpdir)),
        );
        return Ok(());
    }
    fn cmd(&mut self, command: String, args: Vec<String>, _file_to_clear: Option<Box<TempDir>>) {
        self.commands_buffer.push(SudoCommand { command, args, _file_to_clear })
    }
    pub fn command(&mut self, command: String, args: Vec<String>) {
        self.cmd(command, args, None);
    }
    pub fn flush(&self) -> Result<(), FormattedError> {
        if self.commands_buffer.len() == 0 {
            return Err(FormattedError::from_str("Sudo cmdbuffer cannot be length 0".to_string()));
        }
        let possible_bins = ["/usr/bin/pkexec", "/usr/bin/kdesudo", "/usr/bin/gksudo"];

        for bin in &possible_bins {
            let file = std::path::Path::new(bin);
            if fslog::exists(&file) {
                //we can call it!
                //assemble the megacommand
                let mut final_command: Vec<String> = vec![];
                for cmd in &self.commands_buffer {
                    final_command.push(cmd.command.to_string());
                    for arg in &cmd.args {
                        final_command.push(arg.to_string());
                    }
                    final_command.push("&&".to_string());
                }
                final_command.truncate(final_command.len() - 1); //remove final &&
                let megacommand = final_command.join(" ");
                info!("Found user_sudo as {}", file.display());
                info!("Executing command {} as sudo", megacommand);
                let output = Command::new(file).arg("sh").arg("-c").arg(megacommand).output()?;
                info!("status: {}", output.status);
                io::stdout().write_all(&output.stdout).unwrap();
                io::stderr().write_all(&output.stderr).unwrap();
                //let e = cmd.wait()?;
                if output.status.success() {
                    return Ok(());
                } else {
                    return Err(FormattedError::from_missing_sudo("Sudo command threw!".to_string()));
                }
            }
        }
        return Err(FormattedError::from_missing_sudo("No Sudo Found".to_string()));
    }
}
