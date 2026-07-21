<template>
	<NewModal ref="modal" header="Install from link" :closable="true" :on-hide="reset">
		<div class="flex w-[480px] max-w-full flex-col gap-4">
			<p class="m-0 text-sm text-secondary">
				Paste a Modrinthium seed link to install a modpack that isn't published on Modrinth. You
				can check it for updates later from the instance's settings.
			</p>

			<div class="flex flex-col gap-1">
				<label class="text-sm font-semibold text-contrast" for="seed-url">Seed link</label>
				<input
					id="seed-url"
					v-model="url"
					type="url"
					class="w-full rounded-lg bg-button-bg px-3 py-2 text-contrast"
					placeholder="https://example.com/seed.json"
					:disabled="loading || installing"
					@keydown.enter="loadManifest"
				/>
			</div>

			<div
				v-if="manifest"
				class="flex items-center gap-3 rounded-xl border border-solid border-surface-5 bg-bg p-3"
			>
				<Avatar :src="manifest.icon ?? undefined" size="48px" />
				<div class="flex min-w-0 flex-col">
					<span class="truncate font-bold text-contrast">{{ manifest.name }}</span>
					<span class="text-sm text-secondary">Version {{ manifest.version }}</span>
				</div>
			</div>

			<div v-if="manifest?.changelog" class="flex flex-col gap-1">
				<span class="text-sm font-semibold text-contrast">What's new</span>
				<p
					class="m-0 max-h-40 overflow-y-auto whitespace-pre-wrap rounded-lg bg-bg p-3 text-sm text-primary"
				>
					{{ manifest.changelog }}
				</p>
			</div>

			<div class="flex justify-end gap-2">
				<ButtonStyled>
					<button :disabled="installing" @click="hide"><XIcon /> Cancel</button>
				</ButtonStyled>
				<ButtonStyled v-if="!manifest" color="brand">
					<button :disabled="!url || loading" @click="loadManifest">
						<SpinnerIcon v-if="loading" class="animate-spin" />
						<SearchIcon v-else />
						Load
					</button>
				</ButtonStyled>
				<ButtonStyled v-else color="brand">
					<button :disabled="installing" @click="doInstall">
						<SpinnerIcon v-if="installing" class="animate-spin" />
						<DownloadIcon v-else />
						Install
					</button>
				</ButtonStyled>
			</div>
		</div>
	</NewModal>
</template>

<script setup lang="ts">
import { DownloadIcon, SearchIcon, SpinnerIcon, XIcon } from '@modrinth/assets'
import { Avatar, ButtonStyled, injectNotificationManager, NewModal } from '@modrinth/ui'
import { ref } from 'vue'
import { useRouter } from 'vue-router'

import { installJobInstanceId } from '@/helpers/install'
import { installFromSeed, type SeedManifest, seed_fetch_manifest } from '@/helpers/seed'

const { handleError } = injectNotificationManager()
const router = useRouter()

const modal = ref<InstanceType<typeof NewModal>>()
const url = ref('')
const manifest = ref<SeedManifest | null>(null)
const loading = ref(false)
const installing = ref(false)

async function show(prefillUrl?: string) {
	reset()
	modal.value?.show()
	if (prefillUrl) {
		url.value = prefillUrl
		await loadManifest()
	}
}

function hide() {
	modal.value?.hide()
}

function reset() {
	url.value = ''
	manifest.value = null
	loading.value = false
	installing.value = false
}

async function loadManifest() {
	if (!url.value || loading.value) return
	loading.value = true
	try {
		manifest.value = await seed_fetch_manifest(url.value.trim())
	} catch (err) {
		handleError(err)
	} finally {
		loading.value = false
	}
}

async function doInstall() {
	const resolved = manifest.value
	if (!resolved || installing.value) return
	installing.value = true
	try {
		const job = await installFromSeed(url.value.trim(), resolved)
		hide()
		const instanceId = installJobInstanceId(job)
		await router.push(instanceId ? `/instance/${encodeURIComponent(instanceId)}` : '/library')
	} catch (err) {
		handleError(err)
	} finally {
		installing.value = false
	}
}

defineExpose({ show, hide })
</script>
