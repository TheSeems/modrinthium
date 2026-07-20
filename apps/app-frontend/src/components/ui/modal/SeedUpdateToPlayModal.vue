<template>
	<NewModal ref="modal" header="Update available">
		<div class="flex w-[440px] max-w-full flex-col gap-4">
			<p class="m-0 text-primary">
				<span class="font-bold text-contrast">{{ info?.name }}</span> has an update ready (v{{
					info?.currentVersion
				}}
				&rarr; v{{ info?.latestVersion }}). Update now to stay in sync with everyone else.
			</p>
			<p
				v-if="info?.changelog"
				class="m-0 max-h-40 overflow-y-auto whitespace-pre-wrap rounded-lg bg-bg p-3 text-sm text-primary"
			>
				{{ info.changelog }}
			</p>
			<div class="flex justify-end gap-2">
				<ButtonStyled>
					<button :disabled="updating" @click="playAnyway"><PlayIcon /> Play anyway</button>
				</ButtonStyled>
				<ButtonStyled color="brand">
					<button :disabled="updating" @click="updateAndPlay">
						<SpinnerIcon v-if="updating" class="animate-spin" />
						<DownloadIcon v-else />
						Update &amp; play
					</button>
				</ButtonStyled>
			</div>
		</div>
	</NewModal>
</template>

<script setup lang="ts">
import { DownloadIcon, PlayIcon, SpinnerIcon } from '@modrinth/assets'
import { ButtonStyled, injectNotificationManager, NewModal } from '@modrinth/ui'
import { ref } from 'vue'

import { type SeedUpdateInfo, updateFromSeed } from '@/helpers/seed'

const { handleError } = injectNotificationManager()

const emit = defineEmits<{ play: [] }>()

const modal = ref<InstanceType<typeof NewModal>>()
const instanceId = ref('')
const info = ref<SeedUpdateInfo | null>(null)
const updating = ref(false)

function show(id: string, updateInfo: SeedUpdateInfo) {
	instanceId.value = id
	info.value = updateInfo
	modal.value?.show()
}

function playAnyway() {
	modal.value?.hide()
	emit('play')
}

async function updateAndPlay() {
	if (updating.value) return
	updating.value = true
	try {
		await updateFromSeed(instanceId.value)
		modal.value?.hide()
		emit('play')
	} catch (err) {
		handleError(err)
	} finally {
		updating.value = false
	}
}

defineExpose({ show })
</script>
