import http from 'node:http'
import { URL } from 'node:url'

const host = process.env.ENTERPRISE_MOCK_HOST ?? '127.0.0.1'
const port = Number(process.env.ENTERPRISE_MOCK_PORT ?? '18080')
const appCode = process.env.ENTERPRISE_MOCK_APP_CODE ?? 'company-proxy-desktop'
const realm = process.env.ENTERPRISE_MOCK_REALM ?? 'staff'

const json = (response, status, body) => {
  response.writeHead(status, {
    'Access-Control-Allow-Origin': '*',
    'Access-Control-Allow-Headers':
      'Authorization,Content-Type,Accept,X-App-Code',
    'Access-Control-Allow-Methods': 'GET,POST,OPTIONS',
    'Content-Type': 'application/json',
  })
  response.end(JSON.stringify(body))
}

const html = (response, status, body) => {
  response.writeHead(status, { 'Content-Type': 'text/html;charset=utf-8' })
  response.end(body)
}

const readBody = (request) =>
  new Promise((resolve, reject) => {
    const chunks = []
    request.on('data', (chunk) => chunks.push(chunk))
    request.on('end', () => resolve(Buffer.concat(chunks).toString('utf8')))
    request.on('error', reject)
  })

const tokenResponse = () => ({
  access_token: 'mock-access-token',
  refresh_token: 'mock-refresh-token',
  expires_in: 3600,
  token_type: 'Bearer',
})

const policyResponse = () => ({
  version: new Date().toISOString().slice(0, 10).replaceAll('-', '.') + '.dev',
  mode: 'managed-allowlist',
  expiresAt: new Date(Date.now() + 60 * 60 * 1000).toISOString(),
  refreshAfterSeconds: 300,
  proxy: {
    name: 'company-proxy',
    type: 'http',
    server: '127.0.0.1',
    port: 7890,
    tls: false,
  },
  allowlist: [
    { type: 'domain', value: 'docs.company.example' },
    { type: 'domain_suffix', value: '.corp.company.example' },
    { type: 'ip_cidr', value: '10.10.0.0/16' },
  ],
})

const result = (data) => ({
  code: '000000',
  message: 'success',
  data,
})

const server = http.createServer(async (request, response) => {
  try {
    if (request.method === 'OPTIONS') {
      json(response, 204, {})
      return
    }

    const url = new URL(request.url ?? '/', `http://${host}:${port}`)
    const path = url.pathname

    if (path === `/realms/${realm}/protocol/openid-connect/auth`) {
      const redirectUri = url.searchParams.get('redirect_uri')
      const state = url.searchParams.get('state') ?? ''
      if (!redirectUri) {
        json(response, 400, { error: 'missing redirect_uri' })
        return
      }

      const redirect = new URL(redirectUri)
      redirect.searchParams.set('code', 'mock-auth-code')
      redirect.searchParams.set('state', state)
      response.writeHead(302, { Location: redirect.toString() })
      response.end()
      return
    }

    if (path === `/realms/${realm}/protocol/openid-connect/token`) {
      await readBody(request)
      json(response, 200, tokenResponse())
      return
    }

    if (path === `/realms/${realm}/protocol/openid-connect/logout`) {
      await readBody(request)
      response.writeHead(204)
      response.end()
      return
    }

    if (path === '/iam/user/me') {
      json(
        response,
        200,
        result({
          id: 7,
          username: 'enterprise.demo',
          realName: 'Enterprise Demo',
          email: 'enterprise.demo@example.com',
          tenantId: 3,
          tenantCode: 'demo',
        }),
      )
      return
    }

    if (path === '/iam/user/me/apps') {
      json(response, 200, result([appCode]))
      return
    }

    if (path === '/enterprise/proxy/policy') {
      const requestAppCode = request.headers['x-app-code']
      if (requestAppCode !== appCode) {
        json(response, 403, {
          code: 'FORBIDDEN',
          message: `unexpected X-App-Code: ${requestAppCode ?? ''}`,
        })
        return
      }

      json(response, 200, policyResponse())
      return
    }

    if (path === '/') {
      html(
        response,
        200,
        `<p>Enterprise mock server is running.</p><p>Base URL: http://${host}:${port}</p>`,
      )
      return
    }

    json(response, 404, { error: `not found: ${path}` })
  } catch (error) {
    json(response, 500, {
      error: error instanceof Error ? error.message : String(error),
    })
  }
})

server.listen(port, host, () => {
  console.log(`Enterprise mock server listening on http://${host}:${port}`)
  console.log(`Realm: ${realm}`)
  console.log(`App code: ${appCode}`)
})
