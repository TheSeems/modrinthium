use crate::api::Result;
use theseus::seed::{
    PublishConfig, SeedEntry, SeedManifest, SeedServer, SeedUpdateInfo,
};

pub fn init<R: tauri::Runtime>() -> tauri::plugin::TauriPlugin<R> {
    tauri::plugin::Builder::new("seed")
        .invoke_handler(tauri::generate_handler![
            seed_fetch_manifest,
            seed_attach,
            seed_get,
            seed_remove,
            seed_check_update,
            seed_publish,
            seed_publish_config_get,
            seed_publish_config_set,
        ])
        .build()
}

/// Fetch and parse a seed manifest from its URL.
#[tauri::command]
pub async fn seed_fetch_manifest(url: String) -> Result<SeedManifest> {
    Ok(theseus::seed::fetch_manifest(&url).await?)
}

/// Record that an instance was installed from a seed.
#[tauri::command]
pub async fn seed_attach(
    instance_id: String,
    seed_url: String,
    version: String,
    name: String,
    icon_url: Option<String>,
) -> Result<()> {
    Ok(theseus::seed::attach(
        &instance_id,
        &seed_url,
        &version,
        &name,
        icon_url.as_deref(),
    )
    .await?)
}

/// Get the recorded seed for an instance, if any.
#[tauri::command]
pub async fn seed_get(instance_id: String) -> Result<Option<SeedEntry>> {
    Ok(theseus::seed::get(&instance_id).await?)
}

/// Forget an instance's seed.
#[tauri::command]
pub async fn seed_remove(instance_id: String) -> Result<()> {
    Ok(theseus::seed::remove(&instance_id).await?)
}

/// Re-fetch a seed's manifest and report whether an update is available.
#[tauri::command]
pub async fn seed_check_update(
    instance_id: String,
) -> Result<Option<SeedUpdateInfo>> {
    Ok(theseus::seed::check_update(&instance_id).await?)
}

/// Export the instance to a `.mrpack` and publish it as a new seed version.
#[tauri::command]
pub async fn seed_publish(
    instance_id: String,
    publish_url: String,
    seed_key: String,
    version: String,
    changelog: Option<String>,
    servers: Vec<SeedServer>,
    remember: bool,
) -> Result<()> {
    Ok(theseus::seed::publish(
        &instance_id,
        &publish_url,
        &seed_key,
        &version,
        changelog.as_deref(),
        servers,
        remember,
    )
    .await?)
}

/// Get the saved publish credentials for an instance, if any.
#[tauri::command]
pub async fn seed_publish_config_get(
    instance_id: String,
) -> Result<Option<PublishConfig>> {
    Ok(theseus::seed::get_publish_config(&instance_id).await?)
}

/// Save publish credentials for an instance.
#[tauri::command]
pub async fn seed_publish_config_set(
    instance_id: String,
    publish_url: String,
    seed_key: String,
) -> Result<()> {
    Ok(theseus::seed::set_publish_config(
        &instance_id,
        &PublishConfig {
            publish_url,
            seed_key,
        },
    )
    .await?)
}
