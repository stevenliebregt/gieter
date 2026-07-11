use std::io::Write;
use std::process::{Command, Stdio};

/// Runs an external command, writes `stdin` to it, and returns its stdout. A non-zero exit is an
/// error carrying the captured stderr. The command is an argv array.
pub(crate) fn run(command: &[String], stdin: &[u8]) -> Result<Vec<u8>, String> {
    let (program, args) = command
        .split_first()
        .ok_or_else(|| "external command is empty".to_string())?;

    let mut child = Command::new(program)
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| format!("failed to spawn '{program}': {error}"))?;

    // Write the request on a separate thread so a large stdout can't fill its pipe
    // buffer and deadlock us while we are still writing the request to stdin.
    let mut sink = child
        .stdin
        .take()
        .ok_or_else(|| "failed to open plugin stdin".to_string())?;
    let request = stdin.to_vec();
    let writer = std::thread::spawn(move || sink.write_all(&request));

    let output = child
        .wait_with_output()
        .map_err(|error| format!("failed to run '{program}': {error}"))?;

    // Delivery of stdin is best-effort: a plugin that produced valid output without
    // reading its whole request (closing the pipe early) is not a failure.
    let _ = writer.join();

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!(
            "plugin '{program}' exited with {}: {}",
            output.status,
            stderr.trim()
        ));
    }

    Ok(output.stdout)
}

/// Reads the required `command` argv array from a source/emitter's options.
pub(crate) fn read_command(options: &toml::Table) -> Result<Vec<String>, String> {
    let array = options
        .get("command")
        .ok_or_else(|| "an external plugin requires a `command` array".to_string())?
        .as_array()
        .ok_or_else(|| "`command` must be an array of strings".to_string())?;

    if array.is_empty() {
        return Err("`command` must not be empty".to_string());
    }

    array
        .iter()
        .map(|item| {
            item.as_str()
                .map(str::to_string)
                .ok_or_else(|| "`command` entries must be strings".to_string())
        })
        .collect()
}

/// The options forwarded to the plugin as JSON: every key except the transport `command`.
pub(crate) fn forward_options(options: &toml::Table) -> Result<serde_json::Value, String> {
    let mut forwarded = options.clone();
    forwarded.remove("command");
    serde_json::to_value(&forwarded).map_err(|error| error.to_string())
}
