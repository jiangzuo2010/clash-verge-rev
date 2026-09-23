import {
  CheckCircleRounded,
  LanguageRounded,
  LoginRounded,
  LogoutRounded,
  SyncRounded,
} from '@mui/icons-material'
import {
  Box,
  Button,
  Chip,
  CircularProgress,
  Stack,
  Typography,
} from '@mui/material'
import { useLockFn } from 'ahooks'
import { useState } from 'react'
import { useTranslation } from 'react-i18next'

import appLogo from '@/assets/image/app_logo.png'
import { EnhancedCard } from '@/components/home/enhanced-card'
import {
  EnterpriseAllowedResources,
  EnterpriseStatusLine,
} from '@/components/home/enterprise-access-details'
import { useEnterprise } from '@/hooks/use-enterprise'
import { useI18n } from '@/hooks/use-i18n'
import { syncEnterprisePolicy } from '@/services/enterprise'
import { showNotice } from '@/services/notice-service'

interface EnterpriseAccessHomeProps {
  fullscreen?: boolean
}

const loginLanguages = [
  { code: 'zh', label: '中文' },
  { code: 'en', label: 'English' },
  { code: 'pl', label: 'Polski' },
]

export const EnterpriseAccessHome = ({
  fullscreen = false,
}: EnterpriseAccessHomeProps) => {
  const { t } = useTranslation()
  const { currentLanguage, switchLanguage } = useI18n()
  const { enterprise, clearSession, startLogin, refetchEnterprise } =
    useEnterprise()
  const session = enterprise?.session
  const policy = enterprise?.policyStatus
  const authenticated = session?.authenticated ?? false
  const allowedResources = enterprise?.allowedResources ?? []
  const [loggingIn, setLoggingIn] = useState(false)
  const [syncing, setSyncing] = useState(false)

  const onStartLogin = useLockFn(async () => {
    setLoggingIn(true)
    try {
      await startLogin(true)
      const loggedIn = await pollEnterpriseLogin(refetchEnterprise)
      if (loggedIn) {
        const synced = await syncPolicyAfterLogin(refetchEnterprise, t)
        if (synced) {
          showNotice.success(t('home.enterprise.notices.connected'))
        }
        return
      }
      showNotice.info(t('home.enterprise.notices.completeInBrowser'))
    } catch (err) {
      const result = await refetchEnterprise()
      if (result.data?.session.authenticated) {
        showNotice.error(t('home.enterprise.notices.syncFailed'))
      } else {
        showNotice.error(t('home.enterprise.notices.loginFailed'))
      }
      console.error('[enterprise-login] failed:', err)
    } finally {
      setLoggingIn(false)
    }
  })

  const onSyncPolicy = useLockFn(async () => {
    setSyncing(true)
    try {
      await syncEnterprisePolicy()
      await refetchEnterprise()
      showNotice.success(t('home.enterprise.notices.synced'))
    } catch (err) {
      await refetchEnterprise()
      showNotice.error(t('home.enterprise.notices.syncFailed'))
      console.error('[enterprise-sync] failed:', err)
    } finally {
      setSyncing(false)
    }
  })

  const onLogout = useLockFn(async () => {
    try {
      await clearSession()
      await refetchEnterprise()
      showNotice.success(t('home.enterprise.notices.signedOut'))
    } catch (err) {
      showNotice.error(t('home.enterprise.notices.signOutFailed'))
      console.error('[enterprise-logout] failed:', err)
    }
  })

  if (!authenticated) {
    return (
      <Box
        sx={{
          minHeight: fullscreen ? '100vh' : 'calc(100vh - 96px)',
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'center',
          px: { xs: 2, sm: 4 },
          py: { xs: 4, sm: 6 },
          background: (theme) =>
            theme.palette.mode === 'light'
              ? 'linear-gradient(135deg, #fff7f5 0%, #f6f8fb 42%, #edf4ff 100%)'
              : 'linear-gradient(135deg, #191312 0%, #111827 48%, #101820 100%)',
        }}
      >
        <Stack
          spacing={3}
          sx={{
            width: '100%',
            maxWidth: 980,
          }}
        >
          <Box
            sx={{
              alignSelf: 'flex-end',
              display: 'flex',
              alignItems: 'center',
              gap: 1,
              px: 1,
              py: 0.75,
              borderRadius: 2,
              bgcolor: (theme) =>
                theme.palette.mode === 'light'
                  ? 'rgba(255,255,255,0.78)'
                  : 'rgba(17,24,39,0.72)',
              boxShadow: '0 10px 30px rgba(15, 23, 42, 0.08)',
              backdropFilter: 'blur(14px)',
            }}
          >
            <LanguageRounded fontSize="small" color="action" />
            {loginLanguages.map(({ code, label }) => (
              <Button
                key={code}
                size="small"
                variant={currentLanguage === code ? 'contained' : 'text'}
                onClick={() => switchLanguage(code)}
                sx={{ minWidth: 0, borderRadius: 1.25, px: 1.25 }}
              >
                {label}
              </Button>
            ))}
          </Box>

          <Box
            sx={{
              display: 'grid',
              gridTemplateColumns: { xs: '1fr', md: '1.05fr 0.95fr' },
              gap: { xs: 3, md: 5 },
              alignItems: 'center',
            }}
          >
            <Stack spacing={2.25}>
              <Box
                component="img"
                src={appLogo}
                alt="ChinEuro Secure Access"
                sx={{
                  width: 76,
                  height: 76,
                  borderRadius: 2.5,
                  boxShadow: '0 16px 40px rgba(124, 35, 31, 0.28)',
                }}
              />
              <Stack spacing={1.25}>
                <Typography
                  variant="h3"
                  sx={{
                    fontWeight: 800,
                    letterSpacing: 0,
                    lineHeight: 1.08,
                    fontSize: { xs: 34, sm: 44, md: 52 },
                  }}
                >
                  {t('home.enterprise.login.headline')}
                </Typography>
                <Typography
                  variant="body1"
                  color="text.secondary"
                  sx={{ maxWidth: 520, lineHeight: 1.75 }}
                >
                  {t('home.enterprise.login.description')}
                </Typography>
              </Stack>
            </Stack>

            <EnhancedCard
              title={t('home.enterprise.login.cardTitle')}
              icon={<LoginRounded />}
              iconColor="primary"
            >
              <Stack spacing={2.5} sx={{ py: 1 }}>
                <Stack spacing={0.75}>
                  <Typography variant="h5" sx={{ fontWeight: 700 }}>
                    {t('home.enterprise.login.title')}
                  </Typography>
                  <Typography variant="body2" color="text.secondary">
                    {t('home.enterprise.login.subtitle')}
                  </Typography>
                </Stack>
                <Button
                  size="large"
                  variant="contained"
                  startIcon={
                    loggingIn ? (
                      <CircularProgress size={18} color="inherit" />
                    ) : (
                      <LoginRounded />
                    )
                  }
                  disabled={loggingIn}
                  onClick={onStartLogin}
                  sx={{
                    alignSelf: 'flex-start',
                    minWidth: 190,
                    borderRadius: 1.5,
                  }}
                >
                  {loggingIn
                    ? t('home.enterprise.login.loggingIn')
                    : t('home.enterprise.login.action')}
                </Button>
                <Typography variant="caption" color="text.secondary">
                  {t('home.enterprise.login.securityHint')}
                </Typography>
              </Stack>
            </EnhancedCard>
          </Box>
        </Stack>
      </Box>
    )
  }

  return (
    <Stack spacing={1.5}>
      <EnhancedCard
        title={t('home.enterprise.access.title')}
        icon={<CheckCircleRounded />}
        iconColor="success"
        action={
          <Stack direction="row" spacing={1}>
            <Button
              size="small"
              variant="outlined"
              startIcon={
                syncing ? <CircularProgress size={16} /> : <SyncRounded />
              }
              disabled={syncing}
              onClick={onSyncPolicy}
              sx={{ borderRadius: 1.5 }}
            >
              {t('home.enterprise.access.sync')}
            </Button>
            <Button
              size="small"
              color="inherit"
              variant="outlined"
              startIcon={<LogoutRounded />}
              onClick={onLogout}
              sx={{ borderRadius: 1.5 }}
            >
              {t('home.enterprise.access.signOut')}
            </Button>
          </Stack>
        }
      >
        <Stack spacing={1.25}>
          <Box sx={{ display: 'flex', alignItems: 'center', gap: 1 }}>
            <Typography variant="body2" sx={{ fontWeight: 600 }}>
              {session?.username || t('home.enterprise.access.account')}
            </Typography>
            <Chip
              size="small"
              color="success"
              label={t('home.enterprise.access.signedIn')}
            />
          </Box>
          <EnterpriseStatusLine
            label={t('home.enterprise.access.policy')}
            value={
              policy
                ? t('home.enterprise.access.syncedAt', {
                    time: policy.syncedAt,
                  })
                : t('home.enterprise.access.pendingSync')
            }
          />
          <EnterpriseAllowedResources
            resources={allowedResources}
            emptyText={t('home.enterprise.resources.emptyAfterSync')}
          />
        </Stack>
      </EnhancedCard>
    </Stack>
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

async function syncPolicyAfterLogin(
  refetchEnterprise: ReturnType<typeof useEnterprise>['refetchEnterprise'],
  t: (key: string) => string,
) {
  try {
    await syncEnterprisePolicy()
    await refetchEnterprise()
    return true
  } catch (err) {
    await refetchEnterprise()
    showNotice.error(t('home.enterprise.notices.syncFailed'))
    console.error('[enterprise-login-sync] failed:', err)
    return false
  }
}

function sleep(ms: number) {
  return new Promise((resolve) => window.setTimeout(resolve, ms))
}
