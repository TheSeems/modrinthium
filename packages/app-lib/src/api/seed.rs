//! Seed modpacks: install an `.mrpack` from a hosted manifest URL and keep it
//! updated without the pack ever being published on Modrinth.
//!
//! A "seed" is a small JSON manifest a server owner hosts at a stable URL:
//!
//! ```json
//! {
//!   "name": "My SMP",
//!   "version": "1.4.0",
//!   "changelog": "Added Sodium, removed JEI",
//!   "mrpack": "https://example.com/packs/pack-1.4.0.mrpack",
//!   "hash": "sha1:...",
//!   "icon": "https://example.com/icon.png"
//! }
//! ```
//!
//! Installing resolves the manifest, installs the referenced `.mrpack` through
//! the normal installer, and records the manifest URL + installed version in a
//! local registry. Checking for updates re-fetches the manifest and compares
//! the `version`; updating re-installs the new `.mrpack` (a full sync).

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

use crate::State;

/// A Minecraft server that a seed adds to the instance's server list so every
/// client joins the same address.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SeedServer {
    pub address: String,
    #[serde(default)]
    pub name: Option<String>,
}

/// The manifest hosted by a seed publisher.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SeedManifest {
    pub name: String,
    pub version: String,
    #[serde(default)]
    pub changelog: Option<String>,
    /// URL of the `.mrpack` file to install for this version.
    pub mrpack: String,
    /// Optional integrity hash of the `.mrpack` (e.g. `"sha1:..."`).
    #[serde(default)]
    pub hash: Option<String>,
    #[serde(default)]
    pub icon: Option<String>,
    /// Servers to add to the instance so all clients join the same address.
    #[serde(default)]
    pub servers: Vec<SeedServer>,
}

/// A seed recorded for a locally-installed instance.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SeedEntry {
    pub seed_url: String,
    pub version: String,
    pub name: String,
}

/// The result of checking a seed for updates.
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct SeedUpdateInfo {
    pub seed_url: String,
    pub name: String,
    pub current_version: String,
    pub latest_version: String,
    pub changelog: Option<String>,
    pub has_update: bool,
    /// The `.mrpack` URL to install for the latest version.
    pub mrpack_url: String,
    pub icon: Option<String>,
}

fn registry_path(state: &State) -> PathBuf {
    state.directories.settings_dir.join("seeds.json")
}

async fn read_registry(
    state: &State,
) -> crate::Result<HashMap<String, SeedEntry>> {
    let path = registry_path(state);
    if !path.exists() {
        return Ok(HashMap::new());
    }
    let bytes = tokio::fs::read(&path).await?;
    Ok(serde_json::from_slice(&bytes).unwrap_or_default())
}

async fn write_registry(
    state: &State,
    registry: &HashMap<String, SeedEntry>,
) -> crate::Result<()> {
    let bytes = serde_json::to_vec_pretty(registry)?;
    tokio::fs::write(registry_path(state), bytes).await?;
    Ok(())
}

/// Fetch and parse a seed manifest from its URL.
pub async fn fetch_manifest(url: &str) -> crate::Result<SeedManifest> {
    let state = State::get().await?;
    let bytes = crate::util::fetch::fetch(
        url,
        None,
        None,
        None,
        &state.fetch_semaphore,
        &state.pool,
    )
    .await?;
    Ok(serde_json::from_slice(&bytes)?)
}

/// Record that an instance was installed from a seed. When `icon_url` is set,
/// the icon is downloaded and applied to the instance (best-effort).
pub async fn attach(
    instance_id: &str,
    seed_url: &str,
    version: &str,
    name: &str,
    icon_url: Option<&str>,
) -> crate::Result<()> {
    let state = State::get().await?;
    let mut registry = read_registry(&state).await?;
    registry.insert(
        instance_id.to_string(),
        SeedEntry {
            seed_url: seed_url.to_string(),
            version: version.to_string(),
            name: name.to_string(),
        },
    );
    write_registry(&state, &registry).await?;

    if let Err(err) =
        sync_instance_link(&state, instance_id, name, version).await
    {
        tracing::warn!("Failed to sync seed link for {instance_id}: {err}");
    }

    if let Some(icon_url) = icon_url
        && let Err(err) =
            apply_instance_icon(&state, instance_id, icon_url).await
    {
        tracing::warn!("Failed to apply seed icon for {instance_id}: {err}");
    }

    Ok(())
}

