import { Paper, ThemeProvider } from '@mui/material'
import dayjs from 'dayjs'
import relativeTime from 'dayjs/plugin/relativeTime'
import { Suspense, lazy, useCallback, useEffect, useMemo, useRef } from 'react'
import { useTranslation } from 'react-i18next'
import { Outlet, useLocation, useNavigate } from 'react-router'

import { BaseErrorBoundary } from '@/components/base'
import { LayoutSidebar } from '@/components/layout/layout-sidebar'
import { NoticeManager } from '@/components/layout/notice-manager'
import { ServiceMigrationDialog } from '@/components/layout/service-migration-dialog'
import { SysproxyPrivilegeDialog } from '@/components/layout/sysproxy-privilege-dialog'
import {
  WindowControls,
  WindowResizeHandles,
} from '@/components/layout/window-controller'
import { useEnterprise } from '@/hooks/use-enterprise'
import { useI18n } from '@/hooks/use-i18n'
import { useVerge } from '@/hooks/use-verge'
import { useWindowDecorations } from '@/hooks/use-window'
import { hasEnterpriseAdvancedAccess } from '@/services/enterprise-access'
import { useThemeMode } from '@/services/states'
import getSystem from '@/utils/get-system'

import {
  useCustomTheme,
  useLayoutEvents,
  useLoadingOverlay,
  usePendingFailures,
} from './_layout/hooks'
import { handleNoticeMessage } from './_layout/utils'

import 'dayjs/locale/ru'
import 'dayjs/locale/zh-cn'

dayjs.extend(relativeTime)

const OS = getSystem()

const LogsPage = lazy(() => import('./logs'))

const ENTERPRISE_ALWAYS_RESTRICTED_PATHS = new Set(['/profile', '/unlock'])
const ENTERPRISE_ADVANCED_PATHS = new Set([
  '/proxies',
  '/connections',
  '/rules',
  '/logs',
  '/settings',
])
const ENTERPRISE_SIGNED_OUT_PATHS = new Set(['/'])

