//! Pet packs: a folder with a `manifest.json` and image frames.
//!
//! ```text
//! pets/panda/
//!   manifest.json
//!   idle-0.svg
//!   walk-0.svg ...
//! ```
//!
//! Only `idle` is required. Any other animation a pack leaves out falls back
//! along [`Anim::fallback`], so a pack with just `idle` and `walk` still works.

use crate::behavior::Anim;
use base64::Engine;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};
use std::fs;
use std::path::{Component, Path, PathBuf};

pub const MANIFEST: &str = "manifest.json";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AnimSpec {
    pub frames: Vec<String>,
    #[serde(default = "default_fps")]
    pub fps: f32,
}

fn default_fps() -> f32 {
    6.0
}

/// Which way the artwork faces. The UI mirrors it when walking the other way.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum SpriteFacing {
    Left,
    #[default]
    Right,
    /// Symmetric art that never needs mirroring.
    None,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Manifest {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub author: String,
    #[serde(default)]
    pub license: String,
    /// Display size in logical pixels at scale 1.0.
    pub width: u32,
    pub height: u32,
    #[serde(default)]
    pub facing: SpriteFacing,
    pub animations: HashMap<String, AnimSpec>,
}

#[derive(Debug, Clone)]
pub struct Pack {
    pub dir: PathBuf,
    pub manifest: Manifest,
}

/// A summary for the dashboard's pet picker.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PackInfo {
    pub id: String,
    pub name: String,
    pub author: String,
    pub license: String,
    pub preview: String,
}

/// Everything the pet window needs to draw a pack, with frames inlined as
/// data URIs so the webview needs no file access.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LoadedPack {
    pub id: String,
    pub name: String,
    pub width: u32,
    pub height: u32,
    pub facing: SpriteFacing,
    pub animations: BTreeMap<&'static str, LoadedAnim>,
}

#[derive(Debug, Clone, Serialize)]
pub struct LoadedAnim {
    pub frames: Vec<String>,
    pub fps: f32,
}

#[derive(Debug)]
pub struct PackError {
    pub dir: PathBuf,
    pub reason: String,
}

impl std::fmt::Display for PackError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.dir.display(), self.reason)
    }
}

impl std::error::Error for PackError {}

impl Pack {
    pub fn load(dir: &Path) -> Result<Pack, PackError> {
        let fail = |reason: String| PackError { dir: dir.to_owned(), reason };
        let text = fs::read_to_string(dir.join(MANIFEST)).map_err(|e| fail(format!("cannot read {MANIFEST}: {e}")))?;
        let manifest: Manifest = serde_json::from_str(&text).map_err(|e| fail(format!("bad {MANIFEST}: {e}")))?;

        if manifest.id.is_empty()
            || !manifest.id.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-' || c == '_')
        {
            return Err(fail("id must use only a-z, 0-9, '-' or '_'".into()));
        }
        if !(16..=1024).contains(&manifest.width) || !(16..=1024).contains(&manifest.height) {
            return Err(fail("width and height must be between 16 and 1024".into()));
        }
        match manifest.animations.get(Anim::Idle.key()) {
            Some(idle) if !idle.frames.is_empty() => {}
            _ => return Err(fail("an 'idle' animation with at least one frame is required".into())),
        }
        for (name, spec) in &manifest.animations {
            if spec.frames.is_empty() {
                return Err(fail(format!("animation '{name}' has no frames")));
            }
            if !(0.5..=60.0).contains(&spec.fps) {
                return Err(fail(format!("animation '{name}' fps must be between 0.5 and 60")));
            }
            for frame in &spec.frames {
                if !is_plain_relative(frame) {
                    return Err(fail(format!("frame '{frame}' must be a file inside the pack folder")));
                }
                if mime_for(frame).is_none() {
                    return Err(fail(format!("frame '{frame}' must be .png, .gif, .webp, .jpg or .svg")));
                }
                if !dir.join(frame).is_file() {
                    return Err(fail(format!("frame '{frame}' not found")));
                }
            }
        }
        Ok(Pack { dir: dir.to_owned(), manifest })
    }

    /// The animation to play for `anim`, following fallbacks.
    pub fn resolve(&self, anim: Anim) -> &AnimSpec {
        let mut a = anim;
        loop {
            if let Some(spec) = self.manifest.animations.get(a.key()) {
                return spec;
            }
            // `idle` is checked in `load`, and every chain ends there.
            a = a.fallback().unwrap_or(Anim::Idle);
        }
    }

    pub fn info(&self) -> Result<PackInfo, PackError> {
        let m = &self.manifest;
        Ok(PackInfo {
            id: m.id.clone(),
            name: m.name.clone(),
            author: m.author.clone(),
            license: m.license.clone(),
            preview: self.data_uri(&self.resolve(Anim::Idle).frames[0])?,
        })
    }

    pub fn load_frames(&self) -> Result<LoadedPack, PackError> {
        // Read each file once even when several animations share it.
        let mut cache: HashMap<&str, String> = HashMap::new();
        let mut animations = BTreeMap::new();
        for anim in Anim::ALL {
            let spec = self.resolve(anim);
            let mut frames = Vec::with_capacity(spec.frames.len());
            for f in &spec.frames {
                if !cache.contains_key(f.as_str()) {
                    cache.insert(f, self.data_uri(f)?);
                }
                frames.push(cache[f.as_str()].clone());
            }
            animations.insert(anim.key(), LoadedAnim { frames, fps: spec.fps });
        }
        let m = &self.manifest;
        Ok(LoadedPack {
            id: m.id.clone(),
            name: m.name.clone(),
            width: m.width,
            height: m.height,
            facing: m.facing,
            animations,
        })
    }