/// Keep the instance's modpack link in step with the seed: the content tab and
/// installation settings display the link's name/version, which otherwise stay
/// at whatever `.mrpack` filename the instance happened to be installed from.
async fn sync_instance_link(
    state: &State,
    instance_id: &str,
    name: &str,
    version: &str,
) -> crate::Result<()> {
    use crate::state::InstanceLink;

    let Some(metadata) =
        crate::state::instances::commands::get_instance_metadata(
            instance_id,
            &state.pool,
        )
        .await?
    else {
        return Ok(());
    };

    if let InstanceLink::ImportedModpack {
        name: existing_name,
        version_number: existing_version,
        filename,
        ..
    } = &metadata.link
        && existing_name.as_deref() == Some(name)
        && existing_version.as_deref() == Some(version)
        && filename.is_none()
    {
        return Ok(());
    }

    let link = match metadata.link {
        InstanceLink::ImportedModpack {
            project_id,
            version_id,
            ..
        } => InstanceLink::ImportedModpack {
            project_id,
            version_id,
            name: Some(name.to_string()),
            version_number: Some(version.to_string()),
            filename: None,
        },
        // Anything else (including an explicit unlink) is left alone.
        _ => return Ok(()),
    };

    crate::api::instance::edit(
        instance_id,
        crate::state::EditInstance {
            link: Some(link),
            ..Default::default()
        },
    )
    .await?;
    Ok(())
}

async fn apply_instance_icon(
    state: &State,
    instance_id: &str,
    icon_url: &str,
) -> crate::Result<()> {
    let bytes = crate::util::fetch::fetch(
        icon_url,
        None,
        None,
        None,
        &state.fetch_semaphore,
        &state.pool,
    )
    .await?;
    let filename = icon_url
        .split(['?', '#'])
        .next()
        .unwrap_or(icon_url)
        .rsplit('/')
        .find(|segment| !segment.is_empty())
        .unwrap_or("icon.png");
    let path = crate::util::fetch::write_cached_icon(
        filename,
        &state.directories.caches_dir(),
        bytes,
        &state.io_semaphore,
    )
    .await?;
    crate::api::instance::edit_icon(instance_id, Some(path.as_path())).await?;
    Ok(())
}

/// Get the recorded seed for an instance, if any.
pub async fn get(instance_id: &str) -> crate::Result<Option<SeedEntry>> {
    let state = State::get().await?;
    Ok(read_registry(&state).await?.get(instance_id).cloned())
}

/// Forget an instance's seed.
pub async fn remove(instance_id: &str) -> crate::Result<()> {
    let state = State::get().await?;
    let mut registry = read_registry(&state).await?;
    if registry.remove(instance_id).is_some() {
        write_registry(&state, &registry).await?;
    }
    Ok(())
}

/// Publish credentials for an instance's seed server, stored locally so the
/// user only enters them once. The key is kept in plaintext in the settings
/// directory (acceptable for a self-hosted tool).
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct PublishConfig {
    pub publish_url: String,
    pub seed_key: String,
}

fn publish_registry_path(state: &State) -> PathBuf {
    state.directories.settings_dir.join("seed_publish.json")
}

async fn read_publish_registry(
    state: &State,
) -> crate::Result<HashMap<String, PublishConfig>> {
    let path = publish_registry_path(state);
    if !path.exists() {
        return Ok(HashMap::new());
    }
    let bytes = tokio::fs::read(&path).await?;
    Ok(serde_json::from_slice(&bytes).unwrap_or_default())
}

async fn write_publish_registry(
    state: &State,
    registry: &HashMap<String, PublishConfig>,
) -> crate::Result<()> {
    let bytes = serde_json::to_vec_pretty(registry)?;
    tokio::fs::write(publish_registry_path(state), bytes).await?;
    Ok(())
}

/// Get the saved publish credentials for an instance, if any.
pub async fn get_publish_config(
    instance_id: &str,
) -> crate::Result<Option<PublishConfig>> {
    let state = State::get().await?;
    Ok(read_publish_registry(&state)
        .await?
        .get(instance_id)
        .cloned())
}

/// Save publish credentials for an instance.
pub async fn set_publish_config(
    instance_id: &str,
    config: &PublishConfig,
) -> crate::Result<()> {
    let state = State::get().await?;
    let mut registry = read_publish_registry(&state).await?;
    registry.insert(instance_id.to_string(), config.clone());
    write_publish_registry(&state, &registry).await
}

