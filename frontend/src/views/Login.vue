<template>
  <main class="login-page">
    <v-card class="login-card" rounded="lg" elevation="0">
      <header class="login-header">
        <h1>YT-HOME</h1>
        <h2>{{ $t('login.title') }}</h2>
      </header>

      <v-form class="login-form" @submit.prevent="login">
        <v-text-field
          v-model="username"
          :label="$t('login.username')"
          :rules="usernameRules"
          variant="outlined"
          density="comfortable"
          autocomplete="username"
          required
        />
        <v-text-field
          v-model="password"
          :label="$t('login.password')"
          :rules="passwordRules"
          type="password"
          variant="outlined"
          density="comfortable"
          autocomplete="current-password"
          required
        />
        <v-btn
          :loading="loading"
          type="submit"
          color="primary"
          block
          size="large"
          class="login-button"
          v-text="$t('actions.submit')"
        />
      </v-form>
    </v-card>
  </main>
</template>

<script lang="ts" setup>
import { ref } from "vue"
import { i18n } from '@/locales'
import { useRouter } from 'vue-router'
import HttpUtil from '@/plugins/httputil'

const username = ref('')
const usernameRules = [
  (value: string) => {
    if (value?.length > 0) return true
    return i18n.global.t('login.unRules')
  },
]

const password = ref('')
const passwordRules = [
  (value: string) => {
    if (value?.length > 0) return true
    return i18n.global.t('login.pwRules')
  },
]

const loading = ref(false)
const router = useRouter()

const login = async () => {
  if (loading.value || username.value == '' || password.value == '') return

  loading.value = true
  try {
    const response = await HttpUtil.post('api/login', { user: username.value, pass: password.value })
    if (response.success) {
      await router.replace('/')
    }
  } finally {
    loading.value = false
  }
}
</script>

<style scoped>
.login-page {
  min-height: 100vh;
  display: grid;
  place-items: center;
  padding: 24px;
  background: rgb(var(--v-theme-background));
}

.login-card {
  width: 100%;
  max-width: 400px;
  padding: 32px;
  color: rgb(var(--v-theme-on-surface));
  background: rgb(var(--v-theme-surface));
  border: 1px solid rgb(var(--v-theme-on-surface) / 0.12);
  box-shadow: 0 18px 36px rgb(0 0 0 / 0.1);
}

.login-header {
  margin-bottom: 24px;
  text-align: center;
}

.login-header h1,
.login-header h2 {
  margin: 0;
  letter-spacing: 0;
}

.login-header h1 {
  color: rgb(var(--v-theme-primary));
  font-size: 2rem;
  font-weight: 800;
  line-height: 1.1;
}

.login-header h2 {
  margin-top: 8px;
  font-size: 1.05rem;
  font-weight: 600;
  line-height: 1.35;
}

.login-form {
  display: grid;
  gap: 2px;
}

.login-button {
  margin-top: 8px;
  min-height: 46px;
}

@media (max-width: 760px) {
  .login-page {
    padding: 16px;
  }

  .login-card {
    padding: 26px 20px;
  }
}
</style>
