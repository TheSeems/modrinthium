<template>
	<NewModal ref="modal" header="Publish to seed" :on-hide="reset">
		<div class="flex w-[520px] max-w-full flex-col gap-4">
			<p class="m-0 text-sm text-secondary">
				Export this instance to a modpack and publish it as a new version on your seed server.
				Clients that installed the seed will be offered the update.
			</p>

			<div class="flex flex-col gap-1">
				<label class="text-sm font-semibold text-contrast" for="publish-url">Seed server URL</label>
				<input
					id="publish-url"
					v-model="publishUrl"
					type="url"
					class="w-full rounded-lg bg-button-bg px-3 py-2 text-contrast"
					placeholder="https://seed.example.com"
					:disabled="publishing"
				/>
			</div>

			<div class="flex flex-col gap-1">
				<label class="text-sm font-semibold text-contrast" for="seed-key">Seed key</label>
				<input
					id="seed-key"
					v-model="seedKey"
					type="password"
					class="w-full rounded-lg bg-button-bg px-3 py-2 text-contrast"
					placeholder="Secret publish key"
					:disabled="publishing"
				/>
			</div>

			<div class="flex flex-col gap-1">
				<label class="text-sm font-semibold text-contrast" for="version">Version</label>
				<input
					id="version"
					v-model="version"
					type="text"
					class="w-full rounded-lg bg-button-bg px-3 py-2 text-contrast"
					placeholder="1.4.0"
					:disabled="publishing"
				/>
			</div>

			<div class="flex flex-col gap-1">
				<label class="text-sm font-semibold text-contrast" for="changelog">Changelog (optional)</label>
				<textarea
					id="changelog"
					v-model="changelog"
					rows="3"
					class="w-full resize-y rounded-lg bg-button-bg px-3 py-2 text-contrast"
					placeholder="What changed in this version?"
					:disabled="publishing"
				/>
			</div>

			<div class="flex flex-col gap-2">
				<span class="text-sm font-semibold text-contrast">Servers (optional)</span>
				<p class="m-0 text-xs text-secondary">
					Added to every client's server list so everyone joins the same address.
				</p>
				<div v-for="(server, index) in servers" :key="index" class="flex items-center gap-2">
					<input
						v-model="server.name"
						type="text"
						class="w-1/3 rounded-lg bg-button-bg px-3 py-2 text-contrast"
						placeholder="Name"
						:disabled="publishing"
					/>
					<input
						v-model="server.address"
						type="text"
						class="flex-1 rounded-lg bg-button-bg px-3 py-2 text-contrast"
						placeholder="play.example.com"
						:disabled="publishing"
					/>
					<ButtonStyled>
						<button :disabled="publishing" @click="removeServer(index)"><XIcon /></button>
					</ButtonStyled>
				</div>
				<div>
					<ButtonStyled>
						<button :disabled="publishing" @click="addServer"><PlusIcon /> Add server</button>
					</ButtonStyled>
				</div>
			</div>

			<label class="flex items-center gap-2 text-sm text-primary">
				<input v-model="remember" type="checkbox" :disabled="publishing" />
				Remember this server URL and key for this instance
			</label>

			<div class="flex justify-end gap-2">
				<ButtonStyled>
					<button :disabled="publishing" @click="modal?.hide()"><XIcon /> Cancel</button>
				</ButtonStyled>
				<ButtonStyled color="brand">
					<button :disabled="!canPublish || publishing" @click="doPublish">
						<SpinnerIcon v-if="publishing" class="animate-spin" />
						<ArrowBigUpDashIcon v-else />
						Publish
					</button>
				</ButtonStyled>
			</div>
		</div>
	</NewModal>
</template>

<script setup lang="ts">
import { ArrowBigUpDashIcon, PlusIcon, SpinnerIcon, XIcon } from '@modrinth/assets'
import { ButtonStyled, injectNotificationManager, NewModal } from '@modrinth/ui'
import { computed, ref } from 'vue'

import {
	seed_attach,
	seed_fetch_manifest,
	seed_get,
	seed_publish,
	seed_publish_config_get,
} from '@/helpers/seed'

const { handleError } = injectNotificationManager()

const modal = ref<InstanceType<typeof NewModal>>()
const instanceId = ref('')
const publishUrl = ref('')
const seedKey = ref('')
const version = ref('')
const changelog = ref('')
const servers = ref<{ name: string; address: string }[]>([])
const remember = ref(true)
const publishing = ref(false)

const canPublish = computed(
	() => !!publishUrl.value.trim() && !!seedKey.value.trim() && !!version.value.trim(),
)

async function show(id: string) {
	reset()
	instanceId.value = id
	modal.value?.show()
	const config = await seed_publish_config_get(id).catch(() => null)
	if (config) {
		publishUrl.value = config.publishUrl
		seedKey.value = config.seedKey
	}

	// If the instance came from a seed, prefill from it: the publish URL is the
	// manifest URL minus manifest.json, and version/servers default to what the
	// seed currently has. Saved credentials above still take precedence.
	const entry = await seed_get(id).catch(() => null)
	if (!entry) return
	if (!publishUrl.value) {
		publishUrl.value = entry.seed_url.replace(/\/manifest\.json$/i, '').replace(/\/+$/, '')
	}
	if (!version.value) {
		version.value = entry.version
	}
	const manifest = await seed_fetch_manifest(entry.seed_url).catch(() => null)
	if (!manifest) return
	if (!version.value || version.value === entry.version) {
		version.value = manifest.version
	}
	if (servers.value.length === 0) {
		servers.value = (manifest.servers ?? []).map((server) => ({
			name: server.name ?? '',
			address: server.address,
		}))
	}
}

function reset() {
	instanceId.value = ''
	publishUrl.value = ''
	seedKey.value = ''
	version.value = ''
	changelog.value = ''
	servers.value = []
	remember.value = true
	publishing.value = false
}

function addServer() {
	servers.value.push({ name: '', address: '' })
}

function removeServer(index: number) {
	servers.value.splice(index, 1)
}

async function doPublish() {
	if (publishing.value || !canPublish.value) return
	publishing.value = true
	try {
		const cleanServers = servers.value
			.filter((server) => server.address.trim())
			.map((server) => ({
				address: server.address.trim(),
				name: server.name.trim() || null,
			}))
		await seed_publish(
			instanceId.value,
			publishUrl.value.trim(),
			seedKey.value.trim(),
			version.value.trim(),
			changelog.value.trim() || null,
			cleanServers,
			remember.value,
		)
		// If this instance is itself attached to the seed we just published to,
		// bump the local record so it isn't prompted to "update" to its own release.
		const entry = await seed_get(instanceId.value).catch(() => null)
		if (
			entry &&
			entry.seed_url.replace(/\/manifest\.json$/i, '').replace(/\/+$/, '') ===
				publishUrl.value.trim().replace(/\/+$/, '')
		) {
			await seed_attach(instanceId.value, entry.seed_url, version.value.trim(), entry.name).catch(
				(err) => console.error('Failed to sync local seed version after publish:', err),
			)
		}
		modal.value?.hide()
	} catch (err) {
		handleError(err)
	} finally {
		publishing.value = false
	}
}

defineExpose({ show })
</script>
