# 嬉皮漫画图源

Aidoku 中文漫画图源，基于 [m.hipmh.com](https://m.hipmh.com)。

## 如何使用

点击 [这里](https://aidoku.app/add-source-list/?url=https://raw.githubusercontent.com/doyayaa/aidoku-source-zh-hipmh/gh-pages/index.min.json) 将图源添加到 Aidoku 中。

如果添加不了，可以点击 [这里](https://aidoku.app/add-source-list/?url=https://cdn.jsdelivr.net/gh/doyayaa/aidoku-source-zh-hipmh@gh-pages/index.min.json) 试试。

图源更新时，在 Aidoku 中刷新图源列表即可升级到最新版本。

## 图源列表

- [嬉皮漫画](https://m.hipmh.com)

## 从源码构建

需要 **Rust nightly**（edition 2024）与 `wasm32-unknown-unknown` 目标：

```bash
rustup target add wasm32-unknown-unknown
./build.sh          # Windows PowerShell 使用 ./build.ps1
```

产物 `package.aix` 由 `pack.py` 打包（zip 条目必须是正向斜杠的 `Payload/*`，否则 iOS 上解压失败）。将 `public/` 目录部署到 `gh-pages` 分支后，即可通过上方的安装链接分发。
