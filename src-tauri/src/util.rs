use std::io::Write;
use std::path::Path;

/// Write a file atomically with owner-only permissions on Unix.
pub fn write_atomic(path: &Path, data: &[u8]) -> std::io::Result<()> {
    let tmp = path.with_extension("tmp");
    {
        let mut f = std::fs::File::create(&tmp)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            f.set_permissions(std::fs::Permissions::from_mode(0o600))?;
        }
        f.write_all(data)?;
        f.sync_all()?;
    }
    std::fs::rename(tmp, path)
}

/// Resident memory (MB) and CPU% of a process, measured with `ps`. Returns None if unavailable.
pub fn process_usage(pid: u32) -> Option<(f64, f64)> {
    #[cfg(unix)]
    {
        let out = std::process::Command::new("/bin/ps").args(["-o", "rss=,%cpu=", "-p", &pid.to_string()]).output().ok()?;
        let s = String::from_utf8_lossy(&out.stdout);
        let mut it = s.split_whitespace();
        let rss_kb: f64 = it.next()?.parse().ok()?;
        let cpu: f64 = it.next()?.parse().ok()?;
        Some((rss_kb / 1024.0, cpu))
    }
    #[cfg(not(unix))]
    {
        let _ = pid;
        None
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn atomic_write() {
        let dir = std::env::temp_dir().join(format!("minpoint-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let p = dir.join("x.json");
        super::write_atomic(&p, b"{}").unwrap();
        assert_eq!(std::fs::read(&p).unwrap(), b"{}");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert_eq!(std::fs::metadata(&p).unwrap().permissions().mode() & 0o777, 0o600);
        }
        #[cfg(unix)]
        assert!(super::process_usage(std::process::id()).is_some());
        std::fs::remove_dir_all(dir).unwrap();
    }
}
