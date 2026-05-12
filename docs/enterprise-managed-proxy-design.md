# 企业受管代理改造设计

## 目标

将当前个人版代理客户端改造成企业受管客户端。员工安装后必须登录公司 IAM，客户端根据服务端下发的 JSON 策略生成代理配置。公司代理只服务白名单资源，非白名单流量走本机默认网络，不能借公司代理转发。

## 关键原则

- 登录前不可使用代理能力。
- 策略由服务端统一下发，本地只做缓存和校验。
- 白名单命中走公司代理，未命中走 `DIRECT`。
- 员工不能通过自定义订阅、脚本、外部控制器绕过企业策略。
- 登录失效、刷新失败、策略过期时关闭系统代理。
- 敏感 token 不写明文配置文件。

## IAM 对接

`platform-iam` 文档明确 Keycloak 负责认证，IAM 负责用户、租户、应用、权限等业务身份数据。新客户端应使用 Keycloak OIDC，不使用 `/iam/auth/login` 作为主登录入口。

推荐登录方式：

1. 桌面客户端启动一次性本地回调监听。
2. 打开 Keycloak 授权页，使用 Authorization Code + PKCE。
3. 回调后用 code 换取 `access_token` 与 `refresh_token`。
4. 用 `Authorization: Bearer <access_token>` 调用 IAM 用户态接口。
5. token 临近过期时用 refresh token 刷新。
6. refresh 失败时登出并关闭系统代理。

建议 Keycloak client：

```text
realm: staff
client_id: company-proxy-desktop
redirect_uri: http://127.0.0.1:<random-port>/auth/callback
```

当前实现默认使用：

```text
redirect_uri: http://127.0.0.1:33221/auth/callback
```

客户端会校验回调地址必须是 `http://localhost` 或 `http://127.0.0.1`，且必须包含端口。登录完成后会调用 `/iam/user/me`，把 `id`、`username`、`tenantId`、`tenantCode` 写入本地 session 视图；token 和 PKCE verifier 使用项目现有加密序列化保存。

IAM 校验接口：

```text
GET /iam/user/me
GET /iam/user/me/apps
GET /iam/user/permissions
```

客户端应检查当前用户是否被授权使用企业代理应用，例如 `company-proxy-desktop` 或后续确定的应用编码。

当前实现会在登录完成和 token 刷新后调用：

```text
GET /iam/user/me/apps
```

如果返回的应用列表不包含当前 `appCode`，客户端会清理 session 与策略缓存，后续运行时进入 fail closed。

退出企业会话或禁用企业模式时，客户端会 best-effort 调用 Keycloak logout endpoint 撤销 refresh token；即使远端撤销失败，本地仍会清理 session 与策略缓存并关闭企业运行时。

## 服务端策略 JSON

建议策略接口由企业代理服务提供，不直接放在 IAM 中。IAM 只提供身份与授权上下文。

请求：

```text
GET /enterprise/proxy/policy
Authorization: Bearer <access_token>
```

响应示例：

```json
{
  "version": "2026.05.11.1",
  "mode": "managed-allowlist",
  "expiresAt": "2026-05-11T18:00:00+08:00",
  "refreshAfterSeconds": 600,
  "proxy": {
    "name": "company-proxy",
    "type": "http",
    "server": "proxy.company.example",
    "port": 443,
    "tls": true
  },
  "allowlist": [
    {
      "type": "domain",
      "value": "docs.company.example"
    },
    {
      "type": "domain_suffix",
      "value": ".corp.company.example"
    },
    {
      "type": "ip_cidr",
      "value": "10.10.0.0/16"
    }
  ]
}
```

字段约束：

- `version`：策略版本，用于审计与缓存命中。
- `expiresAt`：超过该时间策略不可继续信任。
- `refreshAfterSeconds`：客户端下一次刷新建议。
- `proxy`：公司代理出口配置。
- `allowlist`：允许经公司代理访问的资源集合，当前支持 `domain`、`domain_suffix`、`ip_cidr`。

当前策略是域名/IP 级白名单，不支持路径级 URL 强约束。原因是 mihomo 规则无法按 HTTP path 做通用路由，HTTPS path 对本地代理也不可见；如果后续必须支持 `https://host/path` 粒度，需要引入 PAC 作为浏览器路径选择层，或实现专用本地 HTTP 代理做请求级校验。不能只把 path 写进 mihomo 规则，否则会形成虚假的安全边界。

