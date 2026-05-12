import {
  BusinessRounded,
  CheckCircleRounded,
  ErrorOutlineRounded,
  SyncRounded,
} from '@mui/icons-material'
import { Box, Button, Chip, Stack, Typography } from '@mui/material'
import { useLockFn } from 'ahooks'

import { EnhancedCard } from '@/components/home/enhanced-card'
import { useEnterprise } from '@/hooks/use-enterprise'
import { syncEnterprisePolicy } from '@/services/enterprise'
import { showNotice } from '@/services/notice-service'

export const EnterpriseStatusCard = () => {
  const { enterprise, refetchEnterprise } = useEnterprise()
  const session = enterprise?.session
  const policy = enterprise?.policyStatus
  const authenticated = session?.authenticated ?? false

  const onSyncPolicy = useLockFn(async () => {
    try {
      await syncEnterprisePolicy()
      await refetchEnterprise()
      showNotice.success('企业策略已同步')
    } catch (err) {
      showNotice.error(err)
    }
  })

  return (
    <EnhancedCard
      title="企业受管"
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
          同步
        </Button>
      }
    >
      <Stack spacing={1.25}>
        <Box sx={{ display: 'flex', alignItems: 'center', gap: 1 }}>
          <BusinessRounded fontSize="small" color="action" />
          <Typography variant="body2" sx={{ fontWeight: 600 }}>
            {session?.username || '未登录'}
          </Typography>
          <Chip
            size="small"
            color={authenticated ? 'success' : 'warning'}
            label={authenticated ? '已登录' : '待登录'}
          />
        </Box>

        <StatusLine
          label="租户"
          value={session?.tenantCode || session?.tenantId}
        />
        <StatusLine label="策略版本" value={policy?.version} />
        <StatusLine label="策略过期" value={policy?.expiresAt} />
        <StatusLine label="上次同步" value={policy?.syncedAt} />
      </Stack>
    </EnhancedCard>
  )
}

const StatusLine = ({ label, value }: { label: string; value?: string }) => (
  <Box sx={{ display: 'flex', justifyContent: 'space-between', gap: 2 }}>
    <Typography variant="caption" color="text.secondary">
      {label}
    </Typography>
    <Typography
      variant="caption"
      title={value || '无'}
      sx={{
        minWidth: 0,
        textAlign: 'right',
        overflow: 'hidden',
        textOverflow: 'ellipsis',
        whiteSpace: 'nowrap',
      }}
    >
      {value || '无'}
    </Typography>
  </Box>
)
