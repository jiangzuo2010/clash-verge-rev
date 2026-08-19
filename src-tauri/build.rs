fn main() {
    configure_enterprise_policy_secret();

    #[cfg(feature = "clippy")]
    {
        println!("cargo:warning=Skipping tauri_build during Clippy");
    }

    #[cfg(not(feature = "clippy"))]
    tauri_build::build();
}

fn configure_enterprise_policy_secret() {
    const KEY: &str = "ENTERPRISE_POLICY_CRYPTO_SECRET";

    if let Ok(value) = std::env::var(KEY)
        && !value.trim().is_empty()
    {
        println!("cargo:rustc-env={KEY}={value}");
        return;
    }

    let manifest_dir = std::path::PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap_or_default());
    let candidates = [manifest_dir.join(".env"), manifest_dir.join("..").join(".env")];
    for path in candidates {
        println!("cargo:rerun-if-changed={}", path.display());
        let Ok(content) = std::fs::read_to_string(&path) else {
            continue;
        };
        if let Some(value) = read_env_value(&content, KEY) {
            println!("cargo:rustc-env={KEY}={value}");
            return;
        }
    }
}

fn read_env_value(content: &str, key: &str) -> Option<String> {
    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let (name, value) = line.split_once('=')?;
        if name.trim() == key {
            let value = value.trim().trim_matches('"').trim_matches('\'');
            if !value.is_empty() {
                return Some(value.to_owned());
            }
        }
    }
    None
}
