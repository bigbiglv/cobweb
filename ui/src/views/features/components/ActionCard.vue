<script setup lang="ts">
import { computed } from 'vue'
import { Power, RefreshCw, RotateCw, TestTube2 } from 'lucide-vue-next'
import { Button } from '../../../components/ui/button/index'
import type { ActionFeatureDefinition } from '../types'

interface Props {
  feature: ActionFeatureDefinition
  pending: boolean
  refreshable?: boolean
  refreshing?: boolean
}

interface Emits {
  execute: [feature: ActionFeatureDefinition]
  cancel: [feature: ActionFeatureDefinition]
  refresh: [feature: ActionFeatureDefinition]
}

const { feature, pending, refreshable = false, refreshing = false } = defineProps<Props>()
const emit = defineEmits<Emits>()

const icon = computed(() => {
  if (feature.featureKey === 'shutdown') return Power
  if (feature.featureKey === 'restart') return RotateCw
  return TestTube2
})

const buttonVariant = computed(() =>
  pending ? 'secondary' : feature.control.tone === 'danger' ? 'destructive' : 'default',
)
</script>

<template>
  <article class="rounded-[1.75rem] border border-border/70 bg-background/70 p-5">
    <div class="flex h-full flex-col gap-5">
      <div class="flex items-center justify-between gap-3">
        <div class="flex items-center gap-3">
          <div class="flex size-10 items-center justify-center rounded-full bg-muted text-foreground">
            <component :is="icon" class="size-5" />
          </div>
          <h3 class="text-lg font-semibold text-foreground">
            {{ feature.title }}
          </h3>
        </div>

        <Button
          v-if="refreshable"
          variant="outline"
          size="icon"
          class="rounded-full"
          :disabled="refreshing || pending"
          @click="emit('refresh', feature)"
        >
          <RefreshCw class="size-4" :class="{ 'animate-spin': refreshing }" />
        </Button>
      </div>

      <Button
        :variant="buttonVariant"
        class="mt-auto rounded-full"
        :disabled="pending"
        @click="emit('execute', feature)"
      >
        <component :is="icon" class="size-4" />
        <span>{{ pending ? '执行中' : feature.control.buttonText }}</span>
      </Button>
    </div>
  </article>
</template>
