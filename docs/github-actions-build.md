# GitHub Actions 出包指南

本仓库使用 `.github/workflows/build-installers.yml` 构建 macOS 与 Windows 安装包。

## 产物

| 架构 | 产物 | 适用机器 |
| --- | --- | --- |
| `aarch64-apple-darwin` | `ChinEuro Secure Access_<版本>_aarch64.dmg` | M1/M2/M3/M4 Mac |
| `x86_64-apple-darwin` | `ChinEuro Secure Access_<版本>_x64.dmg` | Intel Mac |
| `x86_64-pc-windows-msvc` | `ChinEuro Secure Access_<版本>_x64-setup.exe` | 绝大多数 Windows PC |
| `aarch64-pc-windows-msvc` | `ChinEuro Secure Access_<版本>_arm64-setup.exe` | 骁龙 X / Surface Pro X |

配置了自更新签名密钥时，还会附带 `.app.tar.gz` / `.sig` 等自更新产物。

## 两种触发方式

**手动触发**（日常出测试包）
Actions → Build Installers → Run workflow。可选择只构建 `macos` 或 `windows`，未选中的架构不会占用 runner。勾选「同时创建一个草稿 Release」可把产物汇总成一个草稿 Release，否则只放在 workflow artifacts 里（保留 30 天）。

**推 tag**（正式发版）

```bash
# tag 必须与 package.json 的 version 一致
git tag v2.5.3
git push origin v2.5.3
```

四个架构全部构建，产物汇总为**草稿** Release。确认无误后在 GitHub 上点 Publish 才会对外可见。tag 名含 `-rc` 时标记为预发布。

## 需要配置的 Secrets

在 `Settings → Secrets and variables → Actions` 添加。三个都是可选的，但**强烈建议至少配置前两个**。

### `ENTERPRISE_POLICY_CRYPTO_SECRET`

企业策略传输加密密钥，由 `src-tauri/build.rs` 在编译期注入二进制。

**不配置的后果**：回落到 `src-tauri/src/enterprise/sync.rs` 中硬编码的默认值 `chineuro-enterprise-policy-transport-secret-v1`。该值在代码库中公开可见，任何人都能解密策略下发内容。**仅可用于测试包，正式分发必须配置。**

值需要与你的策略服务端保持一致。

### `TAURI_PRIVATE_KEY` 与 `TAURI_KEY_PASSWORD`

应用自更新的签名私钥及其密码。`src-tauri/tauri.conf.json` 中已经配置了对应的公钥：

```
plugins.updater.pubkey = dW50cnVzdGVkIGNvbW1lbnQ6...
plugins.updater.endpoints = https://updates.chineuro.com/chineuro-proxy/...
```

私钥应该是你当初生成这对密钥时留下的那一份。**必须与上述 pubkey 配对**，否则装出来的应用在检查更新时会拒绝更新包。

**不配置的后果**：workflow 会自动关闭 `createUpdaterArtifacts` 继续出包（否则 tauri 会因为「有公钥但没私钥」直接构建失败），但装出来的应用**不支持应用内自动更新**，只能手动下载新版覆盖安装。

如果私钥已丢失，需要重新生成一对并更新 `tauri.conf.json` 里的 pubkey——注意这会导致**已经装在用户机器上的旧版本无法再自动更新**，必须手动分发一次。

```bash
pnpm tauri signer generate -w ~/.tauri/chineuro.key
```

## 代码签名现状

当前**未配置代码签名**，用户首次运行会遇到系统拦截：

- **macOS**：使用 ad-hoc 签名（`signingIdentity: "-"`）。用户首次打开需右键 →「打开」，或在「系统设置 → 隐私与安全性」中点「仍要打开」。
- **Windows**：NSIS 安装程序未签名，会弹出 SmartScreen「Windows 已保护你的电脑」，需点「更多信息」→「仍要运行」。

如果需要消除这些提示：

- macOS 需要 Apple 开发者账号（99 美元/年），在 workflow 的构建步骤补上 `APPLE_CERTIFICATE`、`APPLE_CERTIFICATE_PASSWORD`、`APPLE_SIGNING_IDENTITY`、`APPLE_ID`、`APPLE_PASSWORD`、`APPLE_TEAM_ID` 六个环境变量即可自动签名并公证。
- Windows 需要代码签名证书（OV 约 200 美元/年起，EV 更贵但能立即消除 SmartScreen 警告），在 `tauri.windows.conf.json` 的 `bundle.windows.certificateThumbprint` 配置。

上游的 `.github/workflows/release.yml` 里有完整的签名步骤可以参考。

## 构建耗时

单个架构大约 15–30 分钟，四个架构并行执行。Rust 依赖有缓存，首次构建最慢，后续会快不少。

## 与上游 workflow 的关系

为避免重复构建和无谓的失败通知，本 fork 调整了两个上游 workflow 的触发方式（内容未改，仍可手动运行）：

| workflow | 上游行为 | 本 fork | 原因 |
| --- | --- | --- | --- |
| `release.yml` | 推 `v*.*.*` tag 自动触发 | 改为仅手动 | 会与 `build-installers.yml` 重复构建；且它强制要求 tag 来自 `main` 分支，而本仓库主干是 `dev`，必然失败 |
| `autobuild.yml` | 每天 UTC 4:00 / 10:00 定时构建 | 关闭定时，改为仅手动 | 它 `uses:` 的是**上游仓库**的 reusable workflow 并依赖上游 secrets，在 fork 中定时运行只会失败并消耗 Actions 额度 |

下次合并上游时，这两处触发配置可能产生冲突，保留本 fork 的版本即可。

## 排错

**`pnpm install --frozen-lockfile` 失败**
`pnpm-lock.yaml` 与 `package.json` 不同步。本地跑一次 `pnpm install` 并提交更新后的 lock 文件。

**prebuild 下载内核超时**
`scripts/prebuild.mjs` 会从 GitHub Releases 下载 mihomo 内核、服务端二进制和 GeoIP 数据。偶发网络失败，重跑 workflow 即可（脚本自带重试）。

**`A public key has been found, but no private key`**
配了 `TAURI_PRIVATE_KEY` 但值为空或格式不对。私钥文件内容要**整个**贴进 secret（含 `untrusted comment:` 开头那行）。

**Actions 版本报错（如 `Unable to resolve action actions/upload-artifact@v7`）**
workflow 里的 action 版本对齐的是上游 `release.yml` 当前在用的版本。若你的环境不可用，把版本号降到 `@v4` 即可，功能没有差别。
