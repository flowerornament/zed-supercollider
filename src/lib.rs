use zed_extension_api as zed;

struct SuperColliderExtension;

impl zed::Extension for SuperColliderExtension {
    fn new() -> Self {
        SuperColliderExtension
    }
}

zed::register_extension!(SuperColliderExtension);

