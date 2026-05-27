import {
  ScienceRounded,
  LoginRounded,
  LogoutRounded,
  SyncRounded,
} from '@mui/icons-material'
import { Box, Button, Chip, Stack, TextField } from '@mui/material'
import { useLockFn } from 'ahooks'
import { useState } from 'react'

import { Switch } from '@/components/base'
import {
  SettingItem,
  SettingList,
} from '@/components/setting/mods/setting-comp'
import { useEnterprise } from '@/hooks/use-enterprise'
import {
  type EnterpriseConfigPatch,
  syncEnterprisePolicy,
} from '@/services/enterprise'
import { hasEnterpriseAdvancedAccess } from '@/services/enterprise-access'
import { showNotice } from '@/services/notice-service'

interface Props {
  onError: (err: Error) => void
}

interface EnterpriseTextFieldProps {
  value: string
  disabled: boolean
  width: number
  onCommit: (value: string) => void | Promise<void>
}

const EnterpriseTextField = ({
  value,
  disabled,
  width,
  onCommit,
}: EnterpriseTextFieldProps) => (
  <TextField
    key={value}
    size="small"
    defaultValue={value}
    disabled={disabled}
    onBlur={(event) => {
      const next = event.target.value.trim()
      if (next !== value) {
        void onCommit(next)
      }
    }}
    sx={{ width }}
  />
)

const LOCAL_MOCK_CONFIG: EnterpriseConfigPatch = {
  enabled: true,
  iamBaseUrl: 'http://127.0.0.1:18080',
  policyBaseUrl: 'http://127.0.0.1:18080',
  keycloakBaseUrl: 'http://127.0.0.1:18080',
  keycloakRealm: 'staff',
  keycloakClientId: 'company-proxy-desktop',
  keycloakRedirectUri: 'http://127.0.0.1:33221/auth/callback',
  appCode: 'company-proxy-desktop',
}

