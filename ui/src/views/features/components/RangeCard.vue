<script setup lang="ts">
import { computed } from 'vue'
import { RefreshCw, Volume2 } from 'lucide-vue-next'
import { Button } from '../../../components/ui/button/index'
import type { RangeFeatureDefinition } from '../types'

interface Props {
  feature: RangeFeatureDefinition
  value: number
  pending: boolean
  refreshing: boolean
}

interface Emits {
  'update:value': [value: number]
  apply: [feature: RangeFeatureDefinition]
  refresh: []
}

const { feature, value, pending, refreshing } = defineProps<Props>()
const emit = defineEmits<Emits>()

const displayValue = computed(() => `${value}${feature.control.unit}`)
</script>

<template>
  <article class="rounded-[1.75rem] border border-border/70 bg-background/70 p-5">
    <div class="flex flex-col gap-5">
      <div class="flex items-center justify-between gap-3">
        <div class="flex items-center gap-3">
          <div class="flex size-10 items-center justify-center rounded-full bg-muted text-foreground">
            <Volume2 class="size-5" />
          </div>
          <h3 class="text-lg font-semibold text-foreground">
            {{ feature.title }}
          </h3>
        </div>
        <span class="text-lg font-semibold text-primary">{{ displayValue }}</span>
      </div>

      <input
        class="range-input"
        type="range"
        :min="feature.control.min"
        :max="feature.control.max"
        :step="feature.control.step"
        :value="value"
        :disabled="pending"
        @input="emit('update:value', Number(($event.target as HTMLInputElement).value))"
      >

      <div class="flex justify-end gap-2">
        <Button
          variant="outline"
          size="icon"
          class="rounded-full"
          :disabled="refreshing || pending"
          @click="emit('refresh')"
        >
          <RefreshCw class="size-4" :class="{ 'animate-spin': refreshing }" />
        </Button>

        <Button class="rounded-full" :disabled="pending" @click="emit('apply', feature)">
          {{ pending ? '执行中' : feature.control.actionText }}
        </Button>
      </div>
    </div>
  </article>
</template>

<style scoped>
.range-input {
  width: 100%;
  accent-color: currentColor;
}
</style>
