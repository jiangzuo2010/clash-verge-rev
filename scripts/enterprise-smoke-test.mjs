import { spawn } from 'node:child_process'

const baseUrl = 'http://127.0.0.1:18080'
const appCode = 'company-proxy-desktop'
const accessToken = 'mock-access-token'

const server = spawn(process.execPath, ['scripts/enterprise-mock-server.mjs'], {
  cwd: process.cwd(),
  stdio: ['ignore', 'pipe', 'pipe'],
})

let output = ''
server.stdout.on('data', (chunk) => {
  output += chunk.toString()
})
server.stderr.on('data', (chunk) => {
  output += chunk.toString()
})

try {
  await waitForServer()
  await assertTokenEndpoint()
  await assertIamUser()
  await assertAuthorizedApps()
  await assertPolicy()
  console.log('Enterprise smoke test passed.')
} finally {
  server.kill()
}

async function waitForServer() {
  const deadline = Date.now() + 5_000
  while (Date.now() < deadline) {
    try {
      const response = await fetch(`${baseUrl}/`)
      if (response.ok) return
    } catch {
      await delay(100)
    }
  }
  throw new Error(`Enterprise mock server did not start.\n${output}`)
}

async function assertTokenEndpoint() {
  const body = new URLSearchParams({
    grant_type: 'authorization_code',
    code: 'mock-auth-code',
    client_id: appCode,
    redirect_uri: 'http://127.0.0.1:33221/auth/callback',
    code_verifier: 'smoke-test',
  })
  const json = await requestJson(
    '/realms/staff/protocol/openid-connect/token',
    {
      method: 'POST',
      headers: { 'Content-Type': 'application/x-www-form-urlencoded' },
      body,
    },
  )

  assert(
    json.access_token === accessToken,
    'token endpoint should return mock access token',
  )
  assert(
    json.refresh_token === 'mock-refresh-token',
    'token endpoint should return mock refresh token',
  )
}

async function assertIamUser() {
  const json = await requestJson('/iam/user/me', authHeaders())
  assert(json.code === '000000', 'IAM user response should use success code')
  assert(
    json.data?.username === 'enterprise.demo',
    'IAM user response should include username',
  )
  assert(
    json.data?.tenantCode === 'demo',
    'IAM user response should include tenant',
  )
}

async function assertAuthorizedApps() {
  const json = await requestJson('/iam/user/me/apps', authHeaders())
  assert(json.code === '000000', 'IAM apps response should use success code')
  assert(Array.isArray(json.data), 'IAM apps response data should be a list')
  assert(
    json.data.includes(appCode),
    'IAM apps response should authorize enterprise app code',
  )
}

async function assertPolicy() {
  const json = await requestJson('/enterprise/proxy/policy', authHeaders())
  assert(
    json.mode === 'managed-allowlist',
    'policy mode should be managed-allowlist',
  )
  assert(
    json.proxy?.server === '127.0.0.1',
    'policy should include mock proxy server',
  )
  assert(
    Array.isArray(json.allowlist) && json.allowlist.length > 0,
    'policy should include allowlist',
  )
}

function authHeaders() {
  return {
    headers: {
      Authorization: `Bearer ${accessToken}`,
      'X-App-Code': appCode,
    },
  }
}

async function requestJson(path, options = {}) {
  const response = await fetch(`${baseUrl}${path}`, options)
  const text = await response.text()
  if (!response.ok) {
    throw new Error(`${path} failed with ${response.status}: ${text}`)
  }
  return JSON.parse(text)
}

function assert(condition, message) {
  if (!condition) throw new Error(message)
}

function delay(ms) {
  return new Promise((resolve) => {
    setTimeout(resolve, ms)
  })
}
