//! Development entry point for the constrained UHID broker.

use std::os::unix::net::UnixListener;
use std::path::PathBuf;
use std::sync::Arc;

use gr_provider_linux_uhid_broker::{BrokerPolicy, UhidBrokerServer, serve};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args_os();
    let _program = args.next();
    let Some(socket) = args.next() else {
        return Err("usage: vgpd-uhid-broker <socket-path>".into());
    };
    if args.next().is_some() {
        return Err("usage: vgpd-uhid-broker <socket-path>".into());
    }
    let path = PathBuf::from(socket);
    let listener = UnixListener::bind(&path)?;
    let server = Arc::new(UhidBrokerServer::new(BrokerPolicy::default()));
    serve(&listener, &server)?;
    Ok(())
}
