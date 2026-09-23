import { DragDropProvider, KeyboardSensor, PointerSensor } from '@dnd-kit/react'
import { Box, List, Menu, MenuItem } from '@mui/material'
import { useCallback, useMemo, useState } from 'react'
import { useTranslation } from 'react-i18next'

import appLogo from '@/assets/image/app_logo.png'
import { useEnterprise } from '@/hooks/use-enterprise'
import { useVerge } from '@/hooks/use-verge'
import { useNavMenuOrder } from '@/pages/_layout/hooks'
import { navItems } from '@/pages/_navigation'
import { hasEnterpriseAdvancedAccess } from '@/services/enterprise-access'

import { SortableItem } from '../base'

import { LayoutItem } from './layout-item'
import { LayoutTraffic } from './layout-traffic'
import { UpdateButton } from './update-button'

type MenuContextPosition = { top: number; left: number }

interface LayoutSidebarProps {
  isDark: boolean
  isCollapsed: boolean
}

const SENSORS = [PointerSensor, KeyboardSensor]

const ENTERPRISE_BASE_PATHS = new Set(['/'])
const ENTERPRISE_ADVANCED_PATHS = new Set([
  '/proxies',
  '/connections',
  '/rules',
  '/logs',
  '/settings',
])
const ENTERPRISE_SIGNED_OUT_PATHS = new Set(['/'])

