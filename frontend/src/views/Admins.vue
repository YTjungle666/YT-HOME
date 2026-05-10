<template>
  <v-row justify="center">
    <v-col cols="12" md="8" lg="6" xl="5">
      <v-card :loading="loading" class="elevation-3">
        <v-card-title>{{ $t('admin.accountSecurity') }}</v-card-title>
        <v-card-text>
          <v-form @submit.prevent="saveAccount">
            <v-row>
              <v-col cols="12">
                <v-text-field
                  v-model.trim="form.username"
                  :label="$t('login.username')"
                  prepend-inner-icon="mdi-account"
                  autocomplete="username"
                  :rules="[requiredRule]"
                  hide-details="auto"
                />
              </v-col>
              <v-col cols="12">
                <v-text-field
                  v-model="form.currentPassword"
                  :label="$t('admin.oldPass')"
                  prepend-inner-icon="mdi-lock-check"
                  type="password"
                  autocomplete="current-password"
                  :rules="[requiredRule]"
                  hide-details="auto"
                />
              </v-col>
              <v-col cols="12" sm="6">
                <v-text-field
                  v-model="form.newPassword"
                  :label="$t('admin.newPass')"
                  prepend-inner-icon="mdi-lock-plus"
                  type="password"
                  autocomplete="new-password"
                  :rules="[passwordLengthRule]"
                  hide-details="auto"
                />
              </v-col>
              <v-col cols="12" sm="6">
                <v-text-field
                  v-model="form.confirmPassword"
                  :label="$t('admin.confirmPass')"
                  prepend-inner-icon="mdi-lock-alert"
                  type="password"
                  autocomplete="new-password"
                  :rules="[confirmPasswordRule]"
                  hide-details="auto"
                />
              </v-col>
            </v-row>
            <v-card-actions class="px-0">
              <v-btn
                color="primary"
                type="submit"
                prepend-icon="mdi-content-save"
                :loading="loading"
                :disabled="!canSave"
              >
                {{ $t('actions.save') }}
              </v-btn>
              <v-spacer />
              <v-btn
                variant="outlined"
                color="warning"
                prepend-icon="mdi-logout"
                @click="logout"
              >
                {{ $t('menu.logout') }}
              </v-btn>
            </v-card-actions>
          </v-form>
        </v-card-text>
      </v-card>
    </v-col>
  </v-row>
</template>

<script lang="ts" setup>
import { computed, inject, onMounted, reactive, ref, Ref } from 'vue'
import { useRouter } from 'vue-router'
import { i18n } from '@/locales'
import HttpUtils, { logout } from '@/plugins/httputil'
import { push } from 'notivue'

const router = useRouter()
const loading: Ref = inject('loading') ?? ref(false)

const originalUsername = ref('')
const form = reactive({
  username: '',
  currentPassword: '',
  newPassword: '',
  confirmPassword: '',
})

const requiredRule = (value: string) => !!value || i18n.global.t('login.unRules')
const passwordLengthRule = (value: string) => {
  return !value || value.length >= 8 || i18n.global.t('admin.passwordLength')
}
const confirmPasswordRule = (value: string) => {
  return value === form.newPassword || i18n.global.t('admin.passwordMismatch')
}

const canSave = computed(() => {
  const usernameChanged = form.username.trim() !== originalUsername.value
  const passwordChanged = form.newPassword.length > 0
  return form.username.trim().length > 0
    && form.currentPassword.length > 0
    && (!passwordChanged || (form.newPassword.length >= 8 && form.confirmPassword === form.newPassword))
    && (usernameChanged || passwordChanged)
})

onMounted(async () => {
  loading.value = true
  const msg = await HttpUtils.get('api/account')
  if (msg.success) {
    originalUsername.value = msg.obj.username ?? ''
    form.username = originalUsername.value
  }
  loading.value = false
})

const saveAccount = async () => {
  if (!canSave.value) return

  loading.value = true
  const msg = await HttpUtils.post('api/account', {
    username: form.username.trim(),
    currentPassword: form.currentPassword,
    newPassword: form.newPassword,
  })
  loading.value = false

  if (msg.success) {
    push.success({
      title: i18n.global.t('success'),
      duration: 5000,
      message: i18n.global.t('admin.loginAgain'),
    })
    await router.replace('/login')
  }
}
</script>
