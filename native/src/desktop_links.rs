//! Dispatch external browser requests outside the desktop event loop.
use egui::{Context, OutputCommand};

pub fn dispatch_pending(ctx: &Context) {
    for url in take_urls(ctx) {
        let wake = ctx.clone();
        if let Err(error) = std::thread::Builder::new()
            .name("desktop-browser-launch".into())
            .spawn(move || {
                if let Err(error) = webbrowser::open(&url) {
                    eprintln!("Could not open browser: {error}");
                }
                wake.request_repaint();
            })
        {
            eprintln!("Could not start browser launcher: {error}");
        }
    }
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
