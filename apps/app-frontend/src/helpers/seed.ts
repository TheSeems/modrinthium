import { invoke } from '@tauri-apps/api/core'

import {
	install_create_modpack_instance,
	type InstallJobSnapshot,
	install_pack_to_existing_instance,
	installJobInstanceId,
	wait_for_install_job,
} from './install'
import {
	add_server_to_instance,
	get_instance_worlds,
	isServerWorld,
	normalizeServerAddress,
} from './worlds'

/** A Minecraft server a seed adds to the instance so all clients join the same address. */
export interface SeedServer {
	address: string
	name?: string | null
}

/** The manifest a seed publisher hosts at a stable URL. */
export interface SeedManifest {
	name: string
	version: string
	changelog?: string | null
	/** URL of the `.mrpack` file for this version. */
	mrpack: string
	hash?: string | null
	icon?: string | null
	servers?: SeedServer[]
}

/** A seed recorded for a locally-installed instance. */
export interface SeedEntry {
	seed_url: string
	version: string
	name: string
}

/** The result of checking a seed for updates. */
export interface SeedUpdateInfo {
	seedUrl: string
	name: string
	currentVersion: string
	latestVersion: string
	changelog?: string | null
	hasUpdate: boolean
	/** The `.mrpack` URL to install for the latest version. */
	mrpackUrl: string
	icon?: string | null
}

export async function seed_fetch_manifest(url: string) {
	return await invoke<SeedManifest>('plugin:seed|seed_fetch_manifest', { url })
}

export async function seed_attach(
	instanceId: string,
	seedUrl: string,
	version: string,
	name: string,
	iconUrl?: string | null,
) {
	return await invoke<void>('plugin:seed|seed_attach', {
		instanceId,
		seedUrl,
		version,
		name,
		iconUrl,
	})
}

export async function seed_get(instanceId: string) {
	return await invoke<SeedEntry | null>('plugin:seed|seed_get', { instanceId })
}

export async function seed_remove(instanceId: string) {
	return await invoke<void>('plugin:seed|seed_remove', { instanceId })
}

export async function seed_check_update(instanceId: string) {
	return await invoke<SeedUpdateInfo | null>('plugin:seed|seed_check_update', { instanceId })
}

/** Saved credentials for publishing to a seed server (entered once per instance). */
export interface PublishConfig {
	publishUrl: string
	seedKey: string
}

export async function seed_publish_config_get(instanceId: string) {
	return await invoke<PublishConfig | null>('plugin:seed|seed_publish_config_get', { instanceId })
}

export async function seed_publish_config_set(
	instanceId: string,
	publishUrl: string,
	seedKey: string,
) {
	return await invoke<void>('plugin:seed|seed_publish_config_set', {
		instanceId,
		publishUrl,
		seedKey,
	})
}

/** Export the instance to a .mrpack and publish it as a new seed version. */
export async function seed_publish(
	instanceId: string,
	publishUrl: string,
	seedKey: string,
	version: string,
	changelog: string | null,
	servers: SeedServer[],
	remember: boolean,
) {
	return await invoke<void>('plugin:seed|seed_publish', {
		instanceId,
		publishUrl,
		seedKey,
		version,
		changelog,
		servers,
		remember,
	})
}

async function applySeedServers(instanceId: string, manifest: SeedManifest) {
	const desired = manifest.servers ?? []
	if (desired.length === 0) return

	// Dedupe against servers the instance already has (e.g. from a previous
	// install/update or an override-shipped servers.dat) so updates don't pile
	// up duplicate entries.
	const existingAddresses = new Set<string>()
	try {
		for (const world of await get_instance_worlds(instanceId)) {
			if (isServerWorld(world)) {
				existingAddresses.add(normalizeServerAddress(world.address))
			}
		}
	} catch (err) {
		console.error('Failed to read existing worlds while applying seed servers:', err)
	}

	for (const server of desired) {
		const key = normalizeServerAddress(server.address)
		if (existingAddresses.has(key)) continue
		try {
			await add_server_to_instance(instanceId, server.name ?? manifest.name, server.address, 'prompt')
			existingAddresses.add(key)
		} catch (err) {
			console.error(`Failed to add seed server ${server.address}:`, err)
		}
	}
}

/**
 * Start installing an instance from a seed. Returns the install job immediately;
 * the seed is recorded and its icon/servers applied in the background once the
 * job finishes, so the UI isn't blocked while mods download.
 */
export async function installFromSeed(
	seedUrl: string,
	manifest?: SeedManifest,
): Promise<InstallJobSnapshot> {
	const resolved = manifest ?? (await seed_fetch_manifest(seedUrl))

	// The instance is initially named after the .mrpack file (often just a
	// version like "1.0.8"), so have the install pipeline rename it to the
	// manifest name once it finishes.
	const job = await install_create_modpack_instance(
		{ type: 'fromUrl', url: resolved.mrpack },
		{ name: resolved.name },
	)

	// Attach the seed right away — the instance id is known as soon as the job
	// starts — so the update banner and icon are there even while mods are
	// still downloading.
	const instanceId = installJobInstanceId(job)
	if (instanceId) {
		await seed_attach(instanceId, seedUrl, resolved.version, resolved.name, resolved.icon).catch(
			(err) => console.error('Failed to attach seed:', err),
		)
	}

	void wait_for_install_job(job.job_id)
		.then(async (finished) => {
			const finishedId = installJobInstanceId(finished) ?? instanceId
			if (!finishedId) return
			// Re-attach after the install finishes: the install pipeline rewrites
			// the instance's modpack link from the .mrpack file, and attach syncs
			// it back to the seed's name/version.
			await seed_attach(finishedId, seedUrl, resolved.version, resolved.name, resolved.icon)
			await applySeedServers(finishedId, resolved)
		})
		.catch((err) => console.error('Seed post-install steps failed:', err))

	return job
}

/**
 * Full-sync an existing seeded instance to its latest version and re-record the
 * installed version. Returns the version that was installed.
 */
export async function updateFromSeed(instanceId: string): Promise<string> {
	const info = await seed_check_update(instanceId)
	if (!info) {
		throw new Error('Instance is not linked to a seed')
	}

	// Re-fetch the full manifest so the update also picks up server/icon changes
	// (SeedUpdateInfo doesn't carry the server list).
	const manifest = await seed_fetch_manifest(info.seedUrl)

	const job = await install_pack_to_existing_instance(instanceId, {
		type: 'fromUrl',
		url: manifest.mrpack,
	})
	await wait_for_install_job(job.job_id)

	await seed_attach(instanceId, info.seedUrl, manifest.version, manifest.name, manifest.icon)
	await applySeedServers(instanceId, manifest)
	return manifest.version
}