const Layout = () => {
  const mode = useThemeMode()
  const isDark = mode !== 'light'
  const { t } = useTranslation()
  const { theme } = useCustomTheme()
  const { verge } = useVerge()
  const { enterprise } = useEnterprise()
  const { language } = verge ?? {}
  const navCollapsed = verge?.collapse_navbar ?? false
  const { switchLanguage } = useI18n()
  const navigate = useNavigate()
  const { pathname } = useLocation()
  const isLogsPage = pathname === '/logs'
  const enterpriseManaged = enterprise?.config.enabled ?? false
  const enterpriseAuthenticated = enterprise?.session.authenticated ?? false
  const enterpriseAdvancedAccess = hasEnterpriseAdvancedAccess(enterprise)
  const enterpriseSignedOut = enterpriseManaged && !enterpriseAuthenticated
  const logsPageMountedRef = useRef(false)
  if (isLogsPage) logsPageMountedRef.current = true
  const themeReady = useMemo(() => Boolean(theme), [theme])
  const windowControlsRef = useRef<any>(null)
  const { decorated } = useWindowDecorations()

  const customTitlebar = useMemo(
    () =>
      decorated === false ? (
        <div className="the_titlebar">
          <div
            className="the_titlebar-drag-region"
            data-tauri-drag-region="true"
          />
          <WindowControls ref={windowControlsRef} />
        </div>
      ) : null,
    [decorated],
  )

  useLoadingOverlay(themeReady)

  const handleNotice = useCallback(
    (payload: [string, string]) => {
      const [status, msg] = payload
      try {
        handleNoticeMessage(status, msg, t, navigate)
      } catch (error) {
        console.error('[通知处理] 失败:', error)
      }
    },
    [t, navigate],
  )

  useLayoutEvents(handleNotice)
  usePendingFailures()

  useEffect(() => {
    if (
      enterpriseManaged &&
      !enterpriseAuthenticated &&
      !ENTERPRISE_SIGNED_OUT_PATHS.has(pathname)
    ) {
      navigate('/', { replace: true })
      return
    }

    if (
      enterpriseManaged &&
      enterpriseAuthenticated &&
      (ENTERPRISE_ALWAYS_RESTRICTED_PATHS.has(pathname) ||
        (ENTERPRISE_ADVANCED_PATHS.has(pathname) && !enterpriseAdvancedAccess))
    ) {
      navigate('/', { replace: true })
    }
  }, [
    enterpriseAdvancedAccess,
    enterpriseAuthenticated,
    enterpriseManaged,
    navigate,
    pathname,
  ])

  useEffect(() => {
    if (language) {
      dayjs.locale(language === 'zh' ? 'zh-cn' : language)
      switchLanguage(language)
    }
  }, [language, switchLanguage])

  if (!themeReady) {
    return (
      <div
        style={{
          width: '100vw',
          height: '100vh',
          background: mode === 'light' ? '#fff' : '#181a1b',
          transition: 'background 0.2s',
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'center',
          color: mode === 'light' ? '#333' : '#fff',
        }}
      ></div>
    )
  }

  if (enterpriseSignedOut) {
    return (
      <ThemeProvider theme={theme}>
        <NoticeManager position={verge?.notice_position} />
        <Paper
          square
          elevation={0}
          className={`${OS} layout layout--enterprise-login`}
          sx={[
            ({ palette }) => ({
              bgcolor: palette.background.paper,
              width: '100vw',
              height: '100vh',
              overflow: 'hidden',
            }),
            OS === 'linux' ? { borderRadius: '8px' } : {},
          ]}
        >
          {customTitlebar}
          <BaseErrorBoundary>
            <Outlet />
          </BaseErrorBoundary>
        </Paper>
      </ThemeProvider>
    )
  }

  return (
    <ThemeProvider theme={theme}>
      {/* 左侧底部窗口控制按钮 */}
      <NoticeManager position={verge?.notice_position} />
      <ServiceMigrationDialog />
      <SysproxyPrivilegeDialog />
      <div
        style={{
          animation: 'fadeIn 0.5s',
          WebkitAnimation: 'fadeIn 0.5s',
        }}
      />
      <style>
        {`
            @keyframes fadeIn {
              from { opacity: 0; }
              to { opacity: 1; }
            }
          `}
      </style>
      <Paper
        square
        elevation={0}
        className={`${OS} layout${navCollapsed ? ' layout--nav-collapsed' : ''}`}
        style={{
          borderTopLeftRadius: '0px',
          borderTopRightRadius: '0px',
        }}
        onContextMenu={(e) => {
          if (
            OS === 'windows' &&
            !['input', 'textarea'].includes(
              e.currentTarget.tagName.toLowerCase(),
            ) &&
            !e.currentTarget.isContentEditable
          ) {
            e.preventDefault()
          }
        }}
        sx={[
          ({ palette }) => ({ bgcolor: palette.background.paper }),
          OS === 'linux'
            ? {
                borderRadius: '8px',
                width: '100vw',
                height: '100vh',
              }
            : {},
        ]}
      >
        {decorated === false && <WindowResizeHandles />}

        {/* Custom titlebar - rendered only when decorated is false, memoized for performance */}
        {customTitlebar}

        <div className="layout-content">
          <LayoutSidebar isDark={isDark} isCollapsed={navCollapsed} />

          <div className="layout-content__right">
            <div className="the-bar"></div>
            <div className="the-content">
              <BaseErrorBoundary>
                <Outlet />
              </BaseErrorBoundary>
              {logsPageMountedRef.current && (
                <div
                  style={{
                    position: 'absolute',
                    top: 0,
                    left: 0,
                    right: 0,
                    bottom: 0,
                    display: isLogsPage ? undefined : 'none',
                  }}
                >
                  <Suspense fallback={null}>
                    <LogsPage />
                  </Suspense>
                </div>
              )}
            </div>
          </div>
        </div>
      </Paper>
    </ThemeProvider>
  )
}

export default Layout
