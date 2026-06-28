<script setup lang="ts">
import { ref, onMounted, watch } from "vue";
import { getAudioDevices, setAudioDeviceVolume } from "../api";
import type { AudioOutputDevice } from "../types";
import ActionIconButton from "./ActionIconButton.vue";

const props = defineProps<{
  open: boolean;
}>();

const emit = defineEmits<{
  close: [];
}>();

const devices = ref<AudioOutputDevice[]>([]);
const loading = ref(false);
const errorMsg = ref("");

async function loadDevices() {
  try {
    loading.value = true;
    errorMsg.value = "";
    devices.value = await getAudioDevices();
  } catch (error) {
    errorMsg.value = error instanceof Error ? error.message : String(error);
  } finally {
    loading.value = false;
  }
}

let volumeTimer: number | undefined;
function onVolumeChange(device: AudioOutputDevice) {
  clearTimeout(volumeTimer);
  volumeTimer = window.setTimeout(async () => {
    try {
      devices.value = await setAudioDeviceVolume(device.id, Number(device.volume));
    } catch (error) {
      errorMsg.value = error instanceof Error ? error.message : String(error);
    }
  }, 200);
}

watch(
  () => props.open,
  (isOpen) => {
    if (isOpen) {
      loadDevices();
    }
  }
);
</script>

<template>
  <Teleport to="body">
    <Transition name="dialog-fade">
      <div v-if="open" class="confirm-layer" role="presentation" @click.self="$emit('close')">
        <section class="confirm-dialog audio-device-modal" role="dialog" aria-modal="true" aria-label="音频设备音量控制">
          <div class="confirm-title">
            <span>音频输出设备</span>
            <ActionIconButton icon="refresh" label="刷新" floating @click="loadDevices" />
          </div>
          
          <div class="device-list">
            <p v-if="loading && devices.length === 0" class="confirm-message">加载中...</p>
            <p v-else-if="errorMsg" class="confirm-message danger">{{ errorMsg }}</p>
            <p v-else-if="devices.length === 0" class="confirm-message">未找到音频输出设备</p>
            
            <div v-for="device in devices" :key="device.id" class="device-item">
              <div class="device-name">
                <span :class="{ 'default-device': device.isDefault }">{{ device.name }}</span>
                <span v-if="device.isDefault" class="device-tag">默认</span>
              </div>
              <div class="device-volume-control">
                <input
                  type="range"
                  class="volume-range"
                  min="0"
                  max="100"
                  step="1"
                  v-model.number="device.volume"
                  @input="onVolumeChange(device)"
                />
                <span class="volume-value">{{ device.volume }}%</span>
              </div>
            </div>
          </div>
          

        </section>
      </div>
    </Transition>
  </Teleport>
</template>

<style scoped>
.audio-device-modal {
  width: 90%;
  max-width: 400px;
  max-height: 80vh;
  display: flex;
  flex-direction: column;
}

.confirm-title {
  display: flex;
  justify-content: space-between;
  align-items: center;
}

.device-list {
  flex-grow: 1;
  overflow-y: auto;
  margin: 1rem 0;
  display: flex;
  flex-direction: column;
  gap: 1.25rem;
}

.device-item {
  display: flex;
  flex-direction: column;
  gap: 0.5rem;
}

.device-name {
  font-size: 0.95rem;
  font-weight: 500;
  display: flex;
  align-items: center;
  gap: 0.5rem;
}

.device-name .default-device {
  font-weight: 600;
  color: var(--accent-base);
}

.device-tag {
  font-size: 0.7rem;
  padding: 0.1rem 0.4rem;
  border-radius: 4px;
  background-color: var(--accent-transparent);
  color: var(--accent-base);
}

.device-volume-control {
  display: flex;
  align-items: center;
  gap: 1rem;
}

.volume-range {
  flex-grow: 1;
  accent-color: var(--primary);
}

.volume-value {
  font-size: 0.85rem;
  min-width: 3ch;
  text-align: right;
  color: var(--text-secondary);
}

.danger {
  color: var(--danger-base);
}
</style>
