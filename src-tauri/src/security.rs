//! 凭证加解密 — DPAPI（用户级）加密平台令牌，替代 config.json 明文存储
//!
//! 设计要点：
//! - Windows DPAPI 加密，密钥绑定「当前 Windows 账户 + 本机」：
//!   config.json 被拷到别的电脑/账户，密文无法还原
//! - 附加应用熵（ENTROPY）：同机其他程序即使调 DPAPI 也需知道本应用的熵值，
//!   可挡住"空熵参数盲扫 DPAPI 密文"的通用窃密手法；无法防针对本应用的逆向
//! - 密文统一 `enc1:` 前缀 + base64 标记，读取按前缀识别；
//!   无前缀 = 旧版明文，兼容读取，落盘时自动升级为密文
//! - 加解密只发生在存储边界（platform.rs 存/读 store 处），业务层全程明文内存值

use base64::engine::general_purpose::STANDARD as BASE64;
use base64::Engine as _;

/// 密文前缀标记（v1：升级算法时换 enc2: 并保留旧解密兼容）
const ENC_PREFIX: &str = "enc1:";

/// 应用熵（固定常量）：防同机其他程序以空熵参数直接解密
const ENTROPY: &[u8] = b"Sensend::credentials::v1::2026";

/// 是否已是密文格式
pub fn is_encrypted(value: &str) -> bool {
    value.starts_with(ENC_PREFIX)
}

/// 加密单个凭证。空串原样返回；已是密文幂等返回。
/// [n-04 修复] 非 Windows 平台没有 DPAPI：直接报错，绝不静默返回明文
/// （否则 config.json 会以为"已加密"而实际写着明文，属于安全假象）。
pub fn protect(secret: &str) -> Result<String, String> {
    if secret.is_empty() || is_encrypted(secret) {
        return Ok(secret.to_string());
    }
    #[cfg(windows)]
    {
        let cipher = dpapi::protect(secret.as_bytes())?;
        return Ok(format!("{}{}", ENC_PREFIX, BASE64.encode(cipher)));
    }
    #[cfg(not(windows))]
    {
        let _ = secret;
        Err("凭证加密需要 Windows DPAPI，当前平台不支持；请勿在非 Windows 平台保存平台令牌".to_string())
    }
}

/// 解密单个凭证。空串/无前缀（旧版明文）原样返回，兼容旧配置。
pub fn unprotect(value: &str) -> Result<String, String> {
    if value.is_empty() || !is_encrypted(value) {
        return Ok(value.to_string());
    }
    // [n-04 修复] 非 Windows 平台无 DPAPI：密文无法还原，明确报错而不是编译失败。
    // 当前只发布 Windows 包；此分支让代码在非 Windows（如 CI / 本机调试）也能编译。
    #[cfg(not(windows))]
    {
        let _ = value;
        return Err("凭证解密需要 Windows DPAPI，当前平台不支持".to_string());
    }
    #[cfg(windows)]
    {
        let b64 = &value[ENC_PREFIX.len()..];
        let blob = BASE64
            .decode(b64)
            .map_err(|_| "凭证密文格式异常（base64 解码失败），请重新填写授权码".to_string())?;
        let plain = dpapi::unprotect(&blob)?;
        String::from_utf8(plain).map_err(|_| "凭证解密结果不是有效文本，请重新填写授权码".to_string())
    }
}

// ── DPAPI 实现（仅 Windows） ──

#[cfg(windows)]
mod dpapi {
    use windows_sys::Win32::Security::Cryptography::{
        CryptProtectData, CryptUnprotectData, CRYPTPROTECT_UI_FORBIDDEN, CRYPT_INTEGER_BLOB,
    };
    use windows_sys::Win32::Foundation::LocalFree;
    use super::ENTROPY;

    fn blob(data: &[u8]) -> CRYPT_INTEGER_BLOB {
        CRYPT_INTEGER_BLOB {
            cbData: data.len() as u32,
            pbData: data.as_ptr() as *mut u8,
        }
    }

    /// 取回输出 blob 内容并释放系统缓冲
    /// [m-12 修复] 释放前把系统缓冲里的明文令牌清零，缩短明文在内存的驻留窗口。
    /// 用 write_volatile 防止编译器把"写完马上 free"的清零优化掉。
    unsafe fn take_out(out: CRYPT_INTEGER_BLOB) -> Vec<u8> {
        let slice = std::slice::from_raw_parts_mut(out.pbData, out.cbData as usize);
        let v = slice.to_vec();
        // 清零 DPAPI 输出缓冲（明文凭证），再释放
        for byte in slice.iter_mut() {
            std::ptr::write_volatile(byte, 0u8);
        }
        LocalFree(out.pbData.cast());
        v
    }

    pub fn protect(data: &[u8]) -> Result<Vec<u8>, String> {
        unsafe {
            let mut out = CRYPT_INTEGER_BLOB { cbData: 0, pbData: std::ptr::null_mut() };
            let entropy = blob(ENTROPY);
            let ok = CryptProtectData(
                &blob(data),
                std::ptr::null(), // 描述串（不需要）
                &entropy,         // 附加熵
                std::ptr::null_mut(),
                std::ptr::null(),
                CRYPTPROTECT_UI_FORBIDDEN,
                &mut out,
            );
            if ok == 0 {
                return Err("凭证加密失败（DPAPI CryptProtectData）".to_string());
            }
            Ok(take_out(out))
        }
    }

    pub fn unprotect(data: &[u8]) -> Result<Vec<u8>, String> {
        unsafe {
            let mut out = CRYPT_INTEGER_BLOB { cbData: 0, pbData: std::ptr::null_mut() };
            let entropy = blob(ENTROPY);
            let ok = CryptUnprotectData(
                &blob(data),
                std::ptr::null_mut(),
                &entropy,
                std::ptr::null_mut(),
                std::ptr::null(),
                CRYPTPROTECT_UI_FORBIDDEN,
                &mut out,
            );
            if ok == 0 {
                return Err("凭证解密失败：配置可能来自其他电脑/账户（DPAPI 密钥不匹配），请重新填写授权码".to_string());
            }
            Ok(take_out(out))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 加密→解密还原原文（依赖 DPAPI，仅 Windows 可跑）
    #[test]
    #[cfg(windows)]
    fn roundtrip() {
        let secret = "p@ss w0rd 中文令牌🚀";
        let enc = protect(secret).unwrap();
        assert!(is_encrypted(&enc));
        assert_ne!(enc, secret);
        assert_eq!(unprotect(&enc).unwrap(), secret);
    }

    /// 已是密文时再加密保持不变（幂等，防二次包裹）
    #[test]
    #[cfg(windows)]
    fn idempotent() {
        let enc = protect("abc123").unwrap();
        assert_eq!(protect(&enc).unwrap(), enc);
    }

    #[test]
    fn empty_passthrough() {
        assert_eq!(protect("").unwrap(), "");
        assert_eq!(unprotect("").unwrap(), "");
    }

    /// 旧版明文（无前缀）解密原样返回，兼容迁移
    #[test]
    fn legacy_plaintext_passthrough() {
        assert_eq!(unprotect("secret_token_plain").unwrap(), "secret_token_plain");
    }

    #[test]
    fn corrupt_ciphertext_errors() {
        assert!(unprotect("enc1:not-valid-base64!!!").is_err());
    }
}
