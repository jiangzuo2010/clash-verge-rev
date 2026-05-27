import type { EnterpriseStateView } from '@/services/enterprise'

const ADVANCED_PERMISSIONS = new Set([
  '*',
  'company-proxy-desktop:advanced',
  'company-proxy-desktop:debug',
  'workspace:enterprise-proxy:view',
  'workspace:enterprise-proxy:manage',
])

const ADVANCED_ROLES = new Set([
  'SUPER_ADMIN',
  'ADMIN',
  'ENTERPRISE_PROXY_ADMIN',
  'ENTERPRISE_PROXY_MANAGER',
])

export function hasEnterpriseAdvancedAccess(
  enterprise?: EnterpriseStateView | null,
) {
  const session = enterprise?.session
  if (!enterprise?.config.enabled || !session?.authenticated) {
    return false
  }

  if (session.isSuperAdmin) {
    return true
  }

  const hasPermission = session.permissions?.some((permission) =>
    ADVANCED_PERMISSIONS.has(permission),
  )
  if (hasPermission) {
    return true
  }

  return session.roles?.some((role) => ADVANCED_ROLES.has(role)) ?? false
}
