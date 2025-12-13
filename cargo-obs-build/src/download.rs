use std::{
    fs::File,
    io::{stdout, BufReader, Write},
    path::{Path, PathBuf},
    sync::mpsc::{self},
    thread,
    time::{Duration, Instant},
};

use anyhow::{anyhow, bail};
#[cfg(feature = "cli")]
use colored::Colorize;
use http_req::{
    chunked::ChunkReader,
    request::RequestMessage,
    response::Response,
    stream::{Stream, ThreadReceive, ThreadSend},
    uri::Uri,
};
#[cfg(feature = "cli")]
use indicatif::{ProgressBar, ProgressStyle};
#[cfg(feature = "cli")]
use log::{debug, info};
use log::{error, trace};
use sha2::{Digest, Sha256};

use crate::git::ReleaseInfo;

const DEFAULT_REQ_TIMEOUT: u64 = 60 * 60;

/// Downloads OBS binaries and optionally debug symbols.
/// Returns (main_binary_path, Option<debug_symbols_path>).
pub fn download_binaries(
    build_dir: &Path,
    info: &ReleaseInfo,
    strip_debug: bool,
) -> anyhow::Result<(PathBuf, Option<PathBuf>)> {
    // Determine platform-specific search criteria based on TARGET platform
    // Priority: OBS_BUILD_TARGET > CARGO_CFG_TARGET_OS > host platform
    let target_os = std::env::var("OBS_BUILD_TARGET_OS")
        .or_else(|_| std::env::var("CARGO_CFG_TARGET_OS"))
        .unwrap_or_else(|_| std::env::consts::OS.to_string());
    let target_arch = std::env::var("OBS_BUILD_TARGET_ARCH")
        .or_else(|_| std::env::var("CARGO_CFG_TARGET_ARCH"))
        .unwrap_or_else(|_| std::env::consts::ARCH.to_string());

    // Map Rust architecture names to OBS naming
    let architecture = if target_arch == "x86_64" {
        "x64"
    } else {
        "arm64"
    };

    let (platform_name, file_extension, output_filename, arch_name) = if target_os == "macos" {
        let arch = if target_arch == "x86_64" {
            "intel" // macOS uses "Intel" for x86_64
        } else {
            "apple" // macOS uses "Apple" for arm64 (Apple Silicon)
        };
        // macOS only provides DMG for main binaries (tar.xz is only for dSYMs)
        ("macos", ".dmg", "obs-prebuilt-macos.dmg", arch)
    } else if target_os == "windows" {
        ("windows", ".zip", "obs-prebuilt-windows.zip", architecture)
    } else {
        // Linux not supported - require manual obs-studio installation
        bail!("Linux OBS download not supported - install obs-studio manually");
    };

    // Find main binary - always exclude debug symbol archives
    let to_download = info.assets.iter().find(|e| {
        let name = e["name"].as_str().unwrap_or("").to_lowercase();

        // Examples:
        // Windows: OBS-Studio-32.0.2-Windows-x64.zip
        // macOS: OBS-Studio-32.0.2-macOS-Intel.dmg or OBS-Studio-32.0.2-macOS-Apple.dmg
        name.contains("obs-studio")
            && (name.contains(platform_name) || (target_os == "windows" && name.contains("full")))
            && name.contains(file_extension)
            && name.contains(arch_name)
            && !name.contains("pdb")
            && !name.contains("dsym")
            && !name.contains("dbsym")
    });

    let to_download = to_download.ok_or_else(|| {
        anyhow!(
            "No OBS Studio binaries found for platform: {}",
            platform_name
        )
    })?;

    let url = to_download["browser_download_url"]
        .as_str()
        .ok_or(anyhow!("No download url found"))?;

    let download_path = build_dir.join(output_filename);

    #[cfg(feature = "colored")]
    println!("Downloading OBS from {}", url.green());
    let hash = download_file(url, &download_path)?;

    let name = to_download["name"].as_str().unwrap_or("");
    let checksum = &info.checksums.get(&name.to_lowercase());

    if let Some(checksum) = checksum {
        if checksum.to_lowercase() != hash.to_lowercase() {
            bail!("Checksums do not match for main binary");
        } else {
            #[cfg(feature = "colored")]
            info!("{}", "Checksums match".on_green());
        }
    } else {
        error!("No checksum found for {}", name);
    }

    // Download debug symbols if not stripping
    let debug_symbols_path = if !strip_debug {
        download_debug_symbols(build_dir, info, &target_os, arch_name)?
    } else {
        None
    };

    Ok((download_path, debug_symbols_path))
}

