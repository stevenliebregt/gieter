use std::io::{Read, Write};
use std::process::{Child, Command, ExitStatus, Stdio};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

/// Runs an external command, writes `stdin` to it, and returns its stdout. A non-zero exit is an
/// error carrying the captured stderr. A run that outlives `timeout` is killed.
pub(crate) fn run(command: &[String], stdin: &[u8], timeout: Duration) -> Result<Vec<u8>, String> {
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

    // Write the request and drain stdout/stderr on their own threads, so none of the pipes can fill
    // and deadlock while we wait for the child to exit.
    let mut sink = child
        .stdin
        .take()
        .ok_or_else(|| "failed to open plugin stdin".to_string())?;
    let request = stdin.to_vec();
    let writer = std::thread::spawn(move || sink.write_all(&request));

    let stdout = drain(
        child
            .stdout
            .take()
            .ok_or_else(|| "failed to open plugin stdout".to_string())?,
    );
    let stderr = drain(
        child
            .stderr
            .take()
            .ok_or_else(|| "failed to open plugin stderr".to_string())?,
    );

    let status = wait(&mut child, program, timeout)?;

    // Delivery of stdin is best-effort: a plugin that produced valid output without
    // reading its whole request (closing the pipe early) is not a failure.
    let _ = writer.join();

    if !status.success() {
        let stderr = join(stderr, "stderr")?;
        return Err(format!(
            "plugin '{program}' exited with {status}: {}",
            String::from_utf8_lossy(&stderr).trim()
        ));
    }

    join(stdout, "stdout")
}

/// Reads a pipe to end on its own thread.
fn drain<R: Read + Send + 'static>(mut pipe: R) -> JoinHandle<std::io::Result<Vec<u8>>> {
    std::thread::spawn(move || {
        let mut buffer = vec![];
        pipe.read_to_end(&mut buffer).map(|_| buffer)
    })
}

fn join(reader: JoinHandle<std::io::Result<Vec<u8>>>, name: &str) -> Result<Vec<u8>, String> {
    reader
        .join()
        .map_err(|_| format!("plugin {name} reader thread panicked"))?
        .map_err(|error| format!("failed to read plugin {name}: {error}"))
}

/// Waits for the child, killing it if it does not resolve within the given `timeout`.
fn wait(child: &mut Child, program: &str, timeout: Duration) -> Result<ExitStatus, String> {
    let start = Instant::now();

    loop {
        match child
            .try_wait()
            .map_err(|error| format!("failed to run '{program}': {error}"))?
        {
            Some(status) => return Ok(status),
            None if start.elapsed() >= timeout => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(format!(
                    "plugin '{program}' timed out after {}s",
                    timeout.as_secs()
                ));
            }
            None => std::thread::sleep(Duration::from_millis(50)),
        }
    }
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

/// External plugins that do not set a `timeout` are killed after this long.
const DEFAULT_TIMEOUT: Duration = Duration::from_secs(120);

/// Reads the `timeout` (whole seconds) from a source/emitter's options, defaulting to
/// [`DEFAULT_TIMEOUT`] when unset.
pub(crate) fn read_timeout(options: &toml::Table) -> Result<Duration, String> {
    let Some(value) = options.get("timeout") else {
        return Ok(DEFAULT_TIMEOUT);
    };

    let seconds = value
        .as_integer()
        .filter(|seconds| *seconds > 0)
        .ok_or_else(|| "`timeout` must be a positive whole number of seconds".to_string())?;

    Ok(Duration::from_secs(seconds as u64))
}

/// The options forwarded to the plugin as JSON: every key except the transport keys
/// `command` and `timeout`.
pub(crate) fn forward_options(options: &toml::Table) -> Result<serde_json::Value, String> {
    let mut forwarded = options.clone();
    forwarded.remove("command");
    forwarded.remove("timeout");
    serde_json::to_value(&forwarded).map_err(|error| error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_command_that_outruns_its_timeout_is_killed() {
        let error = run(
            &["sleep".into(), "5".into()],
            b"",
            Duration::from_millis(200),
        )
        .unwrap_err();
        assert!(error.contains("timed out"), "{error}");
    }
}
