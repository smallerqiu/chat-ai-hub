<script setup lang="ts">
import { computed, nextTick, onMounted, ref, watch } from "vue";
import { invoke } from "@tauri-apps/api/core";

type AiTarget = {
  id: string;
  name: string;
  url: string;
  iconUrl: string;
};

const STORAGE_KEY = "chat-ai-hub.targets";
const EXPANDED_WIDTH = 176;
const COLLAPSED_WIDTH = 64;

const defaultTargets = [
  makeTarget("ChatGPT", "https://chatgpt.com/"),
  makeTarget("豆包", "https://www.doubao.com/chat/"),
  makeTarget("DeepSeek", "https://chat.deepseek.com/"),
];

const targets = ref<AiTarget[]>(loadTargets());
const activeId = ref(targets.value[0]?.id ?? "");
const isSettingsOpen = ref(false);
const isCollapsed = ref(false);

const sidebarWidth = computed(() => (isCollapsed.value ? COLLAPSED_WIDTH : EXPANDED_WIDTH));
const activeTarget = computed(() => targets.value.find((target) => target.id === activeId.value) ?? targets.value[0]);

watch(
  targets,
  () => {
    localStorage.setItem(STORAGE_KEY, JSON.stringify(targets.value));
    if (!targets.value.some((target) => target.id === activeId.value)) {
      activeId.value = targets.value[0]?.id ?? "";
    }
  },
  { deep: true },
);

watch(activeTarget, async (target) => {
  if (!target) {
    await invoke("hide_all_webviews");
    return;
  }

  await invoke("select_target", {
    id: target.id,
    url: target.url,
    sidebarWidth: sidebarWidth.value,
  });
});

watch(sidebarWidth, async () => {
  await syncWebviewLayout();
});

onMounted(async () => {
  await nextTick();
  if (activeTarget.value) {
    await invoke("select_target", {
      id: activeTarget.value.id,
      url: activeTarget.value.url,
      sidebarWidth: sidebarWidth.value,
    });
  }
});

function makeTarget(name: string, url: string): AiTarget {
  const normalizedUrl = normalizeUrl(url);
  return {
    id: crypto.randomUUID(),
    name,
    url: normalizedUrl,
    iconUrl: faviconFor(normalizedUrl),
  };
}

function normalizeUrl(value: string) {
  const trimmed = value.trim();
  if (!trimmed || trimmed === "https://") return "https://";
  return /^https?:\/\//i.test(trimmed) ? trimmed : `https://${trimmed}`;
}

function faviconFor(value: string) {
  try {
    const url = new URL(normalizeUrl(value));
    return `${url.origin}/favicon.ico`;
  } catch {
    return "";
  }
}

function loadTargets() {
  const raw = localStorage.getItem(STORAGE_KEY);
  if (!raw) return defaultTargets;

  try {
    const parsed = JSON.parse(raw) as AiTarget[];
    return parsed.length ? parsed : defaultTargets;
  } catch {
    return defaultTargets;
  }
}

function addTarget() {
  const next = makeTarget("新网站", "https://");
  targets.value.push(next);
  activeId.value = next.id;
  isSettingsOpen.value = true;
}

async function deleteTarget(id: string) {
  targets.value = targets.value.filter((target) => target.id !== id);
  await invoke("remove_target", { id });
}

function updateTarget(id: string, patch: Partial<AiTarget>) {
  const target = targets.value.find((item) => item.id === id);
  if (!target) return;

  if (patch.name !== undefined) target.name = patch.name;
  if (patch.url !== undefined) {
    target.url = normalizeUrl(patch.url);
    target.iconUrl = faviconFor(target.url);
  }
}

async function selectTarget(id: string) {
  activeId.value = id;
}

async function syncWebviewLayout() {
  await invoke("sync_webview_layout", { sidebarWidth: sidebarWidth.value });
}
</script>

<template>
  <main class="app-shell" :style="{ '--sidebar-width': `${sidebarWidth}px` }">
    <aside class="sidebar" :class="{ collapsed: isCollapsed }" data-tauri-drag-region>
      <button class="brand" title="Chat AI Hub" type="button" @click="isCollapsed = !isCollapsed">
        <span>AI</span>
      </button>

      <nav class="target-list" aria-label="已添加的网站">
        <button
          v-for="target in targets"
          :key="target.id"
          class="target-button"
          :class="{ active: target.id === activeTarget?.id }"
          :title="target.name"
          type="button"
          @click="selectTarget(target.id)"
        >
          <img :src="target.iconUrl" alt="" @error="($event.currentTarget as HTMLImageElement).style.display = 'none'" />
          <span class="fallback">{{ target.name.slice(0, 2).toUpperCase() }}</span>
          <span class="label">{{ target.name }}</span>
        </button>
      </nav>

      <div class="sidebar-actions">
        <button class="icon-button" title="添加网站" type="button" @click="addTarget">+</button>
        <button class="icon-button" title="设置" type="button" @click="isSettingsOpen = true">⚙</button>
      </div>
    </aside>

    <section class="webview-stage">
      <div v-if="!activeTarget" class="empty-state">
        <button class="primary-button" type="button" @click="addTarget">添加第一个网站</button>
      </div>
    </section>

    <div v-if="isSettingsOpen" class="modal-backdrop">
      <section class="settings-panel" role="dialog" aria-modal="true" aria-label="设置">
        <header>
          <h1>设置</h1>
          <button class="panel-close" title="关闭" type="button" @click="isSettingsOpen = false">×</button>
        </header>

        <div class="settings-list">
          <article v-for="target in targets" :key="target.id" class="settings-row">
            <img :src="target.iconUrl" alt="" />
            <label>
              <span>名字</span>
              <input :value="target.name" placeholder="ChatGPT" @input="updateTarget(target.id, { name: ($event.target as HTMLInputElement).value })" />
            </label>
            <label>
              <span>URL</span>
              <input :value="target.url" placeholder="https://chatgpt.com/" @change="updateTarget(target.id, { url: ($event.target as HTMLInputElement).value })" />
            </label>
            <button class="delete-button" title="删除" type="button" @click="deleteTarget(target.id)">×</button>
          </article>
        </div>

        <footer>
          <button class="primary-button" type="button" @click="addTarget">添加网站</button>
        </footer>
      </section>
    </div>
  </main>
</template>