/// Downloads debug symbols (PDB for Windows, dSYM for macOS) if available.
fn download_debug_symbols(
    build_dir: &Path,
    info: &ReleaseInfo,
    target_os: &str,
    arch_name: &str,
) -> anyhow::Result<Option<PathBuf>> {
    // Determine debug symbols file pattern based on platform
    let (debug_marker, debug_extension, output_filename) = if target_os == "macos" {
        ("dsym", ".tar.xz", "obs-prebuilt-macos-dsyms.tar.xz")
    } else if target_os == "windows" {
        ("pdb", ".zip", "obs-prebuilt-windows-pdbs.zip")
    } else {
        return Ok(None);
    };

    // Find debug symbols archive
    let debug_asset = info.assets.iter().find(|e| {
        let name = e["name"].as_str().unwrap_or("").to_lowercase();
        name.contains("obs-studio")
            && name.contains(debug_marker)
            && name.contains(debug_extension)
            && name.contains(arch_name)
    });

    let Some(debug_asset) = debug_asset else {
        trace!("No debug symbols archive found for {}", target_os);
        return Ok(None);
    };

    let url = debug_asset["browser_download_url"]
        .as_str()
        .ok_or(anyhow!("No download url found for debug symbols"))?;

    let download_path = build_dir.join(output_filename);

    #[cfg(feature = "colored")]
    println!("Downloading debug symbols from {}", url.green());
    let hash = download_file(url, &download_path)?;

    let name = debug_asset["name"].as_str().unwrap_or("");
    let checksum = &info.checksums.get(&name.to_lowercase());

    if let Some(checksum) = checksum {
        if checksum.to_lowercase() != hash.to_lowercase() {
            bail!("Checksums do not match for debug symbols");
        } else {
            #[cfg(feature = "colored")]
            info!("{}", "Debug symbols checksums match".on_green());
        }
    } else {
        error!("No checksum found for debug symbols: {}", name);
    }

    Ok(Some(download_path))
}

/// Returns hash
pub fn download_file(url: &str, path: &Path) -> anyhow::Result<String> {
    let timeout = Duration::from_secs(60);
    #[cfg(feature = "colored")]
    debug!("Downloading OBS binaries from {}", url.green());

    let uri = Uri::try_from(url)?;
    let mut stream = Stream::connect(&uri, Some(timeout))?;

    stream.set_read_timeout(Some(timeout))?;
    stream.set_write_timeout(Some(timeout))?;

    stream = Stream::try_to_https(stream, &uri, None)?;

    let res = RequestMessage::new(&uri)
        .header("Connection", "Close")
        .header("User-Agent", "cargo-obs-build")
        .parse();
    stream.write_all(&res)?;

    // Set up variables
    let (sender, receiver) = mpsc::channel();
    let (sender_supp, receiver_supp) = mpsc::channel();
    let mut raw_response_head: Vec<u8> = Vec::new();
    let mut buf_reader = BufReader::new(stream);

    // Read from the stream and send over data via `sender`.
    thread::spawn(move || {
        buf_reader.send_head(&sender);

        let params = receiver_supp.recv();
        if params.is_err() {
            return;
        }

        let params: Vec<&str> = params.unwrap();
        //TODO this never exists
        if params.contains(&"chunked") {
            let mut buf_reader = ChunkReader::from(buf_reader);
            buf_reader.send_all(&sender);
        } else {
            buf_reader.send_all(&sender);
        }
    });

    let deadline = Instant::now() + Duration::from_secs(DEFAULT_REQ_TIMEOUT);

    // Receive and process `head` of the response.
    raw_response_head.receive(&receiver, deadline)?;

    let response = Response::from_head(&raw_response_head)?;
    let content_len = response.content_len().unwrap_or(1) as u64;
    let encoding = response.headers().get("Transfer-Encoding");
    let mut params = Vec::with_capacity(4);

    if response.status_code().is_redirect() {
        let location = response.headers().get("Location");
        if location.is_none() {
            bail!("No location header found");
        }

        let location = location.unwrap();
        return download_file(location, path);
    }

    if let Some(encode) = encoding {
        if encode == "chunked" {
            params.push("chunked");
        }
    }

    sender_supp.send(params)?;

    if content_len == 0 {
        bail!("Content length is 0");
    }

    #[cfg(feature = "cli")]
    let pb = ProgressBar::new(content_len);
    #[cfg(feature = "cli")]
    {
        let style = ProgressStyle::default_bar()
            .template("{msg}\n{spinner:.green} [{elapsed_precise}] [{wide_bar:.cyan/blue}] {bytes}/{total_bytes} ({bytes_per_sec}, {eta})")
            .map_err(|e| anyhow!("Couldn't create style {:#?}", e))?
            .progress_chars("#>-");
        pb.set_style(style);
        pb.set_message("Downloading OBS binaries".to_string());
    }

    let mut file =
        File::create(path).or(Err(anyhow!("Failed to create file '{}'", path.display())))?;
    let mut downloaded: u64 = 0;

    let mut hasher = Sha256::new();
    loop {
        let now = Instant::now();
        let remaining_time = deadline - now;

        let item = receiver.recv_timeout(remaining_time);
        if let Err(_e) = item {
            break;
        }

        let chunk = item?;

        hasher.write_all(&chunk)?;
        file.write_all(&chunk)
            .or(Err(anyhow!("Error while writing to file")))?;

        let new = std::cmp::min(downloaded + (chunk.len() as u64), content_len);
        downloaded = new;
        #[cfg(feature = "cli")]
        pb.set_position(new);
    }

    #[cfg(feature = "cli")]
    pb.finish_with_message(format!("Downloaded OBS to {}", path.display()));
    trace!("Hashing...");
    let _ = stdout().flush();
    Ok(hex::encode(hasher.finalize()))
}
