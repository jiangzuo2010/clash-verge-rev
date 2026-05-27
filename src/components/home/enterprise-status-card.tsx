import {
  CheckCircleRounded,
  ErrorOutlineRounded,
  SyncRounded,
} from '@mui/icons-material'
import { Box, Button, Chip, Stack, Typography } from '@mui/material'
import { useLockFn } from 'ahooks'

import { EnhancedCard } from '@/components/home/enhanced-card'
import { useEnterprise } from '@/hooks/use-enterprise'
import { syncEnterprisePolicy } from '@/services/enterprise'
import { hasEnterpriseAdvancedAccess } from '@/services/enterprise-access'
import { showNotice } from '@/services/notice-service'

export const EnterpriseStatusCard = () => {
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
      showNotice.success('连接配置已同步')
    } catch (err) {
      showNotice.error(err)
    }
  })

  return (
    <EnhancedCard
      title="ChinEuro Secure Access"
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
          同步配置
        </Button>
      }
    >
      <Stack spacing={1.25}>
        <Box sx={{ display: 'flex', alignItems: 'center', gap: 1 }}>
          <Typography variant="body2" sx={{ fontWeight: 600 }}>
            {session?.username || '未登录'}
          </Typography>
          <Chip
            size="small"
            color={authenticated ? 'success' : 'warning'}
            label={authenticated ? '已登录' : '未登录'}
          />
        </Box>

        <StatusLine
          label="连接配置"
          value={policy ? `已同步 ${policy.syncedAt}` : '待同步'}
        />
        <AllowedResourcesList resources={allowedResources} />
        {advancedAccess && (
          <>
            <StatusLine
              label="租户"
              value={session?.tenantCode || session?.tenantId}
            />
            <StatusLine label="配置版本" value={policy?.version} />
            <StatusLine label="配置有效期" value={policy?.expiresAt} />
          </>
        )}
      </Stack>
    </EnhancedCard>
  )
}

const AllowedResourcesList = ({
  resources,
}: {
  resources: { type: string; value: string }[]
}) => (
  <Box sx={{ pt: 0.5 }}>
    <Typography
      variant="caption"
      color="text.secondary"
      sx={{ display: 'block', mb: 0.5 }}
    >
      可访问网址
    </Typography>
    {resources.length > 0 ? (
      <Stack
        spacing={0.5}
        sx={{
          maxHeight: 180,
          overflow: 'auto',
          pr: 0.5,
        }}
      >
        {resources.map((resource) => (
          <Box
            key={`${resource.type}:${resource.value}`}
            sx={{
              display: 'flex',
              alignItems: 'center',
              gap: 1,
              minWidth: 0,
            }}
          >
            <Typography
              variant="caption"
              color="text.secondary"
              sx={{ flex: '0 0 52px' }}
            >
              {formatResourceType(resource.type)}
            </Typography>
            <Typography
              variant="body2"
              title={formatResourceValue(resource)}
              sx={{
                minWidth: 0,
                overflow: 'hidden',
                textOverflow: 'ellipsis',
                whiteSpace: 'nowrap',
              }}
            >
              {formatResourceValue(resource)}
            </Typography>
          </Box>
        ))}
      </Stack>
    ) : (
      <Typography variant="body2">暂无可访问网址</Typography>
    )}
  </Box>
)

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

function formatResourceType(type: string) {
  if (type === 'domain') return '域名'
  if (type === 'domain_suffix') return '后缀'
  if (type === 'ip_cidr') return '网段'
  return '规则'
}

function formatResourceValue(resource: { type: string; value: string }) {
  if (resource.type !== 'domain_suffix') {
    return resource.value
  }
  const suffix = resource.value.replace(/^\./, '')
  return `*.${suffix}`
}
