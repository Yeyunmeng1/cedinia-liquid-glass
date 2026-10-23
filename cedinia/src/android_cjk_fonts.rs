//! Android 上的 CJK 字形修复。
//!
//! 背景：Slint 1.17 的 `backend-android-activity-06` 不会加载系统字体，
//! 内置字体只覆盖 Latin / Cyrillic / Greek，导致中日韩字符全部落到 `.notdef`
//! 渲染成豆腐块（upstream issues: #2108 / #2083 / #1987）。
//!
//! Slint 1.17 已从 fontdb 切到 fontique。这里用 fontique 的 script fallback 机制，
//! 把系统自带的 CJK 字体挂到 Han / Hangul / Kana 脚本的 fallback 链上。
//!
//! 需要 feature: `unstable-fontique-010`（见 Cargo.toml）。

#![cfg(target_os = "android")]

use slint::fontique_010::fontique;

/// Android / AOSP / 国产 ROM 上常见的中文字体文件，按优先级排列。
/// 只读存在的那个，不遍历目录 —— 全量扫描 /system/fonts 会让启动多花 1–2 秒。
const CANDIDATES: &[&str] = &[
    "/system/fonts/NotoSansCJK-Regular.ttc",
    "/system/fonts/NotoSansCJKsc-Regular.otf",
    "/system/fonts/NotoSansSC-Regular.otf",
    "/system/fonts/MiSans-Regular.ttf",           // 小米 HyperOS
    "/system/fonts/MiSansVF.ttf",
    "/system/fonts/HarmonyOS_Sans_SC_Regular.ttf", // 鸿蒙
    "/system/fonts/DroidSansFallback.ttf",         // 老设备兜底
];

/// Hani=汉字  Hans=简体  Hant=繁体  Hang=韩文  Hira/Kana=日文假名
const SCRIPTS: &[&str] = &["Hani", "Hans", "Hant", "Hang", "Hira", "Kana"];

/// 在 `slint::android::init()` 之后、创建窗口之前调用。
pub fn install() {
    let mut collection = slint::fontique_010::shared_collection();
    let mut installed = 0usize;

    for path in CANDIDATES {
        if !std::path::Path::new(path).exists() {
            continue;
        }
        let Ok(bytes) = std::fs::read(path) else {
            log::warn!("cjk_fonts: cannot read {path}");
            continue;
        };

        let blob = fontique::Blob::new(std::sync::Arc::new(bytes));
        let fonts = collection.register_fonts(blob, None);
        if fonts.is_empty() {
            log::warn!("cjk_fonts: no face registered from {path}");
            continue;
        }

        for &script in SCRIPTS {
            let key = fontique::FallbackKey::new(
                fontique::Script::from_str_unchecked(script),
                None,
            );
            collection.append_fallbacks(key, fonts.iter().map(|x| x.0));
        }

        installed += 1;
        log::info!("cjk_fonts: installed {path} ({} face(s))", fonts.len());

        // 第一个能用的就够覆盖全部 CJK 脚本，后面的不再重复注册（省内存）
        break;
    }

    if installed == 0 {
        log::error!("cjk_fonts: no usable CJK font found — UI will fall back to tofu");
    } else {
        log::info!("cjk_fonts: ready");
    }
}
