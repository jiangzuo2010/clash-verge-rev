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
import { revalidateQuery, setCacheData, useQuery } from '@/services/query-client'

const ENTERPRISE_STATE_QUERY_KEY = ['getEnterpriseState'] as const

export const useEnterprise = () => {
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
        void revalidateQuery(ENTERPRISE_STATE_QUERY_KEY)
        return
      }

      setCacheData<EnterpriseStateView>(
        ENTERPRISE_STATE_QUERY_KEY,
        updaterOrData,
      )
    },
    [],
  )

  const patchEnterprise = useCallback(
    async (patch: EnterpriseConfigPatch) => {
      const state = await patchEnterpriseConfig(patch)
      setCacheData(ENTERPRISE_STATE_QUERY_KEY, state)
      return state
    },
    [],
  )

  const clearSession = useCallback(async () => {
    const state = await clearEnterpriseSession()
    setCacheData(ENTERPRISE_STATE_QUERY_KEY, state)
    return state
  }, [])

  const startLogin = useCallback((openBrowser = true) => {
    return startEnterpriseLogin(openBrowser)
  }, [])

  const completeLogin = useCallback(
    async (request: EnterpriseAuthCodeRequest) => {
      const state = await completeEnterpriseLogin(request)
      setCacheData(ENTERPRISE_STATE_QUERY_KEY, state)
      return state
    },
    [],
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
