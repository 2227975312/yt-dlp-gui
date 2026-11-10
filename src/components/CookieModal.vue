<script setup lang="ts">
import { useStatusStore } from "@/stores/status";

const statusStore = useStatusStore();
</script>

<template>
  <n-modal
    v-model:show="statusStore.showCookieModal"
    preset="card"
    :title="$t('cookie.settings')"
    style="width: min(760px, 92vw)"
  >
    <CookieProfileCard plain />
  </n-modal>
</template>
