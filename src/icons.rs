use std::path::Path;

use crate::config::IconPack;
use crate::{Error, PendingFile, read_icon_input, serialize_json};

// Fixed filenames keep head links and the public output collision checks in sync.
pub(crate) const PNG_FILES: [(&str, u32); 7] = [
    ("favicon-16x16.png", 16),
    ("favicon-32x32.png", 32),
    ("favicon-48x48.png", 48),
    ("apple-touch-icon.png", 180),
    ("icon-192x192.png", 192),
    ("icon-512x512.png", 512),
    ("icon-maskable-512x512.png", 512),
];

pub(crate) fn public_paths() -> impl Iterator<Item = String> {
    ["favicon.ico", "site.webmanifest"]
        .into_iter()
        .map(str::to_owned)
        .chain(PNG_FILES.iter().map(|(name, _)| format!("icons/{name}")))
}

pub(crate) fn files(
    root: &Path,
    pack: &IconPack,
    title: &str,
    base_path: &str,
    has_home: bool,
) -> Result<Vec<PendingFile>, Error> {
    let mut files = Vec::new();
    for (name, size) in PNG_FILES {
        let relative = pack.directory.join(name);
        let contents = read_icon_input(root, &relative, "icon pack")?;
        // Check the PNG header and advertised dimensions, not a full image decode.
        let valid = contents.starts_with(b"\x89PNG\r\n\x1a\n")
            && contents.get(8..16) == Some(b"\0\0\0\rIHDR")
            && contents.get(16..20) == Some(size.to_be_bytes().as_slice())
            && contents.get(20..24) == Some(size.to_be_bytes().as_slice());
        if !valid {
            return Err(Error::Invalid(format!(
                "icon pack {} must be a {size}x{size} PNG",
                relative.display()
            )));
        }
        files.push(PendingFile {
            path: format!("icons/{name}").into(),
            contents,
        });
    }
    let relative = pack.directory.join("favicon.ico");
    let contents = read_icon_input(root, &relative, "icon pack")?;
    validate_ico(&contents)?;
    files.push(PendingFile {
        path: "favicon.ico".into(),
        contents,
    });
    let scope = format!("{base_path}/");
    let start_url = if has_home {
        scope.clone()
    } else {
        format!("{base_path}/reference/")
    };
    files.push(PendingFile {
        path: "site.webmanifest".into(),
        contents: serialize_json(&serde_json::json!({
            "id": scope,
            "name": title,
            "short_name": title,
            "start_url": start_url,
            "scope": scope,
            // Supply identity/home-screen icons without opting a docs site into app mode.
            "display": "browser",
            "theme_color": pack.theme_color,
            "background_color": pack.background_color,
            "icons": [
                {"src":"icons/icon-192x192.png", "sizes":"192x192", "type":"image/png", "purpose":"any"},
                {"src":"icons/icon-512x512.png", "sizes":"512x512", "type":"image/png", "purpose":"any"},
                {"src":"icons/icon-maskable-512x512.png", "sizes":"512x512", "type":"image/png", "purpose":"maskable"}
            ]
        }))?,
    });
    Ok(files)
}

fn validate_ico(bytes: &[u8]) -> Result<(), Error> {
    let invalid = || {
        Error::Invalid("icon pack favicon.ico must contain 16, 32 and 48px ICO images with valid directory bounds".to_owned())
    };
    if bytes.get(..4) != Some(&[0, 0, 1, 0]) {
        return Err(invalid());
    }
    let count = bytes.get(4..6).ok_or_else(invalid)?;
    let count = usize::from(u16::from_le_bytes([count[0], count[1]]));
    let directory_end = 6 + count * 16;
    let directory = bytes.get(6..directory_end).ok_or_else(invalid)?;
    let mut sizes = std::collections::BTreeSet::new();
    for entry in directory.chunks_exact(16) {
        let length = u32::from_le_bytes([entry[8], entry[9], entry[10], entry[11]]) as usize;
        let offset = u32::from_le_bytes([entry[12], entry[13], entry[14], entry[15]]) as usize;
        let end = offset.checked_add(length).ok_or_else(invalid)?;
        if length == 0 || offset < directory_end || bytes.get(offset..end).is_none() {
            return Err(invalid());
        }
        if entry[0] == entry[1] {
            sizes.insert(entry[0]);
        }
    }
    if ![16, 32, 48].into_iter().all(|size| sizes.contains(&size)) {
        return Err(invalid());
    }
    Ok(())
}