/// Export the instance to a `.mrpack` and publish it as a new version to its
/// seed server. When `remember` is set, the publish URL + key are saved locally
/// so future publishes don't need them re-entered.
#[allow(clippy::too_many_arguments)]
pub async fn publish(
    instance_id: &str,
    publish_url: &str,
    seed_key: &str,
    version: &str,
    changelog: Option<&str>,
    servers: Vec<SeedServer>,
    remember: bool,
) -> crate::Result<()> {
    let state = State::get().await?;
    let metadata =
        crate::api::instance::get(instance_id)
            .await?
            .ok_or_else(|| {
                crate::ErrorKind::InputError(format!(
                    "Unknown instance {instance_id}"
                ))
            })?;

    let candidates =
        crate::api::instance::get_pack_export_candidates(instance_id)
            .await?
            .iter()
            .map(|path| path.as_str().to_string())
            .collect::<Vec<_>>();

    let export_path = state
        .directories
        .caches_dir()
        .join(format!("seed-publish-{instance_id}.mrpack"));

    crate::api::instance::export_mrpack(
        instance_id,
        export_path.clone(),
        candidates,
        Some(version.to_string()),
        changelog.map(str::to_string),
        Some(metadata.instance.name.clone()),
    )
    .await?;

    let upload_result = upload_pack(
        &metadata,
        publish_url,
        seed_key,
        version,
        changelog,
        &servers,
        &export_path,
    )
    .await;

    let _ = tokio::fs::remove_file(&export_path).await;
    upload_result?;

    // The published pack now contains everything in the instance, so fold any
    // freshly added content into the modpack instead of leaving it flagged as
    // "additional content" locally. When anything changes, emit a sync event so
    // an open content tab refreshes instead of showing the stale grouping.
    match crate::state::instances::commands::rebaseline_content_as_modpack(
        instance_id,
        None,
        &state,
    )
    .await
    {
        Ok(updated) if updated > 0 => {
            let _ = crate::event::emit::emit_instance(
                instance_id,
                crate::event::InstancePayloadType::Synced,
            )
            .await;
        }
        Ok(_) => {}
        Err(err) => {
            tracing::warn!(
                "Failed to re-baseline content after publishing {instance_id}: {err}"
            );
        }
    }

    if remember {
        set_publish_config(
            instance_id,
            &PublishConfig {
                publish_url: publish_url.trim_end_matches('/').to_string(),
                seed_key: seed_key.to_string(),
            },
        )
        .await?;
    }

    Ok(())
}

async fn upload_pack(
    metadata: &crate::state::InstanceMetadata,
    publish_url: &str,
    seed_key: &str,
    version: &str,
    changelog: Option<&str>,
    servers: &[SeedServer],
    export_path: &std::path::Path,
) -> crate::Result<()> {
    let mrpack_bytes = tokio::fs::read(export_path).await?;

    let mut form = reqwest::multipart::Form::new()
        .part(
            "mrpack",
            reqwest::multipart::Part::bytes(mrpack_bytes)
                .file_name("pack.mrpack")
                .mime_str("application/zip")?,
        )
        .text("version", version.to_string())
        .text("name", metadata.instance.name.clone());

    if let Some(changelog) = changelog.filter(|c| !c.trim().is_empty()) {
        form = form.text("changelog", changelog.to_string());
    }
    if !servers.is_empty() {
        form = form.text("servers", serde_json::to_string(servers)?);
    }
    if let Some(icon_path) = &metadata.instance.icon_path
        && let Ok(icon_bytes) = tokio::fs::read(icon_path).await
    {
        let file_name = std::path::Path::new(icon_path)
            .file_name()
            .map(|f| f.to_string_lossy().to_string())
            .unwrap_or_else(|| "icon.png".to_string());
        form = form.part(
            "icon",
            reqwest::multipart::Part::bytes(icon_bytes).file_name(file_name),
        );
    }

    let response = reqwest::Client::new()
        .post(format!("{}/publish", publish_url.trim_end_matches('/')))
        .bearer_auth(seed_key)
        .multipart(form)
        .send()
        .await?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(crate::ErrorKind::OtherError(format!(
            "Seed server rejected publish ({status}): {body}"
        ))
        .into());
    }

    Ok(())
}

/// Re-fetch a seed's manifest and report whether an update is available.
pub async fn check_update(
    instance_id: &str,
) -> crate::Result<Option<SeedUpdateInfo>> {
    let Some(entry) = get(instance_id).await? else {
        return Ok(None);
    };
    let manifest = fetch_manifest(&entry.seed_url).await?;
    Ok(Some(SeedUpdateInfo {
        has_update: manifest.version != entry.version,
        seed_url: entry.seed_url,
        name: manifest.name,
        current_version: entry.version,
        latest_version: manifest.version,
        changelog: manifest.changelog,
        mrpack_url: manifest.mrpack,
        icon: manifest.icon,
    }))
}