const SettingEnterprise = ({ onError }: Props) => {
  const {
    enterprise,
    patchEnterprise,
    clearSession,
    startLogin,
    completeLogin,
    refetchEnterprise,
  } = useEnterprise()
  const [authCode, setAuthCode] = useState('')
  const [loginState, setLoginState] = useState('')

  const config = enterprise?.config
  const session = enterprise?.session
  const policy = enterprise?.policyStatus

  const enabled = config?.enabled ?? false
  const authenticated = session?.authenticated ?? false
  const showLocalMock = import.meta.env.DEV
  const showAdvancedControls =
    import.meta.env.DEV || hasEnterpriseAdvancedAccess(enterprise)

  const onToggle = useLockFn(async (next: boolean) => {
    try {
      await patchEnterprise({ enabled: next })
    } catch (err: any) {
      onError(err)
      throw err
    }
  })

  const onUseLocalMock = useLockFn(async () => {
    try {
      await patchEnterprise(LOCAL_MOCK_CONFIG)
      showNotice.success('已应用本地 Mock 企业配置')
    } catch (err: any) {
      onError(err)
      throw err
    }
  })

  const onPatchText = useLockFn(
    async (key: keyof EnterpriseConfigPatch, value: string) => {
      try {
        await patchEnterprise({ [key]: value })
      } catch (err: any) {
        onError(err)
        throw err
      }
    },
  )

  const onStartLogin = useLockFn(async () => {
    try {
      const started = await startLogin(true)
      setLoginState(started.state)
      const loggedIn = await pollEnterpriseLogin(refetchEnterprise)
      if (loggedIn) {
        await syncAndApplyPolicy(refetchEnterprise)
      }
    } catch (err: any) {
      onError(err)
      throw err
    }
  })

  const onCompleteLogin = useLockFn(async () => {
    try {
      const { code, state } = parseAuthInput(authCode, loginState)
      if (!code) return

      await completeLogin({
        code,
        state,
      })
      setAuthCode('')
      setLoginState('')
      await syncAndApplyPolicy(refetchEnterprise)
      showNotice.success('企业登录已完成')
    } catch (err: any) {
      onError(err)
      throw err
    }
  })

  const onSyncPolicy = useLockFn(async () => {
    try {
      await syncAndApplyPolicy(refetchEnterprise)
      showNotice.success('连接配置已同步')
    } catch (err: any) {
      onError(err)
      throw err
    }
  })

  const onLogout = useLockFn(async () => {
    try {
      await clearSession()
    } catch (err: any) {
      onError(err)
      throw err
    }
  })

  return (
    <SettingList title="企业代理">
      <SettingItem
        label="登录状态"
        extra={
          <Stack direction="row" spacing={1} sx={{ alignItems: 'center' }}>
            {showLocalMock && (
              <Button
                size="small"
                variant="outlined"
                startIcon={<ScienceRounded />}
                onClick={onUseLocalMock}
              >
                本地 Mock
              </Button>
            )}
            <Chip
              size="small"
              color={
                authenticated ? 'success' : enabled ? 'warning' : 'default'
              }
              label={authenticated ? '已登录' : enabled ? '未登录' : '未启用'}
              sx={{ ml: 1 }}
            />
          </Stack>
        }
      >
        {showAdvancedControls && (
          <Switch
            edge="end"
            checked={enabled}
            onChange={(_, checked) => onToggle(checked)}
          />
        )}
      </SettingItem>

      {showAdvancedControls && (
        <>
          <SettingItem label="IAM 地址">
            <EnterpriseTextField
              value={config?.iamBaseUrl ?? ''}
              disabled={!enabled}
              width={260}
              onCommit={(value) => onPatchText('iamBaseUrl', value)}
            />
          </SettingItem>

          <SettingItem label="策略地址">
            <EnterpriseTextField
              value={config?.policyBaseUrl ?? ''}
              disabled={!enabled}
              width={260}
              onCommit={(value) => onPatchText('policyBaseUrl', value)}
            />
          </SettingItem>

          <SettingItem label="Keycloak 地址">
            <EnterpriseTextField
              value={config?.keycloakBaseUrl ?? ''}
              disabled={!enabled}
              width={260}
              onCommit={(value) => onPatchText('keycloakBaseUrl', value)}
            />
          </SettingItem>

          <SettingItem label="Realm / Client">
            <Stack direction="row" spacing={1}>
              <EnterpriseTextField
                value={config?.keycloakRealm ?? ''}
                disabled={!enabled}
                width={110}
                onCommit={(value) => onPatchText('keycloakRealm', value)}
              />
              <EnterpriseTextField
                value={config?.keycloakClientId ?? ''}
                disabled={!enabled}
                width={145}
                onCommit={(value) => onPatchText('keycloakClientId', value)}
              />
            </Stack>
          </SettingItem>

          <SettingItem label="授权回调">
            <EnterpriseTextField
              value={config?.keycloakRedirectUri ?? ''}
              disabled={!enabled}
              width={260}
              onCommit={(value) => onPatchText('keycloakRedirectUri', value)}
            />
          </SettingItem>

          <SettingItem label="应用编码">
            <EnterpriseTextField
              value={config?.appCode ?? ''}
              disabled={!enabled}
              width={260}
              onCommit={(value) => onPatchText('appCode', value)}
            />
          </SettingItem>
        </>
      )}

      <SettingItem
        label={authenticated ? '企业账号' : '企业登录'}
        secondary={
          authenticated ? (session?.username ?? '已完成登录') : undefined
        }
      >
        <Stack direction="row" spacing={1} sx={{ alignItems: 'center' }}>
          {!authenticated && (
            <Button
              size="small"
              variant="contained"
              startIcon={<LoginRounded />}
              disabled={!enabled}
              onClick={onStartLogin}
            >
              登录
            </Button>
          )}
          {import.meta.env.DEV && (
            <>
              <TextField
                size="small"
                placeholder="code / URL"
                value={authCode}
                disabled={!enabled}
                onChange={(event) => setAuthCode(event.target.value)}
                sx={{ width: 120 }}
              />
              <Button
                size="small"
                variant="outlined"
                disabled={!enabled || !authCode.trim()}
                onClick={onCompleteLogin}
              >
                完成
              </Button>
            </>
          )}
        </Stack>
      </SettingItem>

      <SettingItem
        label="连接配置"
        secondary={policy ? `已同步 ${policy.syncedAt}` : '待同步'}
      >
        <Box sx={{ display: 'flex', gap: 1 }}>
          <Button
            size="small"
            variant="outlined"
            startIcon={<SyncRounded />}
            disabled={!authenticated}
            onClick={onSyncPolicy}
          >
            同步配置
          </Button>
          <Button
            size="small"
            color="error"
            variant="outlined"
            startIcon={<LogoutRounded />}
            disabled={!authenticated}
            onClick={onLogout}
          >
            退出
          </Button>
        </Box>
      </SettingItem>
    </SettingList>
  )
}

async function pollEnterpriseLogin(
  refetchEnterprise: ReturnType<typeof useEnterprise>['refetchEnterprise'],
) {
  for (let i = 0; i < 30; i += 1) {
    const result = await refetchEnterprise()
    if (result.data?.session.authenticated) {
      return true
    }
    await sleep(1000)
  }
  return false
}

async function syncAndApplyPolicy(
  refetchEnterprise: ReturnType<typeof useEnterprise>['refetchEnterprise'],
) {
  await syncEnterprisePolicy()
  await refetchEnterprise()
}

function sleep(ms: number) {
  return new Promise((resolve) => window.setTimeout(resolve, ms))
}

function parseAuthInput(input: string, fallbackState: string) {
  const value = input.trim()
  if (!value) {
    return { code: '', state: fallbackState }
  }

  try {
    const url = new URL(value)
    return {
      code: url.searchParams.get('code') ?? value,
      state: url.searchParams.get('state') ?? fallbackState,
    }
  } catch {
    return { code: value, state: fallbackState }
  }
}

export default SettingEnterprise
