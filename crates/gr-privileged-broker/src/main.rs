//! Root-owned local broker daemon (Linux only).

#[cfg(target_os = "linux")]
mod daemon;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    #[cfg(target_os = "linux")]
    {
        daemon::run()
    }
    #[cfg(not(target_os = "linux"))]
    {
        Err(std::io::Error::new(
            std::io::ErrorKind::Unsupported,
            "the privileged realization broker requires Linux",
        )
        .into())
    }
}

#[cfg(all(test, not(target_os = "linux")))]
mod tests {
    #[test]
    fn daemon_rejects_unsupported_platforms() {
        assert!(super::main().is_err());
    }
}
