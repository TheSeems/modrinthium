<template>
	<div v-if="seed" class="mb-4 rounded-xl border border-solid border-surface-5 bg-bg-raised p-4">
		<div class="flex items-center gap-3">
			<LinkIcon class="size-5 shrink-0 text-secondary" />
			<div class="flex min-w-0 flex-col">
				<span class="truncate font-bold text-contrast">{{ seed.seed_url }}</span>
				<span class="text-sm text-secondary">Seed &middot; installed v{{ seed.version }}</span>
			</div>
			<div class="ml-auto flex items-center gap-2">
				<span v-if="upToDate" class="flex items-center gap-1 text-sm text-secondary">
					<CheckIcon class="size-4 text-brand" /> Up to date
				</span>
				<ButtonStyled>
					<button :disabled="checking || updating" @click="check">
						<SpinnerIcon v-if="checking" class="animate-spin" />
						<RefreshCwIcon v-else />
						Check for updates
					</button>
				</ButtonStyled>
			</div>
		</div>

		<div
			v-if="updateInfo?.hasUpdate"
			class="mt-4 flex flex-col gap-3 border-0 border-t border-solid border-surface-5 pt-4"
		>
			<div class="flex items-center gap-2">
				<ArrowBigUpDashIcon class="size-5 text-brand" />
				<span class="font-semibold text-contrast">
					Update available: v{{ updateInfo.currentVersion }} &rarr; v{{ updateInfo.latestVersion }}
				</span>
			</div>
			<p
				v-if="updateInfo.changelog"
				class="m-0 max-h-40 overflow-y-auto whitespace-pre-wrap rounded-lg bg-bg p-3 text-sm text-primary"
			>
				{{ updateInfo.changelog }}
			</p>
			<p class="m-0 flex items-center gap-1 text-sm text-orange">
				<TriangleAlertIcon class="size-4 shrink-0" />
				Updating replaces this instance's contents to match the seed. Mods you added manually will
				be removed.
			</p>
			<div class="flex justify-end">
				<ButtonStyled color="brand">
					<button :disabled="updating" @click="update">
						<SpinnerIcon v-if="updating" class="animate-spin" />
						<DownloadIcon v-else />
						Update now
					</button>
				</ButtonStyled>
			</div>
		</div>
	</div>
</template>

<script setup lang="ts">
import {
	ArrowBigUpDashIcon,
	CheckIcon,
	DownloadIcon,
	LinkIcon,
	RefreshCwIcon,
	SpinnerIcon,
	TriangleAlertIcon,
} from '@modrinth/assets'
import { ButtonStyled, injectNotificationManager } from '@modrinth/ui'
import { onMounted, onUnmounted, ref, watch } from 'vue'

import { instance_listener } from '@/helpers/events.js'
import {
	seed_attach,
	seed_check_update,
	seed_get,
	type SeedEntry,
	type SeedUpdateInfo,
	updateFromSeed,
} from '@/helpers/seed'

const props = defineProps<{ instanceId: string }>()
const { handleError } = injectNotificationManager()

const seed = ref<SeedEntry | null>(null)
const updateInfo = ref<SeedUpdateInfo | null>(null)
const checking = ref(false)
const updating = ref(false)
const upToDate = ref(false)

async function load() {
	seed.value = await seed_get(props.instanceId).catch(() => null)
	if (seed.value) {
		// Re-attach (without the icon) to heal the instance's modpack link if it
		// drifted — a no-op when everything already matches.
		void seed_attach(
			props.instanceId,
			seed.value.seed_url,
			seed.value.version,
			seed.value.name,
		).catch((err) => console.error('Failed to sync seed link:', err))
		// Auto-check when the instance opens so pending updates surface immediately.
		void check()
	}
}

let unlistenInstance: (() => void) | undefined

onMounted(async () => {
	await load()
	// Publishing bumps the recorded seed version via an instance edit; re-read it
	// so "installed vX" reflects the new version without a tab switch.
	unlistenInstance = await instance_listener(
		async (event: { event: string; instance_id: string }) => {
			if (
				event.instance_id === props.instanceId &&
				(event.event === 'synced' || event.event === 'edited')
			) {
				seed.value = await seed_get(props.instanceId).catch(() => seed.value)
			}
		},
	)
})

onUnmounted(() => {
	unlistenInstance?.()
})

watch(
	() => props.instanceId,
	() => {
		updateInfo.value = null
		upToDate.value = false
		void load()
	},
)

async function check() {
	if (checking.value) return
	checking.value = true
	upToDate.value = false
	try {
		updateInfo.value = await seed_check_update(props.instanceId)
		if (updateInfo.value && !updateInfo.value.hasUpdate) {
			upToDate.value = true
		}
	} catch (err) {
		handleError(err)
	} finally {
		checking.value = false
	}
}

async function update() {
	if (updating.value) return
	updating.value = true
	try {
		await updateFromSeed(props.instanceId)
		updateInfo.value = null
		upToDate.value = true
		await load()
	} catch (err) {
		handleError(err)
	} finally {
		updating.value = false
	}
}
</script>
