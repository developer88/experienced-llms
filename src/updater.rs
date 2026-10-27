use std::fs;
use std::io;

pub fn get_asset_name() -> Result<&'static str, String> {
    let os = std::env::consts::OS;
    let arch = std::env::consts::ARCH;

    match (os, arch) {
        ("linux", "x86_64") => Ok("experienced-llms-linux-x86_64-gnu"),
        ("linux", "aarch64") => Ok("experienced-llms-linux-aarch64-gnu"),
        ("windows", "x86_64") => Ok("experienced-llms-windows-x86_64.exe"),
        ("macos", "x86_64") => Ok("experienced-llms-darwin-x86_64"),
        ("macos", "aarch64") => Ok("experienced-llms-darwin-arm64"),
        _ => Err(format!("Unsupported platform/arch: {}-{}", os, arch)),
    }
}

fn normalize_version(v: &str) -> String {
    v.trim_start_matches('v')
        .replace('-', "")
        .replace('.', "")
        .to_lowercase()
}

pub fn check_for_updates(tag: Option<&str>) -> Result<(String, String, String, bool), Box<dyn std::error::Error>> {
    let url = match tag {
        Some(t) => format!(
            "https://api.github.com/repos/developer88/experienced-llms/releases/tags/{}",
            t
        ),
        None => "https://api.github.com/repos/developer88/experienced-llms/releases/latest".to_string(),
    };

    let resp = match ureq::get(&url)
        .set("User-Agent", &format!("experienced-llms/{}", env!("CARGO_PKG_VERSION")))
        .call()
    {
        Ok(r) => r,
        Err(e) => {
            return Err(format!("Failed to connect to GitHub Releases API ({}): {}", url, e).into());
        }
    };

    let json: serde_json::Value = resp.into_json()?;
    let tag_name = json["tag_name"]
        .as_str()
        .ok_or("Missing 'tag_name' in release payload")?
        .trim();

    let clean_remote = tag_name.trim_start_matches('v');
    let current = env!("CARGO_PKG_VERSION");
    let is_newer = normalize_version(clean_remote) != normalize_version(current);

    Ok((current.to_string(), clean_remote.to_string(), tag_name.to_string(), is_newer))
}

pub fn run_self_update(check: bool, auto_yes: bool, tag: Option<&str>) -> Result<(), Box<dyn std::error::Error>> {
    println!("[+] Checking for updates from GitHub Releases...");
    let (current, remote, tag_name, is_newer) = match check_for_updates(tag) {
        Ok(res) => res,
        Err(e) => {
            eprintln!("[-] Update check failed: {}", e);
            return Err(e);
        }
    };

    println!("  Current version: {}", current);
    println!("  Latest version:  {}", remote);

    if check {
        if is_newer {
            println!("[*] An update is available: {} -> {}", current, remote);
        } else {
            println!("[✓] Experienced LLMs is already up to date.");
        }
        return Ok(());
    }

    if !is_newer && tag.is_none() && !auto_yes {
        println!("[✓] Already up to date (version {}).", current);
        return Ok(());
    }

    let asset_name = get_asset_name().map_err(|e| -> Box<dyn std::error::Error> { e.into() })?;
    let release_tag = if let Some(t) = tag {
        t.to_string()
    } else {
        tag_name
    };

    let download_url = format!(
        "https://github.com/developer88/experienced-llms/releases/download/{}/{}",
        release_tag, asset_name
    );
    println!("[+] Downloading binary asset from: {}", download_url);

    let current_exe = std::env::current_exe()?;
    let temp_exe = current_exe.with_extension("tmp_download");

    let resp = match ureq::get(&download_url)
        .set("User-Agent", &format!("experienced-llms/{}", current))
        .call()
    {
        Ok(r) => r,
        Err(e) => {
            return Err(format!("Failed to download release asset from {}: {}", download_url, e).into());
        }
    };

    let mut reader = resp.into_reader();
    let mut out_file = fs::File::create(&temp_exe)?;
    io::copy(&mut reader, &mut out_file)?;
    drop(out_file);

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = fs::set_permissions(&temp_exe, fs::Permissions::from_mode(0o755));
    }

    #[cfg(windows)]
    {
        let old_exe = current_exe.with_extension("exe.old");
        let _ = fs::remove_file(&old_exe);
        fs::rename(&current_exe, &old_exe)?;
        fs::rename(&temp_exe, &current_exe)?;
    }

    #[cfg(not(windows))]
    {
        fs::rename(&temp_exe, &current_exe)?;
    }

    println!(
        "[✔] Standalone binary updated to version {} at: {}",
        remote,
        current_exe.display()
    );

    println!("[+] Applying post-update migrations and synchronizing assets...");
    let conn = crate::db::get_connection(None)?;
    crate::db::run_migrations(&conn)?;
    crate::config::ensure_template_with_prompt(&crate::config::get_experienced_home(), auto_yes)?;
    crate::integrations::ensure_skill_with_prompt(None, auto_yes)?;
    crate::config::ensure_llm_cmd_with_prompt(auto_yes, None)?;
    let (count, _) = crate::renderer::rebuild_master_experience(None, None)?;
    println!("[✓] Re-rendered active experience ({} active facts).", count);
    println!("==========================================================");
    println!("  [✔] Experienced LLMs updated successfully to {}!", remote);
    println!("==========================================================");

    Ok(())
}