## mihomo 规则生成

企业策略转换为运行时配置时，生成固定代理组和白名单规则：

```yaml
proxies:
  - name: company-proxy
    type: http
    server: proxy.company.example
    port: 443
    tls: true

proxy-groups:
  - name: COMPANY-PROXY
    type: select
    proxies:
      - company-proxy

rules:
  - DOMAIN,docs.company.example,COMPANY-PROXY
  - DOMAIN-SUFFIX,corp.company.example,COMPANY-PROXY
  - IP-CIDR,10.10.0.0/16,COMPANY-PROXY,no-resolve
  - MATCH,DIRECT
```

默认规则必须是 `MATCH,DIRECT`，不能是公司代理。

## 客户端模块划分

Rust 后端新增模块：

```text
src-tauri/src/enterprise/
  auth.rs      OIDC/PKCE 登录、本地回调监听
  session.rs   access token 刷新、IAM 当前用户信息同步
  policy.rs    策略模型与校验
  sync.rs      策略拉取与缓存
  config.rs    企业策略到 mihomo Mapping 的转换
  state.rs     登录态与策略状态
```

前端新增模块：

```text
src/services/enterprise.ts
src/hooks/use-enterprise.ts
src/components/setting/setting-enterprise.tsx
```

Tauri commands：

```text
get_enterprise_state
patch_enterprise_config
clear_enterprise_session
start_enterprise_login
complete_enterprise_login
sync_enterprise_policy
```

运行时集成：

- `Config::generate()` 在企业模式启用时要求已登录且已有缓存策略。
- 企业策略会强制替换 `mode`、`proxies`、`proxy-groups`、`rules`，并移除 `proxy-providers`、`rule-providers`。
- 应用启动时如果企业模式已登录，会后台尝试刷新 token、同步策略并应用运行时配置；失败只记录日志，不阻断启动。
- 设置页登录成功后会立即同步策略并应用配置。
- 后台任务会持续按策略 `refreshAfterSeconds` 刷新策略，客户端会把刷新间隔限制在 60 秒到 1 小时之间，未登录时使用默认 300 秒低频检查。
- 企业模式启用后，前端隐藏 Profiles 和 Unlock 页面；直接访问这些路由会跳转到 Settings。
- 企业模式启用后，后端会拒绝 profile 写入/切换、DNS 覆盖、运行时代理链修改、Clash 配置与模式修改等可能绕过受管策略的 IPC 命令。
- 企业模式启用但未登录时，前端只保留 Settings，用于完成登录和企业配置；登录后再开放 Home、Proxies、Connections、Rules、Logs 等非策略编辑页面。
- 企业模式启用后，Settings 会隐藏 Clash 配置区和高级 Verge 配置区；后端同时拒绝外部控制器、系统代理、TUN、端口、bypass、DNS 等代理边界相关的 Verge 配置写入。
- 企业配置变更、退出登录、登录完成、策略同步等企业 IPC 命令会自行应用运行时状态：无有效企业会话/策略时重置系统代理并停止 core；禁用企业模式时恢复个人配置；策略有效时应用受管配置。
- 后台同步失败时也会重新评估本地缓存策略；缓存仍有效则继续应用，缓存过期或无效则重置系统代理并停止 core，避免过期策略继续运行。
- 企业模式下启动或重启 core 前会先检查本地企业会话和缓存策略；无有效会话/策略时拒绝启动并保持 fail closed。换核属于受管运行边界，企业模式下禁止。
- 企业模式下托盘、快捷键、deep link、订阅更新、备份恢复等非页面入口也会被企业模式守卫拦截，避免绕过受管策略。
- 企业模式下系统代理由运行态接管：有有效会话和策略时自动指向本机 mihomo 入口，无有效会话/策略时强制关闭；个人配置中的 PAC、proxy host、系统代理开关不会在企业模式下生效。

## 本地联调

可以启动本地 mock 服务验证登录、IAM 用户信息、授权应用和策略拉取：

```bash
pnpm enterprise:mock
```

也可以运行自动冒烟测试，它会临时启动 mock 服务并验证 token、IAM 用户、应用授权和策略接口：

