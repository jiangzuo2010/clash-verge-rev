import {
  BadgeRounded,
  CheckCircleRounded,
  GppGoodOutlined,
  LanguageRounded,
  LockRounded,
  LogoutRounded,
  OpenInNewRounded,
  PolicyRounded,
  SyncRounded,
} from '@mui/icons-material'
import {
  Box,
  Button,
  CircularProgress,
  Divider,
  Stack,
  type Theme,
  Typography,
} from '@mui/material'
import { useLockFn } from 'ahooks'
import { useEffect, useRef, useState } from 'react'
import { useTranslation } from 'react-i18next'

import appLogo from '@/assets/image/app_logo.png'
import { EnhancedCard } from '@/components/home/enhanced-card'
import { useEnterprise } from '@/hooks/use-enterprise'
import { useI18n } from '@/hooks/use-i18n'
import { syncEnterprisePolicy } from '@/services/enterprise'
import { showNotice } from '@/services/notice-service'
import { useEnterpriseLoginResult } from '@/store/app-store-context'
import { version as appVersion } from '@root/package.json'

interface EnterpriseAccessHomeProps {
  fullscreen?: boolean
}

const loginLanguages = [
  { code: 'zh', label: '中文' },
  { code: 'en', label: 'English' },
  { code: 'pl', label: 'Polski' },
]

const BRAND_RED = '#a3262f'
const BRAND_RED_DARK = '#5e1219'
const BRAND_GOLD = '#f2c14e'

const brandText = (theme: Theme) =>
  theme.palette.mode === 'dark' ? '#f08a91' : BRAND_RED

const loginFeatures = [
  { key: 'identity', icon: <BadgeRounded /> },
  { key: 'encryption', icon: <LockRounded /> },
  { key: 'policy', icon: <PolicyRounded /> },
] as const

