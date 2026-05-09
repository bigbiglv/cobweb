<script setup lang="ts">
import {
  ChevronLeft,
  ChevronRight,
  ClipboardCopy,
  Download,
  FileText,
  Trash2,
} from 'lucide-vue-next'
import { computed, ref, watch } from 'vue'
import { Badge } from '../../../components/ui/badge'
import { Button } from '../../../components/ui/button'
import type { ClipboardSyncMessage, ClipboardSyncAttachment, ClipboardSyncSource } from '../types.ts'

interface Props {
  message: ClipboardSyncMessage
  serverUrl: string
}

const props = defineProps<Props>()

const emits = defineEmits<{
  'copy-text': [message: ClipboardSyncMessage]
  'copy-images': [message: ClipboardSyncMessage, attachments: ClipboardSyncAttachment[]]
  delete: [message: ClipboardSyncMessage]
}>()

const maxVisibleImages = 7
const imageWindowStart = ref(0)

const imageAttachments = computed(() => props.message.attachments.filter(isImage))
const fileAttachments = computed(() => props.message.attachments.filter((attachment) => !isImage(attachment)))
const visibleImageAttachments = computed(() => {
  return imageAttachments.value.slice(imageWindowStart.value, imageWindowStart.value + maxVisibleImages)
})
const canShiftImagesPrev = computed(() => imageWindowStart.value > 0)
const canShiftImagesNext = computed(() => imageWindowStart.value + maxVisibleImages < imageAttachments.value.length)

watch(
  () => [props.message.messageId, imageAttachments.value.length],
  () => {
    imageWindowStart.value = 0
  },
  { immediate: true },
)

function fileUrl(message: ClipboardSyncMessage, attachment: ClipboardSyncAttachment) {
  return `${props.serverUrl}/web/api/sync/files/${encodeURIComponent(message.messageId)}/${encodeURIComponent(attachment.attachmentId)}`
}

function formatTime(timestamp: number) {
  return new Intl.DateTimeFormat('zh-CN', {
    month: 'short',
    day: 'numeric',
    hour: '2-digit',
    minute: '2-digit',
    second: '2-digit',
  }).format(timestamp)
}

function formatFileSize(size: number) {
  if (size < 1024)
    return `${size} B`
  if (size < 1024 * 1024)
    return `${(size / 1024).toFixed(1)} KB`
  return `${(size / 1024 / 1024).toFixed(1)} MB`
}

function isImage(attachment: ClipboardSyncAttachment) {
  return attachment.mimeType?.startsWith('image/') ?? false
}

function sourceName(source: ClipboardSyncSource) {
  const device = source.deviceModel || source.deviceName || (source.kind === 'pc' ? 'PC' : 'Web 设备')
  return [device, source.platform, source.browser, source.ip].filter(Boolean).join(' · ')
}

function sourceBadge(source: ClipboardSyncSource) {
  return source.kind === 'pc' ? 'PC' : 'Web'
}

function imageSlotOffset(index: number, total: number) {
  return index - (total - 1) / 2
}

function imageCardRotation(attachment: ClipboardSyncAttachment, index: number) {
  const key = `${attachment.attachmentId}-${attachment.fileName}-${index}`
  const hash = Array.from(key).reduce((value, char) => value + char.charCodeAt(0), 0)
  return ((hash % 900) / 100 - 4.5) * 0.55
}

function imageCardStyle(attachment: ClipboardSyncAttachment, index: number, total: number) {
  const offset = imageSlotOffset(index, total)

  // 固定视觉槽位并保持同一基线，卡片角度按附件信息生成稳定的伪随机值。
  return {
    '--card-x': `calc(var(--deck-step) * ${offset})`,
    '--card-rotate': `${imageCardRotation(attachment, index).toFixed(2)}deg`,
    zIndex: 30 + index,
  }
}

function shiftImages(direction: -1 | 1) {
  const maxStart = Math.max(0, imageAttachments.value.length - maxVisibleImages)
  imageWindowStart.value = Math.min(maxStart, Math.max(0, imageWindowStart.value + direction))
}

