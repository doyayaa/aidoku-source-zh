# Aidoku中文漫画源

Aidoku 中文漫画图源集合，每个图源一个独立目录（`sources/{id}/`）。

## 如何使用

点击 [这里](https://aidoku.app/add-source-list/?url=https://raw.githubusercontent.com/doyayaa/aidoku-source-zh/gh-pages/index.min.json) 将图源添加到 Aidoku 中。

如果添加不了，可以点击 [这里](https://aidoku.app/add-source-list/?url=https://cdn.jsdelivr.net/gh/doyayaa/aidoku-source-zh@gh-pages/index.min.json) 试试。

图源更新时，在 Aidoku 中刷新图源列表即可升级到最新版本。

## 图源列表

| 图源 | 版本 | 说明 |
|------|------|------|
| [嬉皮漫画](https://m.hipmh.com) | v8 | 基于站点 `/v1` JSON API；分类、搜索、章节、阅读均可用 |
| [嗶哩漫畫](https://www.bilimanga.net) | v4 | HTML 解析；分类/浏览、章节、阅读可用；搜索受站点 Cloudflare 限制 |

> **嗶哩漫畫搜索说明**：站点已将搜索改为前端 JS + Cloudflare 保护，非浏览器请求直接返回空页。图源内搜索可能显示无结果，是否可用取决于网络环境（家用 IP / Aidoku 网络栈可能通过）。

## 从源码构建

需要 **Rust nightly**（edition 2024）与 `wasm32-unknown-unknown` 目标：

```bash
rustup target add wasm32-unknown-unknown
./build.sh          # Windows PowerShell 使用 ./build.ps1
```

`build.sh` 会遍历 `sources/*/` 依次构建并打包每个图源，产物为各目录下的 `package.aix`（由 `pack.py` 打包，zip 条目必须是正向斜杠的 `Payload/*`，否则 iOS 上解压失败）。按版本号把产物复制到 `public/sources/{id}-v{n}.aix`、图标到 `public/icons/{id}-v{n}.png`，并更新 `public/index.json`。将 `public/` 目录部署到 `gh-pages` 分支后，即可通过上方的安装链接分发。
