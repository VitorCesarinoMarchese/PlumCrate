//! Dispatch external browser requests outside the desktop event loop.
use egui::{Context, OutputCommand};

pub fn dispatch_pending(ctx: &Context, import: Option<&crate::browser_import::BrowserImport>) {
    for url in take_urls(ctx) {
        let wake = ctx.clone();
        let launch_error = import.and_then(|import| import.launch_failure_reporter(&url));
        let thread_error = import.and_then(|import| import.launch_failure_reporter(&url));
        if let Err(error) = std::thread::Builder::new()
            .name("desktop-browser-launch".into())
            .spawn(move || {
                if let Err(error) = open_browser(&url) {
                    let message = format!("Could not open browser: {error}");
                    eprintln!("{message}");
                    if let Some(report) = launch_error {
                        report(message);
                    }
                }
                wake.request_repaint();
            })
        {
            let message = format!("Could not start browser launcher: {error}");
            eprintln!("{message}");
            if let Some(report) = thread_error {
                report(message);
            }
            ctx.request_repaint();
        }
    }
}

fn open_browser(url: &str) -> Result<(), String> {
    #[cfg(target_os = "linux")]
    {
        open_with_fallback(
            || webbrowser::open(url).map_err(|error| error.to_string()),
            |program, args| {
                let status = std::process::Command::new(program)
                    .args(args)
                    .stdin(std::process::Stdio::null())
                    .status()
                    .map_err(|error| error.to_string())?;
                if status.success() {
                    Ok(())
                } else {
                    Err(format!("exit status {status}"))
                }
            },
            url,
        )
    }
    #[cfg(not(target_os = "linux"))]
    webbrowser::open(url).map_err(|error| error.to_string())
}

#[cfg(any(target_os = "linux", test))]
fn open_with_fallback(
    discover: impl FnOnce() -> Result<(), String>,
    mut run: impl FnMut(&str, &[String]) -> Result<(), String>,
    url: &str,
) -> Result<(), String> {
    let mut errors = match discover() {
        Ok(()) => return Ok(()),
        Err(error) => vec![error],
    };
    for (program, args) in [
        ("xdg-open", vec![url.to_owned()]),
        ("gio", vec!["open".to_owned(), url.to_owned()]),
    ] {
        match run(program, &args) {
            Ok(()) => return Ok(()),
            Err(error) => errors.push(format!("{program}: {error}")),
        }
    }
    Err(errors.join("; "))
}

fn take_urls(ctx: &Context) -> Vec<String> {
    ctx.output_mut(|output| {
        let mut urls = Vec::new();
        output.commands.retain(|command| match command {
            OutputCommand::OpenUrl(url) => {
                urls.push(url.url.clone());
                false
            }
            _ => true,
        });
        urls
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn linux_fallback_uses_desktop_openers_after_discovery_fails() {
        open_with_fallback(
            || Ok(()),
            |_, _| panic!("successful discovery must not open twice"),
            "https://example.com",
        )
        .unwrap();
        let mut attempts = Vec::new();
        open_with_fallback(
            || Err("browser discovery failed".into()),
            |program, args| {
                attempts.push((program.to_owned(), args.to_vec()));
                if program == "gio" {
                    Ok(())
                } else {
                    Err("xdg-open failed".into())
                }
            },
            "https://example.com/download?a=1&b=2",
        )
        .unwrap();
        assert_eq!(
            attempts,
            vec![
                (
                    "xdg-open".into(),
                    vec!["https://example.com/download?a=1&b=2".into()]
                ),
                (
                    "gio".into(),
                    vec!["open".into(), "https://example.com/download?a=1&b=2".into()]
                ),
            ]
        );
        let error = open_with_fallback(
            || Err("discovery".into()),
            |_, _| Err("opener failed".into()),
            "https://example.com",
        )
        .unwrap_err();
        assert!(error.contains("discovery") && error.contains("opener failed"));
    }

    #[test]
    fn browser_dispatch_is_once_only_and_preserves_clipboard_commands() {
        let ctx = Context::default();
        ctx.copy_text("keep this clipboard value".into());
        ctx.open_url(egui::OpenUrl::new_tab("https://example.com/one"));
        ctx.open_url(egui::OpenUrl::new_tab("https://example.com/two"));
        assert_eq!(
            take_urls(&ctx),
            ["https://example.com/one", "https://example.com/two"]
        );
        assert!(take_urls(&ctx).is_empty());
        ctx.output_mut(|output| {
            assert!(matches!(output.commands.as_slice(), [OutputCommand::CopyText(text)] if text == "keep this clipboard value"));
        });
    }
}