function downloadAttachment(attachment: ClipboardSyncAttachment) {
  const link = document.createElement('a')
  link.href = fileUrl(props.message, attachment)
  link.download = attachment.fileName
  link.rel = 'noopener'
  document.body.append(link)
  link.click()
  link.remove()
}

function downloadImages(attachments: ClipboardSyncAttachment[]) {
  attachments.forEach((attachment, index) => {
    window.setTimeout(() => downloadAttachment(attachment), index * 120)
  })
}
</script>

<template>
  <article class="rounded-[1.75rem] border border-border/70 bg-background/70 p-5">
    <div class="flex flex-col gap-4 lg:flex-row lg:items-start lg:justify-between">
      <div class="min-w-0 flex-1 space-y-4">
        <div class="flex items-center justify-between">
          <div class="flex flex-wrap items-end gap-2">
            <Badge class="rounded-full">{{ sourceBadge(message.source) }}</Badge>
            <h3 class="text-base font-semibold tracking-[-0.02em]">
              {{ formatTime(message.createdAtMs) }}
            </h3>
            <span class="text-sm text-muted-foreground">
              {{ sourceName(message.source) }}
            </span>
          </div>
          <div>
            <Button variant="outline" class="rounded-full" size="sm" @click="emits('delete', message)">
              <Trash2 class="size-4" />
            </Button>
          </div>
        </div>

        <div class="flex items-center gap-2" v-if="message.text">
          <div class="flex-1">
            <p class="whitespace-pre-wrap wrap-break-word rounded-2xl bg-muted/50 p-4 text-sm leading-6">
              {{ message.text }}
            </p>
          </div>
          <div>
            <Button variant="outline" class="rounded-full" size="sm" @click="emits('copy-text', message)">
              <ClipboardCopy class="size-4" />
            </Button>
          </div>
        </div>
        <div class="flex items-start gap-2" v-if="imageAttachments.length">
          <div class="image-deck flex-1">
            <Button
              v-if="imageAttachments.length > maxVisibleImages"
              variant="outline"
              size="icon-sm"
              class="image-deck-nav left-2"
              :disabled="!canShiftImagesPrev"
              aria-label="上一组图片"
              title="上一组图片"
              @click="shiftImages(-1)"
            >
              <ChevronLeft class="size-4" />
            </Button>

            <article
              v-for="(attachment, index) in visibleImageAttachments"
              :key="attachment.attachmentId"
              class="image-deck-card"
              :style="imageCardStyle(attachment, index, visibleImageAttachments.length)"
            >
              <a
                class="block overflow-hidden rounded-xl bg-muted/70"
                :href="fileUrl(message, attachment)"
                :download="attachment.fileName"
                :title="attachment.fileName"
              >
                <img
                  class="aspect-[4/5] w-full object-cover"
                  :src="fileUrl(message, attachment)"
                  :alt="attachment.fileName"
                >
              </a>
              <div class="grid grid-cols-2 gap-1.5">
                <Button
                  variant="outline"
                  size="sm"
                  class="h-8 rounded-full bg-background/90 px-1.5 text-[11px]"
                  :aria-label="`复制 ${attachment.fileName}`"
                  :title="`复制 ${attachment.fileName}`"
                  @click="emits('copy-images', message, [attachment])"
                >
                  <ClipboardCopy class="size-3.5" />
                </Button>
                <Button
                  as="a"
                  variant="outline"
                  size="sm"
                  class="h-8 rounded-full bg-background/90 px-1.5 text-[11px]"
                  :href="fileUrl(message, attachment)"
                  :download="attachment.fileName"
                  :aria-label="`下载 ${attachment.fileName}`"
                  :title="`下载 ${attachment.fileName}`"
                >
                  <Download class="size-3.5" />
                </Button>
              </div>
            </article>

            <Button
              v-if="imageAttachments.length > maxVisibleImages"
              variant="outline"
              size="icon-sm"
              class="image-deck-nav right-2"
              :disabled="!canShiftImagesNext"
              aria-label="下一组图片"
              title="下一组图片"
              @click="shiftImages(1)"
            >
              <ChevronRight class="size-4" />
            </Button>
          </div>
          <div class="flex flex-col items-center gap-2">
            <Button
                variant="outline"
                size="sm"
                class="h-8 rounded-full bg-background/90 px-1.5 text-[11px]"
                :aria-label="`下载全部图片`"
                title="下载全部图片"
                @click="downloadImages(imageAttachments)"
            >
              <Download class="size-3.5" />
            </Button>
            <Button
              variant="outline"
              class="rounded-full"
              size="sm"
              aria-label="复制全部图片"
              title="复制全部图片"
              @click="emits('copy-images', message, imageAttachments)"
            >
              <ClipboardCopy class="size-4" />
            </Button>
          </div>
        </div>
        <div v-if="fileAttachments.length" class="grid gap-3 sm:grid-cols-2 xl:grid-cols-3">
          <a
            v-for="attachment in fileAttachments"
            :key="attachment.attachmentId"
            class="group grid min-w-0 gap-2 rounded-2xl border border-border/70 bg-card/70 p-3 text-foreground no-underline transition hover:border-primary/50"
            :href="fileUrl(message, attachment)"
            :download="attachment.fileName"
          >
            <div class="flex aspect-[1.45] w-full items-center justify-center rounded-xl bg-muted/70 text-muted-foreground">
              <FileText class="size-8" />
            </div>
            <div class="flex min-w-0 items-center gap-2">
              <FileText class="size-4 shrink-0 text-primary" />
              <span class="truncate text-sm font-medium">{{ attachment.fileName }}</span>
            </div>
            <span class="flex items-center gap-1 text-xs text-muted-foreground">
              <Download class="size-3.5" />
              {{ formatFileSize(attachment.sizeBytes) }}
            </span>
          </a>
        </div>
      </div>
    </div>
  </article>
