import { invoke } from '@tauri-apps/api/core'

interface EnterpriseConfig {
  enabled: boolean
  iamBaseUrl: string
  policyBaseUrl: string
  keycloakBaseUrl: string
  keycloakRealm: string
  keycloakClientId: string
  keycloakRedirectUri: string
  appCode: string
}

export type EnterpriseConfigPatch = Partial<EnterpriseConfig>

interface EnterpriseSessionView {
  authenticated: boolean
  isSuperAdmin?: boolean
  permissions?: string[]
  roles?: string[]
  userId?: string
  username?: string
  tenantId?: string
  tenantCode?: string
  accessTokenExpiresAt?: string
}

interface EnterprisePolicyStatus {
  version: string
  mode: string
  expiresAt: string
  syncedAt: string
}

interface EnterpriseAllowedResource {
  type: 'domain' | 'domain_suffix' | 'ip_cidr'
  value: string
}

export interface EnterpriseStateView {
  config: EnterpriseConfig
  session: EnterpriseSessionView
  policyStatus?: EnterprisePolicyStatus
  allowedResources?: EnterpriseAllowedResource[]
}

export interface EnterpriseLoginStart {
  authUrl: string
  state: string
  redirectUri: string
}

export interface EnterpriseAuthCodeRequest {
  code: string
  state: string
}

export async function getEnterpriseState() {
  return invoke<EnterpriseStateView>('get_enterprise_state')
}

export async function patchEnterpriseConfig(patch: EnterpriseConfigPatch) {
  return invoke<EnterpriseStateView>('patch_enterprise_config', { patch })
}

export async function clearEnterpriseSession() {
  return invoke<EnterpriseStateView>('clear_enterprise_session')
}

export async function startEnterpriseLogin(openBrowser = true) {
  return invoke<EnterpriseLoginStart>('start_enterprise_login', {
    openBrowser,
  })
}

export async function completeEnterpriseLogin(
  request: EnterpriseAuthCodeRequest,
) {
  return invoke<EnterpriseStateView>('complete_enterprise_login', { request })
}

export async function syncEnterprisePolicy() {
  return invoke<EnterpriseStateView>('sync_enterprise_policy')
}
