import { computed, onMounted, onUnmounted, ref, watch } from "vue";

type ThemeMode = "light" | "dark" | "system";
type ResolvedThemeMode = "light" | "dark";

const STORAGE_KEY = "cobweb-web-theme-mode";
const mode = ref<ThemeMode>("system");
const systemMode = ref<ResolvedThemeMode>("light");

function readStoredMode(): ThemeMode {
  const stored = window.localStorage.getItem(STORAGE_KEY);
  return stored === "light" || stored === "dark" || stored === "system" ? stored : "system";
}

function applyResolvedMode(nextMode: ResolvedThemeMode) {
  document.documentElement.dataset.theme = nextMode;
  document.documentElement.classList.toggle("dark", nextMode === "dark");
  document.documentElement.style.colorScheme = nextMode;
}

export function useTheme() {
  const resolvedMode = computed<ResolvedThemeMode>(() => {
    return mode.value === "system" ? systemMode.value : mode.value;
  });

  function toggleThemeMode() {
    mode.value = resolvedMode.value === "dark" ? "light" : "dark";
  }

  function syncSystemMode(event?: MediaQueryListEvent) {
    systemMode.value = event?.matches ?? window.matchMedia("(prefers-color-scheme: dark)").matches ? "dark" : "light";
  }

  onMounted(() => {
    mode.value = readStoredMode();
    const mediaQuery = window.matchMedia("(prefers-color-scheme: dark)");
    syncSystemMode();
    mediaQuery.addEventListener("change", syncSystemMode);

    onUnmounted(() => {
      mediaQuery.removeEventListener("change", syncSystemMode);
    });
  });

  watch(
    [mode, resolvedMode],
    () => {
      window.localStorage.setItem(STORAGE_KEY, mode.value);
      applyResolvedMode(resolvedMode.value);
    },
    { immediate: true },
  );

  return {
    mode,
    resolvedMode,
    toggleThemeMode,
  };
}
