//! Client for the macOS native helper sidecar (newline-delimited JSON over stdio).

use serde_json::{json, Value};
use std::collections::HashMap;
use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::{channel, Sender};
use std::sync::{Arc, Mutex};
use std::time::Duration;

pub type EventHandler = Arc<dyn Fn(&str, Value) + Send + Sync>;

pub struct HelperClient {
    stdin: Mutex<Option<ChildStdin>>,
    child: Mutex<Option<Child>>,
    pending: Arc<Mutex<HashMap<u64, Sender<Result<Value, String>>>>>,
    next: AtomicU64,
    path: PathBuf,
    on_event: EventHandler,
}

impl HelperClient {
    pub fn spawn(path: PathBuf, on_event: EventHandler) -> Result<Arc<Self>, String> {
        let c = Arc::new(HelperClient {
            stdin: Mutex::new(None),
            child: Mutex::new(None),
            pending: Arc::new(Mutex::new(HashMap::new())),
            next: AtomicU64::new(1),
            path,
            on_event,
        });
        c.start()?;
        Ok(c)
    }

    fn start(self: &Arc<Self>) -> Result<(), String> {
        let mut child = Command::new(&self.path)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()
            .map_err(|e| format!("could not start native helper at {}: {e}", self.path.display()))?;
        let stdout = child.stdout.take().ok_or("no helper stdout")?;
        *self.stdin.lock().unwrap() = child.stdin.take();
        *self.child.lock().unwrap() = Some(child);
        let pending = self.pending.clone();
        let on_event = self.on_event.clone();
        std::thread::Builder::new()
            .name("inputflow-helper-reader".into())
            .spawn(move || {
                for line in BufReader::new(stdout).lines() {
                    let Ok(line) = line else { break };
                    let Ok(v) = serde_json::from_str::<Value>(&line) else { continue };
                    if let Some(ev) = v.get("event").and_then(Value::as_str) {
                        on_event(ev, v.get("data").cloned().unwrap_or(Value::Null));
                        continue;
                    }
                    let Some(id) = v.get("id").and_then(Value::as_u64) else { continue };
                    if let Some(tx) = pending.lock().unwrap().remove(&id) {
                        let r = match v.get("error") {
                            Some(e) => Err(e.as_str().unwrap_or("helper error").to_string()),
                            None => Ok(v.get("result").cloned().unwrap_or(Value::Null)),
                        };
                        let _ = tx.send(r);
                    }
                }
                // Helper exited: fail all pending calls.
                for (_, tx) in pending.lock().unwrap().drain() {
                    let _ = tx.send(Err("native helper stopped".into()));
                }
                on_event("helperExited", Value::Null);
            })
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    /// Blocking call with timeout. Run from a blocking context (spawn_blocking / thread).
    pub fn call(&self, method: &str, params: Value, timeout: Duration) -> Result<Value, String> {
        let id = self.next.fetch_add(1, Ordering::Relaxed);
        let (tx, rx) = channel();
        self.pending.lock().unwrap().insert(id, tx);
        let line = json!({"id": id, "method": method, "params": params}).to_string() + "\n";
        {
            let mut g = self.stdin.lock().unwrap();
            let Some(stdin) = g.as_mut() else {
                self.pending.lock().unwrap().remove(&id);
                return Err("native helper is not running".into());
            };
            if let Err(e) = stdin.write_all(line.as_bytes()).and_then(|_| stdin.flush()) {
                self.pending.lock().unwrap().remove(&id);
                return Err(format!("native helper write failed: {e}"));
            }
        }
        match rx.recv_timeout(timeout) {
            Ok(r) => r,
            Err(_) => {
                self.pending.lock().unwrap().remove(&id);
                Err(format!("native helper timed out on {method}"))
            }
        }
    }

    pub fn shutdown(&self) {
        // Closing stdin makes the helper release its event tap and exit.
        self.stdin.lock().unwrap().take();
        if let Some(mut c) = self.child.lock().unwrap().take() {
            for _ in 0..20 {
                if matches!(c.try_wait(), Ok(Some(_))) {
                    return;
                }
                std::thread::sleep(Duration::from_millis(25));
            }
            let _ = c.kill();
            let _ = c.wait();
        }
    }
}

impl Drop for HelperClient {
    fn drop(&mut self) {
        self.shutdown();
    }
}
