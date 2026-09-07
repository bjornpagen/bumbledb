//! Owned crash-test child with one persistent protocol reader.
//! A Unix socket supplies a real read timeout without a reader thread.

use std::io::{BufRead as _, BufReader};
use std::os::fd::OwnedFd;
use std::os::unix::net::UnixStream;
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

pub struct TestChild {
    pub process: Child,
    output: BufReader<UnixStream>,
    timeout: Duration,
}

impl TestChild {
    pub fn spawn(command: &mut Command, timeout: Duration) -> Self {
        let (reader, writer) = UnixStream::pair().expect("child output socket");
        reader
            .set_read_timeout(Some(timeout))
            .expect("read timeout");
        let process = command
            .stdout(OwnedFd::from(writer))
            .stdin(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()
            .expect("spawn child");
        // The command must not retain another writer and hide child EOF.
        command.stdout(Stdio::null());
        Self {
            process,
            output: BufReader::new(reader),
            timeout,
        }
    }

    pub fn next_line(&mut self) -> String {
        let mut line = String::new();
        let read = self.output.read_line(&mut line).expect("read child line");
        assert!(read > 0, "child stdout closed before its outcome");
        line.trim().to_owned()
    }

    pub fn await_marker(&mut self, marker: &str) {
        let start = Instant::now();
        loop {
            assert!(
                start.elapsed() < self.timeout,
                "child never printed {marker}"
            );
            // libtest prefixes the first marker with `test <name> ... `.
            // Later lines stay in this same reader, including read-ahead.
            if self.next_line().ends_with(marker) {
                return;
            }
        }
    }

    pub fn signal(&self, name: &str) {
        let status = Command::new("kill")
            .args([format!("-{name}"), self.process.id().to_string()])
            .status()
            .expect("kill runs");
        assert!(status.success(), "kill -{name} failed");
    }
}

impl Drop for TestChild {
    fn drop(&mut self) {
        // SIGKILL also terminates a SIGSTOP-suspended child. Child caches a
        // reaped status, so these are harmless after an explicit kill/wait.
        let _ = self.process.kill();
        let _ = self.process.wait();
    }
}
