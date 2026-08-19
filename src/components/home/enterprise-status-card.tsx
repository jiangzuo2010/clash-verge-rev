import {
  CheckCircleRounded,
  ErrorOutlineRounded,
  SyncRounded,
} from '@mui/icons-material'
import { Box, Button, Chip, Stack, Typography } from '@mui/material'
import { useLockFn } from 'ahooks'
import { useTranslation } from 'react-i18next'

import { EnhancedCard } from '@/components/home/enhanced-card'
import {
  EnterpriseAllowedResources,
  EnterpriseStatusLine,
} from '@/components/home/enterprise-access-details'
import { useEnterprise } from '@/hooks/use-enterprise'
import { syncEnterprisePolicy } from '@/services/enterprise'
import { hasEnterpriseAdvancedAccess } from '@/services/enterprise-access'
import { showNotice } from '@/services/notice-service'

export const EnterpriseStatusCard = () => {
  const { t } = useTranslation()
  const { enterprise, refetchEnterprise } = useEnterprise()
  const session = enterprise?.session
  const policy = enterprise?.policyStatus
  const allowedResources = enterprise?.allowedResources ?? []
  const authenticated = session?.authenticated ?? false
  const advancedAccess = hasEnterpriseAdvancedAccess(enterprise)

  const onSyncPolicy = useLockFn(async () => {
    try {
      await syncEnterprisePolicy()
      await refetchEnterprise()
      showNotice.success(t('home.enterprise.notices.synced'))
    } catch (err) {
      showNotice.error(err)
    }
  })

  return (
    <EnhancedCard
      title={t('home.enterprise.login.cardTitle')}
      icon={authenticated ? <CheckCircleRounded /> : <ErrorOutlineRounded />}
      iconColor={authenticated ? 'success' : 'warning'}
      action={
        <Button
          size="small"
          variant="outlined"
          startIcon={<SyncRounded />}
          disabled={!authenticated}
          onClick={onSyncPolicy}
          sx={{ borderRadius: 1.5 }}
        >
          {t('home.enterprise.access.syncConfig')}
        </Button>
      }
    >
      <Stack spacing={1.25}>
        <Box sx={{ display: 'flex', alignItems: 'center', gap: 1 }}>
          <Typography variant="body2" sx={{ fontWeight: 600 }}>
            {session?.username || t('home.enterprise.access.notSignedIn')}
          </Typography>
          <Chip
            size="small"
            color={authenticated ? 'success' : 'warning'}
            label={
              authenticated
                ? t('home.enterprise.access.signedIn')
                : t('home.enterprise.access.notSignedIn')
            }
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
        <EnterpriseAllowedResources resources={allowedResources} />
        {advancedAccess && (
          <>
            <EnterpriseStatusLine
              label={t('home.enterprise.access.tenant')}
              value={session?.tenantCode || session?.tenantId}
            />
            <EnterpriseStatusLine
              label={t('home.enterprise.access.version')}
              value={policy?.version}
            />
            <EnterpriseStatusLine
              label={t('home.enterprise.access.expiresAt')}
              value={policy?.expiresAt}
            />
          </>
        )}
      </Stack>
    </EnhancedCard>
  )
}
