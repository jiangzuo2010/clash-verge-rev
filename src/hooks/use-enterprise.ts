import { useQuery, useQueryClient } from '@tanstack/react-query'
import { useCallback } from 'react'

import {
  clearEnterpriseSession,
  completeEnterpriseLogin,
  getEnterpriseState,
  patchEnterpriseConfig,
  startEnterpriseLogin,
  type EnterpriseAuthCodeRequest,
  type EnterpriseConfigPatch,
  type EnterpriseStateView,
} from '@/services/enterprise'

const ENTERPRISE_STATE_QUERY_KEY = ['getEnterpriseState'] as const

export const useEnterprise = () => {
  const qc = useQueryClient()

  const { data: enterprise, refetch } = useQuery({
    queryKey: ENTERPRISE_STATE_QUERY_KEY,
    queryFn: getEnterpriseState,
    staleTime: 5000,
  })

  const mutateEnterprise = useCallback(
    (
      updaterOrData?:
        | EnterpriseStateView
        | ((
            prev: EnterpriseStateView | undefined,
          ) => EnterpriseStateView | undefined),
    ) => {
      if (updaterOrData === undefined) {
        void refetch()
        return
      }

      if (typeof updaterOrData === 'function') {
        const prev = qc.getQueryData<EnterpriseStateView>(
          ENTERPRISE_STATE_QUERY_KEY,
        )
        qc.setQueryData(ENTERPRISE_STATE_QUERY_KEY, updaterOrData(prev))
      } else {
        qc.setQueryData(ENTERPRISE_STATE_QUERY_KEY, updaterOrData)
      }
    },
    [qc, refetch],
  )

  const patchEnterprise = useCallback(
    async (patch: EnterpriseConfigPatch) => {
      const state = await patchEnterpriseConfig(patch)
      qc.setQueryData(ENTERPRISE_STATE_QUERY_KEY, state)
      return state
    },
    [qc],
  )

  const clearSession = useCallback(async () => {
    const state = await clearEnterpriseSession()
    qc.setQueryData(ENTERPRISE_STATE_QUERY_KEY, state)
    return state
  }, [qc])

  const startLogin = useCallback((openBrowser = true) => {
    return startEnterpriseLogin(openBrowser)
  }, [])

  const completeLogin = useCallback(
    async (request: EnterpriseAuthCodeRequest) => {
      const state = await completeEnterpriseLogin(request)
      qc.setQueryData(ENTERPRISE_STATE_QUERY_KEY, state)
      return state
    },
    [qc],
  )

  return {
    enterprise,
    mutateEnterprise,
    patchEnterprise,
    clearSession,
    startLogin,
    completeLogin,
    refetchEnterprise: refetch,
  }
}