</template>

<style scoped>
.image-deck {
  --deck-step: clamp(3.4rem, 9vw, 5rem);
  --deck-card-shadow: 68 48 31;

  position: relative;
  isolation: isolate;
  min-height: clamp(16.5rem, 28vw, 21rem);
  overflow: hidden;
  border-radius: 1.5rem;
}

.image-deck:hover {
  --deck-step: clamp(4.6rem, 12vw, 6.8rem);
}

.image-deck-card {
  position: absolute;
  bottom: clamp(0.75rem, 1.8vw, 1.25rem);
  left: 50%;
  display: grid;
  width: clamp(8.75rem, 17vw, 12.25rem);
  gap: 0.5rem;
  padding: 0.625rem;
  border: 1px solid rgb(255 251 245 / 86%);
  border-radius: 1rem;
  background: linear-gradient(180deg, #fffaf3 0%, #f7f0e7 62%, #f2eadf 100%);
  box-shadow:
    0 24px 54px rgb(var(--deck-card-shadow) / 20%),
    0 8px 20px rgb(var(--deck-card-shadow) / 12%),
    inset 0 1px 0 rgb(255 255 255 / 82%);
  transform: translateX(calc(-50% + var(--card-x))) rotate(var(--card-rotate)) scale(1);
  transform-origin: center bottom;
  transition:
    transform 180ms ease,
    box-shadow 180ms ease;
}

.image-deck-card:hover {
  z-index: 80 !important;
  box-shadow:
    0 32px 72px rgb(var(--deck-card-shadow) / 26%),
    0 12px 28px rgb(var(--deck-card-shadow) / 16%),
    inset 0 1px 0 rgb(255 255 255 / 88%);
  transform: translateX(calc(-50% + var(--card-x))) translateY(-0.75rem) rotate(0deg) scale(1.06);
}

.image-deck-nav {
  position: absolute;
  top: 50%;
  z-index: 90;
  border-radius: 9999px;
  background: hsl(var(--background) / 92%);
  box-shadow: 0 12px 28px rgb(0 0 0 / 16%);
  transform: translateY(-50%);
}

@media (max-width: 640px) {
  .image-deck {
    --deck-step: clamp(2.35rem, 14vw, 3.5rem);
  }

  .image-deck:hover {
    --deck-step: clamp(2.8rem, 17vw, 4.25rem);
  }

  .image-deck-card {
    width: clamp(8rem, 36vw, 9rem);
  }
}
</style>
