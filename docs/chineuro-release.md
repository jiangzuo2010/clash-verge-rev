# ChinEuro Secure Access 发布说明

## 品牌与首屏

- 应用名称：`ChinEuro Secure Access`
- Bundle Identifier：`com.chineuro.enterprise-proxy`
- 默认启用企业受管模式。
- 新安装首次打开直接进入企业登录页。
- 安装图标和托盘图标使用萃欧红底白字安全盾牌 Logo；侧边栏显示 `ChinEuro Secure Access`。

## 自动更新

当前配置使用 Tauri v2 updater：

```json
{
  "bundle": {
    "createUpdaterArtifacts": true
  },
  "plugins": {
    "updater": {
      "pubkey": "<public-key-content>",
      "endpoints": [
        "https://updates.taxspace.eu/chineuro-proxy/latest.json"
      ]
    }
  }
}
```

上线前需要完成：

1. 生成 updater 签名密钥：

   ```bash
   pnpm tauri signer generate --ci -w ~/.tauri/chineuro-proxy.key
   ```

2. 将生成的公钥写入 `src-tauri/tauri.conf.json` 的 `plugins.updater.pubkey`。当前仓库已写入本机生成的公钥；对应私钥在 `~/.tauri/chineuro-proxy.key`。
3. 私钥只放 CI/CD Secret 或构建机，不提交仓库：

   ```bash
   export TAURI_SIGNING_PRIVATE_KEY_PATH="$HOME/.tauri/chineuro-proxy.key"
   export TAURI_SIGNING_PRIVATE_KEY_PASSWORD="<password-if-any>"
   ```

4. 推 `v*` tag 后，`build-installers.yml` 的 Release 里会附带 `latest.json`（静态更新清单）。发布时把该 Release 的全部文件原样上传到更新站点：

   ```text
   https://updates.taxspace.eu/chineuro-proxy/latest.json          ← 覆盖为最新版本的清单
   https://updates.taxspace.eu/chineuro-proxy/v<版本>/<安装包与 .sig>  ← 清单里的下载地址指向这里
   ```

   客户端只比较 `latest.json` 的 `version`，比自己新才提示更新；先上传 `v<版本>/` 目录，最后再覆盖 `latest.json`，避免清单先指向尚未上传的文件。

更新地址编译进安装包，换域名后已安装的旧版本找不到新版本，只能手动重装一次。

## 企业配置传输安全

企业登录、Token 刷新、当前用户、授权应用和策略配置拉取统一使用受限 HTTP 客户端：

- 生产环境 URL 必须使用 HTTPS；仅允许 `localhost` / `127.0.0.1` / `::1` 使用 HTTP 作为本地开发 Mock。
- 客户端不走系统代理或本地代理，避免被系统代理透明转发。
- TLS 只信任应用内置 WebPKI 根证书，不信任用户系统额外安装的抓包根证书。
- 禁止 HTTP 重定向，避免鉴权头或策略请求被重定向到非预期地址。
- 不允许跳过证书校验或主机名校验。

策略配置响应额外使用应用层加密：

- 服务端 `data` 返回 `encrypted=true` 的 envelope，算法为 `AES-256-GCM`，并用 `HMAC-SHA256` 签名 `keyId/issuedAt/nonce/ciphertext`。
- 客户端校验签名后解密出原始策略 JSON；非本地地址如果返回明文策略，客户端会拒绝同步。
- 客户端构建时或运行时可通过 `ENTERPRISE_POLICY_CRYPTO_SECRET` 覆盖默认传输密钥；生产构建和服务端部署必须使用同一随机密钥。

这可以防止常见抓包工具通过安装本地 CA 解密企业策略配置，也避免服务端边界代理或网关日志直接暴露策略明文。被控终端进程内存读取和客户端二进制逆向不属于本方案可完全消除的风险，后续可通过设备绑定、短期策略、密钥轮换和完整性校验继续降低风险。

## macOS 签名与公证

公司外部分发 DMG 时使用 `Developer ID Application` 证书；App Store 分发使用 `Apple Distribution`。外部分发还需要公证。

本机签名：

```bash
security find-identity -v -p codesigning
export APPLE_SIGNING_IDENTITY="Developer ID Application: <Company Name> (<TEAM_ID>)"
export APPLE_API_ISSUER="<app-store-connect-issuer-id>"
export APPLE_API_KEY="<key-id>"
export APPLE_API_KEY_PATH="/secure/path/AuthKey_<key-id>.p8"
pnpm tauri build -f tauri.macos.conf.json
```

也可以使用 `APPLE_ID`、`APPLE_PASSWORD`、`APPLE_TEAM_ID` 做公证认证。CI/CD 中建议使用 App Store Connect API key。

## Windows 签名

Windows 正式分发需要代码签名证书，否则员工从浏览器下载时容易遇到 SmartScreen 警告。EV 证书信誉建立更快；OV 证书通常需要时间积累信誉。

需要准备：

- 公司代码签名证书。
- 证书指纹 `certificateThumbprint`。
- 时间戳服务 `timestampUrl`。

配置位置：`src-tauri/tauri.windows.conf.json`

```json
{
  "bundle": {
    "windows": {
      "certificateThumbprint": "<thumbprint>",
      "digestAlgorithm": "sha256",
      "timestampUrl": "http://timestamp.digicert.com"
    }
  }
}
```

构建：

```powershell
pnpm tauri build -f tauri.windows.conf.json
```

如果证书托管在 HSM、云签名或 EV USB Key，需要按证书厂商要求配置 Tauri custom sign command。

## Linux

Linux 包名已改为 `chineuro-proxy`，生成 `deb`/`rpm`：

```bash
pnpm tauri build -f tauri.linux.conf.json
```
