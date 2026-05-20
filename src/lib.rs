use zed_extension_api as zed;

mod cmake;
mod debug;
mod environment;
mod error;
mod lsp;
mod paths;

#[derive(Default)]
struct MsvcToolkitExtension;

impl zed::Extension for MsvcToolkitExtension {
    fn new() -> Self {
        Self
    }
}

zed::register_extension!(MsvcToolkitExtension);