export const EnterpriseAccessHome = ({
  fullscreen = false,
}: EnterpriseAccessHomeProps) => {
  const { t } = useTranslation()
  const { currentLanguage, switchLanguage } = useI18n()
  const { enterprise, clearSession, startLogin, refetchEnterprise } =
    useEnterprise()
  const session = enterprise?.session
  const authenticated = session?.authenticated ?? false
  const ready = Boolean(enterprise?.policyStatus)
  const [loggingIn, setLoggingIn] = useState(false)
  const [syncing, setSyncing] = useState(false)

  const onStartLogin = useLockFn(async () => {
    try {
      await startLogin(true)
      setLoggingIn(true)
    } catch (err) {
      setLoggingIn(false)
      showNotice.error(t('home.enterprise.notices.loginFailed'))
      console.error('[enterprise-login] failed:', err)
    }
  })

  const onLoginFinished = useLockFn(async (success: boolean) => {
    setLoggingIn(false)
    if (!success) {
      await refetchEnterprise()
      showNotice.error(t('home.enterprise.notices.loginFailed'))
      return
    }
    setSyncing(true)
    const synced = await syncPolicyAfterLogin(refetchEnterprise, t)
    setSyncing(false)
    if (synced) {
      showNotice.success(t('home.enterprise.notices.connected'))
    }
  })

  const loginResult = useEnterpriseLoginResult()
  // A sign-in that finished before this page mounted is already reflected in the refetched state.
  const handledLoginRevisionRef = useRef(loginResult.revision)
  useEffect(() => {
    if (loginResult.revision === handledLoginRevisionRef.current) return
    handledLoginRevisionRef.current = loginResult.revision
    void onLoginFinished(loginResult.success)
  }, [loginResult, onLoginFinished])

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

  const languageSwitcher = (
    <Box
      sx={{
        display: 'flex',
        alignItems: 'center',
        gap: 0.5,
        px: 1,
        py: 0.5,
        borderRadius: 2,
        border: 1,
        borderColor: 'divider',
        bgcolor: 'background.paper',
      }}
    >
      <LanguageRounded fontSize="small" color="action" />
      {loginLanguages.map(({ code, label }) => (
        <Button
          key={code}
          size="small"
          color="inherit"
          onClick={() => switchLanguage(code)}
          sx={{
            minWidth: 0,
            px: 1,
            textTransform: 'none',
            fontWeight: currentLanguage === code ? 700 : 400,
            color: currentLanguage === code ? brandText : 'text.secondary',
          }}
        >
          {label}
        </Button>
      ))}
    </Box>
  )

  if (!authenticated) {
    return (
      <Box
        sx={{
          minHeight: fullscreen ? '100vh' : 'calc(100vh - 96px)',
          display: 'flex',
          bgcolor: 'background.paper',
        }}
      >
        <Box
          sx={{
            display: { xs: 'none', sm: 'flex' },
            flexDirection: 'column',
            width: '42%',
            maxWidth: 420,
            flexShrink: 0,
            p: 5,
            color: '#fff',
            position: 'relative',
            overflow: 'hidden',
            background: `linear-gradient(160deg, ${BRAND_RED} 0%, #7a1a22 55%, ${BRAND_RED_DARK} 100%)`,
          }}
        >
          <Box
            sx={{
              position: 'absolute',
              right: -140,
              bottom: -140,
              width: 380,
              height: 380,
              borderRadius: '50%',
              border: '56px solid rgba(255, 255, 255, 0.06)',
            }}
          />
          <Stack direction="row" spacing={1.5} sx={{ alignItems: 'center' }}>
            <Box
              component="img"
              src={appLogo}
              alt=""
              sx={{
                width: 44,
                height: 44,
                borderRadius: 1.5,
                boxShadow: '0 8px 24px rgba(0, 0, 0, 0.25)',
              }}
            />
            <Box>
              <Typography
                sx={{ fontSize: 18, fontWeight: 700, lineHeight: 1.2 }}
              >
                ChinEuro
              </Typography>
              <Typography
                sx={{
                  fontSize: 12,
                  letterSpacing: '0.12em',
                  textTransform: 'uppercase',
                  opacity: 0.75,
                }}
              >
                Secure Access
              </Typography>
            </Box>
          </Stack>
          <Box
            sx={{
              flex: 1,
              display: 'flex',
              flexDirection: 'column',
              justifyContent: 'center',
              position: 'relative',
            }}
          >
            <Box
              sx={{
                width: 40,
                height: 4,
                borderRadius: 2,
                bgcolor: BRAND_GOLD,
                mb: 2.5,
              }}
            />
            <Typography sx={{ fontSize: 28, fontWeight: 700, lineHeight: 1.3 }}>
              {t('home.enterprise.login.tagline')}
            </Typography>
            <Stack spacing={1.75} sx={{ mt: 4 }}>
              {loginFeatures.map(({ key, icon }) => (
                <Stack
                  key={key}
                  direction="row"
                  spacing={1.5}
                  sx={{ alignItems: 'center' }}
                >
                  <Box
                    sx={{
                      width: 32,
                      height: 32,
                      borderRadius: 1.5,
                      display: 'grid',
                      placeItems: 'center',
                      bgcolor: 'rgba(255, 255, 255, 0.12)',
                      '& svg': { fontSize: 18 },
                    }}
                  >
                    {icon}
                  </Box>
                  <Typography sx={{ fontSize: 15, opacity: 0.92 }}>
                    {t(`home.enterprise.login.features.${key}`)}
                  </Typography>
                </Stack>
              ))}
            </Stack>
          </Box>
          <Typography sx={{ fontSize: 12, opacity: 0.6, position: 'relative' }}>
            © chineuro · v{appVersion}
          </Typography>
        </Box>

        <Box
          sx={{
            flex: 1,
            display: 'flex',
            flexDirection: 'column',
            p: { xs: 3, sm: 5 },
          }}
        >
          <Box sx={{ alignSelf: 'flex-end' }}>{languageSwitcher}</Box>
          <Box
            sx={{
              flex: 1,
              display: 'flex',
              alignItems: 'center',
              justifyContent: 'center',
            }}
          >
            <Stack spacing={3.5} sx={{ width: '100%', maxWidth: 360 }}>
              <Box
                component="img"
                src={appLogo}
                alt="ChinEuro Secure Access"
                sx={{
                  display: { xs: 'block', sm: 'none' },
                  width: 56,
                  height: 56,
                  borderRadius: 2,
                }}
              />
              <Stack spacing={1}>
                <Typography variant="h4" sx={{ fontWeight: 700 }}>
                  {t('home.enterprise.login.title')}
                </Typography>
                <Typography color="text.secondary">
                  {t('home.enterprise.login.subtitle')}
                </Typography>
              </Stack>
              {loggingIn ? (
                <Box
                  sx={{
                    p: 2.5,
                    borderRadius: 2,
                    border: 1,
                    borderColor: 'divider',
                    bgcolor: 'action.hover',
                  }}
                >
                  <Stack
                    direction="row"
                    spacing={1.5}
                    sx={{ alignItems: 'center' }}
                  >
                    <CircularProgress size={20} sx={{ color: BRAND_RED }} />
                    <Typography sx={{ fontWeight: 600 }}>
                      {t('home.enterprise.login.loggingIn')}
                    </Typography>
                  </Stack>
                  <Typography
                    variant="body2"
                    color="text.secondary"
                    sx={{ mt: 1.25 }}
                  >
                    {t('home.enterprise.login.waitingHint')}
                  </Typography>
                  <Stack direction="row" spacing={1} sx={{ mt: 1.5, ml: -1 }}>
                    <Button
                      size="small"
                      startIcon={<OpenInNewRounded />}
                      onClick={onStartLogin}
                      sx={{ color: brandText, textTransform: 'none' }}
                    >
                      {t('home.enterprise.login.reopen')}
                    </Button>
                    <Button
                      size="small"
                      color="inherit"
                      onClick={() => setLoggingIn(false)}
                      sx={{ textTransform: 'none' }}
                    >
                      {t('home.enterprise.login.cancel')}
                    </Button>
                  </Stack>
                </Box>
              ) : (
                <Button
                  fullWidth
                  size="large"
                  variant="contained"
                  disableElevation
                  startIcon={<LockRounded />}
                  onClick={onStartLogin}
                  sx={{
                    height: 48,
                    borderRadius: 1.5,
                    fontSize: 15,
                    fontWeight: 600,
                    textTransform: 'none',
                    bgcolor: BRAND_RED,
                    '&:hover': { bgcolor: '#8a1f27' },
                  }}
                >
                  {t('home.enterprise.login.action')}
                </Button>
              )}
              <Divider />
              <Stack
                direction="row"
                spacing={1}
                sx={{ alignItems: 'flex-start' }}
              >
                <GppGoodOutlined fontSize="small" color="action" />
                <Typography variant="caption" color="text.secondary">
                  {t('home.enterprise.login.securityHint')}
                </Typography>
              </Stack>
            </Stack>
          </Box>
        </Box>
      </Box>
    )
  }

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
      <Stack spacing={3} sx={{ width: '100%', maxWidth: 460 }}>
        <Box sx={{ alignSelf: 'flex-end' }}>{languageSwitcher}</Box>
        <EnhancedCard
          title={t('home.enterprise.login.cardTitle')}
          icon={<CheckCircleRounded />}
          iconColor={ready ? 'success' : 'warning'}
        >
          <Stack spacing={3} sx={{ py: 1, alignItems: 'center' }}>
            <Box
              component="img"
              src={appLogo}
              alt="ChinEuro Secure Access"
              sx={{
                width: 64,
                height: 64,
                borderRadius: 2,
                boxShadow: '0 12px 32px rgba(124, 35, 31, 0.24)',
              }}
            />
            <Typography variant="h5" sx={{ fontWeight: 700 }}>
              {t('home.enterprise.connection.greeting', {
                name: session?.realName || session?.username || '',
              })}
            </Typography>
            <Stack spacing={0.75} sx={{ alignItems: 'center' }}>
              <Box sx={{ display: 'flex', alignItems: 'center', gap: 1 }}>
                {syncing ? (
                  <CircularProgress size={16} />
                ) : (
                  <Box
                    sx={{
                      width: 10,
                      height: 10,
                      borderRadius: '50%',
                      bgcolor: ready ? 'success.main' : 'warning.main',
                    }}
                  />
                )}
                <Typography variant="subtitle1" sx={{ fontWeight: 600 }}>
                  {syncing
                    ? t('home.enterprise.connection.connecting')
                    : ready
                      ? t('home.enterprise.connection.ready')
                      : t('home.enterprise.connection.notReady')}
                </Typography>
              </Box>
              <Typography
                variant="body2"
                color="text.secondary"
                sx={{ textAlign: 'center' }}
              >
                {ready || syncing
                  ? t('home.enterprise.connection.readyHint')
                  : t('home.enterprise.connection.notReadyHint')}
              </Typography>
            </Stack>
            <Stack direction="row" spacing={1.5}>
              {!ready && (
                <Button
                  variant="contained"
                  startIcon={<SyncRounded />}
                  disabled={syncing}
                  onClick={onSyncPolicy}
                  sx={{ borderRadius: 1.5 }}
                >
                  {t('home.enterprise.connection.retry')}
                </Button>
              )}
              <Button
                color="inherit"
                variant="outlined"
                startIcon={<LogoutRounded />}
                onClick={onLogout}
                sx={{ borderRadius: 1.5 }}
              >
                {t('home.enterprise.access.signOut')}
              </Button>
            </Stack>
          </Stack>
        </EnhancedCard>
      </Stack>
    </Box>
  )
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
