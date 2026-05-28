use std::panic;

/// Install a global panic hook that emits structured tracing output before
/// the process exits. This ensures panics land in the log aggregator rather
/// than being swallowed by async runtimes.
pub fn install_panic_hook() {
    let default_hook = panic::take_hook();
    panic::set_hook(Box::new(move |info| {
        let location = info.location()
            .map(|l| format!("{}:{}:{}", l.file(), l.line(), l.column()))
            .unwrap_or_else(|| "unknown location".into());
        let payload = if let Some(s) = info.payload().downcast_ref::<&str>() {
            (*s).to_string()
        } else if let Some(s) = info.payload().downcast_ref::<String>() {
            s.clone()
        } else {
            "non-string panic payload".into()
        };
        tracing::error!(
            panic.location = %location,
            panic.payload  = %payload,
            "bonsai-daemon panicked — process will exit"
        );
        default_hook(info);
    }));
}
