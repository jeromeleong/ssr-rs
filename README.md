# 🚀 為 JS 前端和 Rust 後端提供服務端渲染的橋樑 - SSR-RS

<p align="center">
  <img src="https://raw.githubusercontent.com/jeromeleong/ssr-rs/main/logo.png" alt="SSR Rust Logo">
</p>

本 crate 是基於 [Valerioageno的ssr-rs](https://github.com/Valerioageno/ssr-rs) 來進行部分修改，旨在以最簡單和最輕量的方式啟用 Rust 伺服器上的伺服器端渲染。它使用嵌入版本的 [V8](https://v8.dev/) JavaScript 引擎（<a href="https://github.com/denoland/rusty_v8" target="_blank">rusty_v8</a>）來解析和評估已建置的 bundle 文件，並返回渲染後的 HTML 字符串。

## 和[Valerioageno的ssr-rs](https://github.com/Valerioageno/ssr-rs)的分別：
- 移除原有的所有的 `unsafe`
- 增加對 ES Modules (ESM) 的支持
- 使用 LRU 緩存機制來優化 JS 加載和渲染性能
- 使用 Once 來確保 V8 平台的初始化只進行一次，避免了重複初始化的開銷

## 安裝

在你的 `Cargo.toml` 中添加以下內容：

```toml
[dependencies]
ssr_rs = { git = "https://github.com/jeromeleong/ssr-rs" }
```

## 使用示例

### 初始化 SSR 實例

```rust
use ssr_rs::Ssr;
use std::fs::read_to_string;

fn main() {
    let source = read_to_string("./path/to/build.js").unwrap();

    let ssr = Ssr::new();
    ssr.load(&source, "entryPoint", "cjs").unwrap();

    let html = ssr.render_to_string(None).unwrap();
    
    assert_eq!(html, "<!doctype html><html>...</html>".to_string());
}
```

### 帶參數渲染

```rust
use ssr_rs::Ssr;
use std::fs::read_to_string;

fn main() {
    let props = r##"{
        "params": [
            "hello",
            "ciao",
            "こんにちは"
        ]
    }"##;

    let source = read_to_string("./path/to/build.js").unwrap();

    let ssr = Ssr::new();
    ssr.load(&source, "entryPoint", "cjs").unwrap();

    let html = ssr.render_to_string(Some(props)).unwrap();

    assert_eq!(html, "<!doctype html><html>...</html>".to_string());
}
```

## 貢獻

歡迎任何形式的貢獻，包括但不限於：

- 代碼改進
- 文檔完善
- 新功能提案
- 錯誤報告

## 許可證

本項目採用 MIT 許可證。詳見 [LICENSE](https://github.com/jeromeleong/ssr-rs/blob/main/LICENSE_MIT) 文件。

## 聯繫方式

如有任何問題或建議，請通過 [GitHub Issues](https://github.com/jeromeleong/ssr-rs/issues) 與我們聯繫。