```bash
pnpm enterprise:smoke
```

默认监听 `http://127.0.0.1:18080`，模拟：

```text
GET  /realms/staff/protocol/openid-connect/auth
POST /realms/staff/protocol/openid-connect/token
POST /realms/staff/protocol/openid-connect/logout
GET  /iam/user/me
GET  /iam/user/me/apps
GET  /enterprise/proxy/policy
```

桌面端企业配置可设置为：

```text
IAM Base URL: http://127.0.0.1:18080
Policy Base URL: http://127.0.0.1:18080
Keycloak Base URL: http://127.0.0.1:18080
Keycloak Realm: staff
Client ID: company-proxy-desktop
Redirect URI: http://127.0.0.1:33221/auth/callback
App Code: company-proxy-desktop
```

设置页提供“本地 Mock”按钮，可一键写入以上配置并启用企业模式。随后点击“打开”会进入 mock OIDC 流程，回调完成后客户端会同步策略并应用受管配置。

mock 策略中的公司代理地址默认是 `127.0.0.1:7890`，只用于验证客户端策略转换和运行态应用；真实网络连通性需要换成可访问的企业代理。

## UI 改造

登录前：

- 只展示登录页。
- 不启动系统代理。
- 不展示主功能页面。

登录后：

- 首页展示当前用户、租户、策略版本、同步状态、代理状态；企业模式下用企业状态卡替代个人 Profile 卡，并隐藏网络开关卡。
- 隐藏或锁定 Profiles、脚本增强、外部控制器、自定义订阅等入口。
- 保留必要的诊断、日志、连接状态页面。

## 失效处理

| 场景 | 处理 |
|---|---|
| access token 过期 | 使用 refresh token 刷新 |
| refresh token 失效 | 登出、关闭系统代理、清理运行配置 |
| 策略拉取失败但缓存未过期 | 继续使用缓存，提示同步失败 |
| 策略缓存过期 | 关闭系统代理，禁止继续使用公司代理 |
| 用户未授权当前应用 | 登出或停留在无权限页，关闭系统代理 |
| 服务端策略非法 | 拒绝应用新策略，保留未过期旧策略或关闭代理 |

## 与现有代码的集成点

- 配置生成入口：[src-tauri/src/config/config.rs](/Users/zorro/Documents/Projects/clash-verge-rev/src-tauri/src/config/config.rs)
- 系统代理应用入口：[src-tauri/src/core/sysopt.rs](/Users/zorro/Documents/Projects/clash-verge-rev/src-tauri/src/core/sysopt.rs)
- 企业运行态入口：[src-tauri/src/enterprise/runtime.rs](/Users/zorro/Documents/Projects/clash-verge-rev/src-tauri/src/enterprise/runtime.rs)
- 应用启动初始化：[src-tauri/src/lib.rs](/Users/zorro/Documents/Projects/clash-verge-rev/src-tauri/src/lib.rs)
- 前端路由入口：[src/pages/_layout.tsx](/Users/zorro/Documents/Projects/clash-verge-rev/src/pages/_layout.tsx)
- 前端应用入口：[src/main.tsx](/Users/zorro/Documents/Projects/clash-verge-rev/src/main.tsx)

## 实施状态

已完成：

1. 企业配置与策略模型，不影响个人模式默认行为。
2. OIDC Authorization Code + PKCE 登录、本地回调和加密会话存储。
3. IAM `/iam/user/me` 与 `/iam/user/me/apps` 校验。
4. 策略拉取、服务端 Result 包装兼容、校验、缓存与定时刷新。
5. 企业策略到 mihomo 运行配置的转换。
6. 登录守卫、受管 UI、首页企业状态卡。
7. profile、订阅、Clash 配置、系统代理、TUN、DNS、托盘、deep link、备份恢复等绕过入口锁定。
8. 企业运行态 fail closed、有效策略自动接管系统代理、禁用企业模式恢复个人代理设置。
9. 本地 mock 服务和策略转换、登录、会话、缓存、运行态决策等单元测试。

## 外部接入参数

- 企业代理服务的正式 base URL。
- Keycloak 桌面客户端的 `client_id`、redirect URI、是否 public client。
- 企业代理应用编码。