    fn data_uri(&self, frame: &str) -> Result<String, PackError> {
        let bytes = fs::read(self.dir.join(frame))
            .map_err(|e| PackError { dir: self.dir.clone(), reason: format!("cannot read '{frame}': {e}") })?;
        let mime = mime_for(frame).unwrap_or("application/octet-stream");
        Ok(format!("data:{mime};base64,{}", base64::engine::general_purpose::STANDARD.encode(bytes)))
    }
}

/// Find every pack under `roots`. A pack in a later root replaces one with
/// the same id from an earlier root, so user packs can override built-ins.
/// Broken packs are returned separately instead of failing the whole scan.
pub fn discover(roots: &[PathBuf]) -> (Vec<Pack>, Vec<PackError>) {
    let mut packs: Vec<Pack> = Vec::new();
    let mut errors = Vec::new();
    for root in roots {
        let Ok(entries) = fs::read_dir(root) else { continue };
        let mut dirs: Vec<PathBuf> =
            entries.filter_map(|e| e.ok()).map(|e| e.path()).filter(|p| p.join(MANIFEST).is_file()).collect();
        dirs.sort();
        for dir in dirs {
            match Pack::load(&dir) {
                Ok(pack) => {
                    packs.retain(|p| p.manifest.id != pack.manifest.id);
                    packs.push(pack);
                }
                Err(e) => errors.push(e),
            }
        }
    }
    packs.sort_by_key(|p| p.manifest.name.to_lowercase());
    (packs, errors)
}

fn is_plain_relative(path: &str) -> bool {
    let p = Path::new(path);
    !path.is_empty() && p.components().all(|c| matches!(c, Component::Normal(_)))
}

fn mime_for(frame: &str) -> Option<&'static str> {
    let ext = Path::new(frame).extension()?.to_str()?.to_ascii_lowercase();
    Some(match ext.as_str() {
        "png" => "image/png",
        "gif" => "image/gif",
        "webp" => "image/webp",
        "jpg" | "jpeg" => "image/jpeg",
        "svg" => "image/svg+xml",
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write_pack(root: &Path, folder: &str, manifest: &str, files: &[&str]) -> PathBuf {
        let dir = root.join(folder);
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join(MANIFEST), manifest).unwrap();
        for f in files {
            fs::write(dir.join(f), b"<svg/>").unwrap();
        }
        dir
    }

    const MINIMAL: &str = r#"{"id":"cat","name":"Cat","width":64,"height":64,
        "animations":{"idle":{"frames":["a.svg"]},"walk":{"frames":["a.svg","b.svg"],"fps":10}}}"#;

    #[test]
    fn minimal_pack_resolves_everything() {
        let tmp = tempfile::tempdir().unwrap();
        let pack = Pack::load(&write_pack(tmp.path(), "cat", MINIMAL, &["a.svg", "b.svg"])).unwrap();
        assert_eq!(pack.resolve(Anim::Walk).frames.len(), 2);
        assert_eq!(pack.resolve(Anim::Drag).frames, vec!["a.svg"]);
        let loaded = pack.load_frames().unwrap();
        assert_eq!(loaded.animations.len(), Anim::ALL.len());
        assert!(loaded.animations["walk"].frames[1].starts_with("data:image/svg+xml;base64,"));
        assert!(pack.info().unwrap().preview.starts_with("data:image/svg+xml"));
    }

    #[test]
    fn rejects_bad_packs() {
        let tmp = tempfile::tempdir().unwrap();
        let cases = [
            ("missing-frame", MINIMAL, &["a.svg"][..]),
            (
                "no-idle",
                r#"{"id":"x","name":"X","width":64,"height":64,"animations":{"walk":{"frames":["a.svg"]}}}"#,
                &["a.svg"][..],
            ),
            (
                "escape",
                r#"{"id":"x","name":"X","width":64,"height":64,"animations":{"idle":{"frames":["../a.svg"]}}}"#,
                &[][..],
            ),
            (
                "bad-id",
                r#"{"id":"X Y","name":"X","width":64,"height":64,"animations":{"idle":{"frames":["a.svg"]}}}"#,
                &["a.svg"][..],
            ),
            (
                "bad-ext",
                r#"{"id":"x","name":"X","width":64,"height":64,"animations":{"idle":{"frames":["a.exe"]}}}"#,
                &["a.exe"][..],
            ),
        ];
        for (folder, manifest, files) in cases {
            let dir = write_pack(tmp.path(), folder, manifest, files);
            assert!(Pack::load(&dir).is_err(), "{folder} should be rejected");
        }
    }

    #[test]
    fn later_roots_override_and_errors_are_collected() {
        let tmp = tempfile::tempdir().unwrap();
        let builtin = tmp.path().join("builtin");
        let user = tmp.path().join("user");
        write_pack(&builtin, "cat", MINIMAL, &["a.svg", "b.svg"]);
        write_pack(&user, "my-cat", &MINIMAL.replace("\"Cat\"", "\"Custom Cat\""), &["a.svg", "b.svg"]);
        write_pack(&user, "broken", "{", &[]);
        let (packs, errors) = discover(&[builtin, user.clone(), tmp.path().join("missing")]);
        assert_eq!(packs.len(), 1);
        assert_eq!(packs[0].manifest.name, "Custom Cat");
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].dir, user.join("broken"));
    }
}
