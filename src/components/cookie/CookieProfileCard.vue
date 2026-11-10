<script setup lang="ts">
import { useCookieStore } from "@/stores/cookie";
import { formatDateTime } from "@/utils/format";
import { useI18n } from "vue-i18n";
import { NCard } from "naive-ui";
import type { CookieMatchResult, CookieProfileSummary } from "@/types";

const { t } = useI18n();
const cookieStore = useCookieStore();

interface Props {
  /** 嵌入弹窗时去掉外层卡片（标题由弹窗提供） */
  plain?: boolean;
}

const props = withDefaults(defineProps<Props>(), { plain: false });

const showModal = ref(false);
const editingProfile = ref<CookieProfileSummary | null>(null);
const testerUrl = ref("");
const testerResult = ref<CookieMatchResult | null>(null);
const testerDone = ref(false);
const testing = ref(false);

onMounted(async () => {
  try {
    await cookieStore.load();
  } catch (error: unknown) {
    window.$message.error(String(error));
  }
});

const openCreate = (): void => {
  editingProfile.value = null;
  showModal.value = true;
};

const openEdit = (profile: CookieProfileSummary): void => {
  editingProfile.value = profile;
  showModal.value = true;
};

/** 直接切换启用状态（不传正文，沿用库中已有内容） */
const toggleEnabled = async (profile: CookieProfileSummary): Promise<void> => {
  try {
    await cookieStore.saveProfile({
      id: profile.id,
      name: profile.name,
      domains: [...profile.domains],
      enabled: !profile.enabled,
      content: null,
    });
  } catch (error: unknown) {
    window.$message.error(String(error));
  }
};

const handleDelete = async (profile: CookieProfileSummary): Promise<void> => {
  try {
    await cookieStore.deleteProfile(profile.id);
    if (editingProfile.value?.id === profile.id) {
      editingProfile.value = null;
      showModal.value = false;
    }
    window.$message.success(t("cookie.deleted"));
  } catch (error: unknown) {
    window.$message.error(String(error));
  }
};

/** 上移/下移：顺序决定命中同一域名规则时谁优先 */
const move = async (index: number, offset: number): Promise<void> => {
  const ids = cookieStore.profiles.map((profile) => profile.id);
  const target = index + offset;
  if (target < 0 || target >= ids.length) return;
  [ids[index], ids[target]] = [ids[target], ids[index]];
  try {
    await cookieStore.reorderProfiles(ids);
  } catch (error: unknown) {
    window.$message.error(String(error));
  }
};

const handleTest = async (): Promise<void> => {
  const url = testerUrl.value.trim();
  if (!url) return;
  testing.value = true;
  try {
    testerResult.value = await cookieStore.testMatch(url);
    testerDone.value = true;
  } catch (error: unknown) {
    window.$message.error(String(error));
  } finally {
    testing.value = false;
  }
};
</script>

<template>
  <component
    :is="props.plain ? 'div' : NCard"
    v-bind="props.plain ? {} : { title: $t('cookie.settings'), size: 'small' }"
    class="section-card"
  >
    <n-flex vertical :size="12">
      <n-flex align="center" justify="space-between">
        <n-text depth="3" style="font-size: 13px; flex: 1">
          {{ $t("cookie.desc") }}
        </n-text>
        <n-button size="small" type="primary" secondary @click="openCreate">
          <template #icon>
            <n-icon><icon-mdi-plus /></n-icon>
          </template>
          {{ $t("cookie.addProfile") }}
        </n-button>
      </n-flex>

      <n-empty
        v-if="cookieStore.profiles.length === 0"
        :description="$t('cookie.noProfiles')"
        style="padding: 24px 0"
      />

      <n-list v-else bordered>
        <n-list-item v-for="(profile, index) in cookieStore.profiles" :key="profile.id">
          <n-flex vertical :size="6" style="min-width: 0; width: 100%">
            <n-flex align="center" :size="8">
              <n-switch
                :value="profile.enabled"
                size="small"
                @update:value="() => toggleEnabled(profile)"
              />
              <n-text strong>{{ profile.name }}</n-text>
              <n-tag size="tiny" :bordered="false">
                {{ $t("cookie.cookieCount", { count: profile.cookieCount }) }}
              </n-tag>
            </n-flex>
            <n-flex :size="4" :wrap="true" align="center">
              <n-tag
                v-for="domain in profile.domains"
                :key="domain"
                size="tiny"
                :bordered="false"
                type="info"
              >
                {{ domain }}
              </n-tag>
            </n-flex>
            <n-text depth="3" style="font-size: 12px">
              {{ $t("cookie.updatedAt", { time: formatDateTime(profile.updatedAt) }) }}
            </n-text>
          </n-flex>

          <template #suffix>
            <n-flex :size="6" :wrap="false">
              <n-button
                quaternary
                circle
                size="medium"
                :disabled="index === 0"
                :aria-label="$t('cookie.moveUp')"
                @click="move(index, -1)"
              >
                <template #icon>
                  <n-icon size="20"><icon-mdi-chevron-up /></n-icon>
                </template>
              </n-button>
              <n-button
                quaternary
                circle
                size="medium"
                :disabled="index === cookieStore.profiles.length - 1"
                :aria-label="$t('cookie.moveDown')"
                @click="move(index, 1)"
              >
                <template #icon>
                  <n-icon size="20"><icon-mdi-chevron-down /></n-icon>
                </template>
              </n-button>
              <n-button
                quaternary
                circle
                size="medium"
                :aria-label="$t('cookie.editProfile')"
                @click="openEdit(profile)"
              >
                <template #icon>
                  <n-icon size="20"><icon-mdi-pencil-outline /></n-icon>
                </template>
              </n-button>
              <n-popconfirm @positive-click="handleDelete(profile)">
                <template #trigger>
                  <n-button
                    quaternary
                    circle
                    size="medium"
                    type="error"
                    :aria-label="$t('cookie.deleteProfile')"
                  >
                    <template #icon>
                      <n-icon size="20"><icon-mdi-delete-outline /></n-icon>
                    </template>
                  </n-button>
                </template>
                {{ $t("cookie.deleteConfirm") }}
              </n-popconfirm>
            </n-flex>
          </template>
        </n-list-item>
      </n-list>

      <n-divider style="margin: 4px 0" />

      <n-flex vertical :size="8">
        <n-text depth="3" style="font-size: 13px">{{ $t("cookie.tester") }}</n-text>
        <n-flex align="center" :size="8">
          <n-input
            v-model:value="testerUrl"
            size="small"
            :placeholder="$t('cookie.testerPlaceholder')"
            style="flex: 1"
            @keydown.enter="handleTest"
          />
          <n-button size="small" :loading="testing" :disabled="!testerUrl.trim()" @click="handleTest">
            {{ $t("cookie.test") }}
          </n-button>
        </n-flex>
        <n-alert v-if="testerDone" :type="testerResult ? 'success' : 'default'" :bordered="false">
          {{
            testerResult
              ? $t("cookie.resolvedAuto", {
                  name: testerResult.profileName,
                  domain: testerResult.matchedDomain,
                })
              : $t("cookie.noMatch")
          }}
        </n-alert>
      </n-flex>
    </n-flex>
  </component>

  <CookieProfileModal
    v-model:show="showModal"
    :profile="editingProfile"
    @saved="testerDone = false"
  />
</template>
