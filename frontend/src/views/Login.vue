<template>
  <v-container fluid class="login-shell">
    <main class="login-wrap">
      <v-card class="auth-card" rounded="lg" elevation="0">
        <header class="auth-header">
          <h1>YT-HOME</h1>
          <h2>{{ $t('login.title') }}</h2>
        </header>

        <v-form @submit.prevent="login" ref="form">
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
  </v-container>
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
  if (username.value == '' || password.value == '') return
  loading.value=true
  const response = await HttpUtil.post('api/login',{user: username.value, pass: password.value})
  if(response.success){
    setTimeout(() => {
      loading.value=false
      router.push('/')
    }, 500)
  } else {
    loading.value=false
  }
}
</script>

<style scoped>
.login-shell {
  --login-border: rgb(var(--v-theme-on-surface) / 0.12);
  --login-shadow: 0 22px 48px rgb(12 18 28 / 0.12);
  min-height: 100vh;
  padding: 20px;
  background:
    linear-gradient(135deg, rgb(var(--v-theme-background)) 0%, rgb(var(--v-theme-surface)) 100%);
}

:global(.v-theme--dark) .login-shell {
  --login-border: rgb(var(--v-theme-on-surface) / 0.16);
  --login-shadow: 0 24px 54px rgb(0 0 0 / 0.36);
  background:
    linear-gradient(135deg, rgb(10 14 20) 0%, rgb(21 28 38) 100%);
}

.login-wrap {
  display: grid;
  min-height: calc(100vh - 40px);
  place-items: center;
}

.auth-card {
  width: 100%;
  max-width: 420px;
  padding: 34px;
  color: rgb(var(--v-theme-on-surface));
  background: rgb(var(--v-theme-surface));
  border: 1px solid var(--login-border);
  box-shadow: var(--login-shadow);
}

.auth-header {
  margin-bottom: 26px;
  text-align: center;
}

.auth-header h1,
.auth-header h2 {
  margin: 0;
  letter-spacing: 0;
}

.auth-header h1 {
  color: rgb(var(--v-theme-primary));
  font-size: 2.1rem;
  font-weight: 800;
  line-height: 1.1;
}

.auth-header h2 {
  margin-top: 10px;
  font-size: 1.12rem;
  font-weight: 600;
  line-height: 1.35;
}

.login-button {
  margin-top: 8px;
  min-height: 46px;
}

@media (max-width: 760px) {
  .login-shell {
    padding: 14px;
  }

  .login-wrap {
    min-height: calc(100vh - 28px);
  }

  .auth-card {
    padding: 26px 20px;
  }
}
</style>
