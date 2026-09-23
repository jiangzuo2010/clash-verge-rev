import type { EnterpriseStateView } from '@/services/enterprise'

export function hasEnterpriseAdvancedAccess(
  enterprise?: EnterpriseStateView | null,
) {
  return enterprise?.advancedAccess === true
}