export const LayoutSidebar = (props: LayoutSidebarProps) => {
  const { isDark, isCollapsed } = props
  const { t } = useTranslation()
  const { verge, mutateVerge, patchVerge } = useVerge()
  const { enterprise } = useEnterprise()
  const enterpriseManaged = enterprise?.config.enabled ?? false
  const enterpriseAuthenticated = enterprise?.session.authenticated ?? false
  const enterpriseAdvancedAccess = hasEnterpriseAdvancedAccess(enterprise)
  const [menuUnlocked, setMenuUnlocked] = useState(false)
  const [menuContextPosition, setMenuContextPosition] =
    useState<MenuContextPosition | null>(null)

  const handleMenuOrderOptimisticUpdate = useCallback(
    (order: string[]) => {
      mutateVerge(
        (prev) => (prev ? { ...prev, menu_order: order } : prev),
        false,
      )
    },
    [mutateVerge],
  )

  const handleMenuOrderPersist = useCallback(
    (order: string[]) => patchVerge({ menu_order: order }),
    [patchVerge],
  )

  const {
    menuOrder,
    navItemMap,
    handleMenuDragEnd,
    isDefaultOrder,
    resetMenuOrder,
  } = useNavMenuOrder({
    enabled: menuUnlocked,
    items: navItems,
    storedOrder: verge?.menu_order,
    onOptimisticUpdate: handleMenuOrderOptimisticUpdate,
    onPersist: handleMenuOrderPersist,
  })

  const visibleMenuOrder = useMemo(() => {
    if (!enterpriseManaged) {
      return menuOrder
    }
    if (!enterpriseAuthenticated) {
      return menuOrder.filter((path) => ENTERPRISE_SIGNED_OUT_PATHS.has(path))
    }
    return menuOrder.filter((path) => {
      if (ENTERPRISE_BASE_PATHS.has(path)) {
        return true
      }
      if (ENTERPRISE_ADVANCED_PATHS.has(path)) {
        return enterpriseAdvancedAccess
      }
      return false
    })
  }, [
    enterpriseAdvancedAccess,
    enterpriseAuthenticated,
    enterpriseManaged,
    menuOrder,
  ])

  const handleMenuContextMenu = useCallback(
    (event: React.MouseEvent<HTMLElement>) => {
      event.preventDefault()
      event.stopPropagation()
      setMenuContextPosition({ top: event.clientY, left: event.clientX })
    },
    [],
  )

  const handleMenuContextClose = useCallback(() => {
    setMenuContextPosition(null)
  }, [])

  const handleResetMenuOrder = useCallback(() => {
    setMenuContextPosition(null)
    void resetMenuOrder()
  }, [resetMenuOrder])

  const handleUnlockMenu = useCallback(() => {
    setMenuUnlocked(true)
    setMenuContextPosition(null)
  }, [])

  const handleLockMenu = useCallback(() => {
    setMenuUnlocked(false)
    setMenuContextPosition(null)
  }, [])

  const handleToggleNavCollapsed = useCallback(() => {
    setMenuContextPosition(null)
    void patchVerge({ collapse_navbar: !isCollapsed })
  }, [isCollapsed, patchVerge])

  // Navigation menu items
  const navMenuItems = visibleMenuOrder.map((path, index) => {
    const item = navItemMap.get(path)
    if (!item) return null

    return (
      <SortableItem
        key={item.path}
        id={item.path}
        index={index}
        disabled={!menuUnlocked}
      >
        {(sortable) => (
          <LayoutItem to={item.path} icon={item.icon} sortable={sortable}>
            {t(item.label)}
          </LayoutItem>
        )}
      </SortableItem>
    )
  })

  return (
    <div className="layout-content__left">
      {/* Logo */}
      <div className="the-logo" data-tauri-drag-region="false">
        <div
          data-tauri-drag-region="true"
          style={{
            height: '27px',
            display: 'flex',
            justifyContent: 'space-between',
          }}
        >
          <Box
            component="img"
            src={appLogo}
            alt="ChinEuro Secure Access"
            style={{
              height: '36px',
              width: '36px',
              marginTop: '-3px',
              marginRight: '5px',
              marginLeft: '-3px',
              borderRadius: '8px',
            }}
          />
          <Box
            component="span"
            sx={{
              alignSelf: 'center',
              color: isDark ? 'white' : 'black',
              fontFamily: "Georgia, 'Times New Roman', serif",
              fontSize: 15,
              fontWeight: 600,
              lineHeight: 1.05,
            }}
          >
            ChinEuro
            <br />
            Secure Access
          </Box>
        </div>
        <UpdateButton className="the-newbtn" />
      </div>

      {/* Edit navigation menu badge */}
      {menuUnlocked && (
        <Box
          sx={(theme) => ({
            px: 1.5,
            py: 0.75,
            mx: 'auto',
            mb: 1,
            maxWidth: 250,
            borderRadius: 1.5,
            fontSize: 12,
            fontWeight: 600,
            textAlign: 'center',
            color: theme.palette.warning.contrastText,
            bgcolor:
              theme.palette.mode === 'light'
                ? theme.palette.warning.main
                : theme.palette.warning.dark,
          })}
        >
          {t('layout.components.navigation.menu.reorderMode')}
        </Box>
      )}

      {/* Navigation menu */}
      <List className="the-menu" onContextMenu={handleMenuContextMenu}>
        <DragDropProvider sensors={SENSORS} onDragEnd={handleMenuDragEnd}>
          {navMenuItems}
        </DragDropProvider>
      </List>

      {/* Context menu */}
      <Menu
        open={Boolean(menuContextPosition)}
        onClose={handleMenuContextClose}
        anchorReference="anchorPosition"
        anchorPosition={
          menuContextPosition
            ? {
                top: menuContextPosition.top,
                left: menuContextPosition.left,
              }
            : undefined
        }
        transitionDuration={200}
        slotProps={{
          list: {
            sx: { py: 0.5 },
          },
        }}
      >
        <MenuItem onClick={handleToggleNavCollapsed} dense>
          {isCollapsed
            ? t('layout.components.navigation.menu.expandNavBar')
            : t('layout.components.navigation.menu.collapseNavBar')}
        </MenuItem>
        <MenuItem
          onClick={menuUnlocked ? handleLockMenu : handleUnlockMenu}
          dense
        >
          {menuUnlocked
            ? t('layout.components.navigation.menu.lock')
            : t('layout.components.navigation.menu.unlock')}
        </MenuItem>
        <MenuItem
          onClick={handleResetMenuOrder}
          dense
          disabled={isDefaultOrder}
        >
          {t('layout.components.navigation.menu.restoreDefaultOrder')}
        </MenuItem>
      </Menu>

      {/* Traffic */}
      <div className="the-traffic">
        <LayoutTraffic />
      </div>
    </div>
  )
}
