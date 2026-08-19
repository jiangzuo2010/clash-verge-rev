import { Box, Stack, Typography } from '@mui/material'
import { useTranslation } from 'react-i18next'

export const EnterpriseStatusLine = ({
  label,
  value,
}: {
  label: string
  value?: string
}) => (
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

export const EnterpriseAllowedResources = ({
  resources,
  emptyText,
}: {
  resources: { type: string; value: string }[]
  emptyText?: string
}) => {
  const { t } = useTranslation()

  return (
    <Box sx={{ pt: 0.5 }}>
      <Typography
        variant="caption"
        color="text.secondary"
        sx={{ display: 'block', mb: 0.75 }}
      >
        {t('home.enterprise.resources.title')}
      </Typography>
      {resources.length > 0 ? (
        <Stack
          spacing={0.75}
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
                {formatResourceType(resource.type, t)}
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
        <Typography variant="body2" color="text.secondary">
          {emptyText ?? t('home.enterprise.resources.empty')}
        </Typography>
      )}
    </Box>
  )
}

function formatResourceType(type: string, t: (key: string) => string) {
  if (type === 'domain') return t('home.enterprise.resources.types.domain')
  if (type === 'domain_suffix') {
    return t('home.enterprise.resources.types.domainSuffix')
  }
  if (type === 'ip_cidr') return t('home.enterprise.resources.types.ipCidr')
  return t('home.enterprise.resources.types.rule')
}

function formatResourceValue(resource: { type: string; value: string }) {
  if (resource.type !== 'domain_suffix') {
    return resource.value
  }
  return `*.${resource.value.replace(/^\./, '')}`
}
