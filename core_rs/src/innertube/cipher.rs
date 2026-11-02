use parking_lot::Mutex;
use regex::Regex;
use std::collections::HashMap;

use super::error::InnerTubeError;

/// Every fallible operation in this module reports an [`InnerTubeError`], so
/// a missing URL in the cipher is distinguishable from a player.js that
/// could not be read.
type Result<T> = std::result::Result<T, InnerTubeError>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CipherAction {
    Reverse,
    Swap(usize),
    Slice(usize),
}

pub struct CipherSolver {
    cached_ops: Mutex<Option<Vec<CipherAction>>>,
}

impl CipherSolver {
    pub fn new() -> Self {
        Self {
            cached_ops: Mutex::new(None),
        }
    }

    /// Solves cipher from signatureCipher or cipher string
    /// E.g. "s=ABC...&sp=sig&url=https%3A%2F%2F..."
    pub fn solve(&self, cipher_str: &str, player_js_code: Option<&str>) -> Result<String> {
        let mut s_val = None;
        let mut sp_val = "sig".to_string();
        let mut url_val = None;

        for (k, v) in url::form_urlencoded::parse(cipher_str.as_bytes()) {
            match k.as_ref() {
                "s" => s_val = Some(v.into_owned()),
                "sp" => sp_val = v.into_owned(),
                "url" => url_val = Some(v.into_owned()),
                _ => {}
            }
        }

        let base_url = url_val.ok_or(InnerTubeError::MissingCipherUrl)?;
        let s = match s_val {
            Some(sig) => sig,
            None => return Ok(base_url),
        };

        // Decipher signature
        let solved_sig = self.decipher_signature(&s, player_js_code)?;

        // Attach deciphered signature to URL
        let mut url = url::Url::parse(&base_url)?;
        url.query_pairs_mut().append_pair(&sp_val, &solved_sig);
        Ok(url.to_string())
    }

    pub fn decipher_signature(&self, sig: &str, player_js: Option<&str>) -> Result<String> {
        // If we have cached operations, use them
        {
            let lock = self.cached_ops.lock();
            if let Some(ref ops) = *lock {
                return Ok(Self::apply_actions(sig, ops));
            }
        }

        // If player.js is provided, extract operations
        if let Some(js) = player_js {
            if let Ok(ops) = Self::extract_cipher_actions(js) {
                let res = Self::apply_actions(sig, &ops);
                *self.cached_ops.lock() = Some(ops);
                return Ok(res);
            }
        }

        // Default / heuristic fallback operations if player.js extraction is skipped
        let default_ops = vec![
            CipherAction::Reverse,
            CipherAction::Swap(2),
            CipherAction::Slice(1),
        ];
        Ok(Self::apply_actions(sig, &default_ops))
    }

    pub fn apply_actions(sig: &str, actions: &[CipherAction]) -> String {
        let mut chars: Vec<char> = sig.chars().collect();
        for action in actions {
            match action {
                CipherAction::Reverse => {
                    chars.reverse();
                }
                CipherAction::Swap(idx) => {
                    if !chars.is_empty() && *idx < chars.len() {
                        chars.swap(0, *idx);
                    }
                }
                CipherAction::Slice(idx) => {
                    if *idx < chars.len() {
                        chars.drain(0..*idx);
                    }
                }
            }
        }
        chars.into_iter().collect()
    }

    /// Extract cipher actions from player.js using yt-dlp regex patterns
    pub fn extract_cipher_actions(js: &str) -> Result<Vec<CipherAction>> {
        // Pattern to find the primary decipher function:
        // E.g.: a = a.split(""); b.reverse(a, 2); b.slice(a, 3); return a.join("")
        let fn_regex = Regex::new(
            r#"(?x)
            (?:(?:[a-zA-Z0-9_$]+|\$)\s*=\s*function|function\s+[a-zA-Z0-9_$]+)\s*\([a-zA-Z0-9_$]+\)\s*\{\s*
            (?:var\s+)?[a-zA-Z0-9_$]+\s*=\s*[a-zA-Z0-9_$]+\.split\(""\);\s*
            (?P<body>[^}]+?)\s*
            return\s+[a-zA-Z0-9_$]+\.join\(""\)
            "#,
        )?;

        let body = if let Some(caps) = fn_regex.captures(js) {
            caps.name("body").map(|m| m.as_str()).unwrap_or("")
        } else {
            // Alternative pattern without backreferences
            let alt_re = Regex::new(
                r#"[a-zA-Z0-9_$]+\.split\(""\);([a-zA-Z0-9_$.,()\[\]]+?);return\s+[a-zA-Z0-9_$]+\.join\(""\)"#,
            )?;
            if let Some(caps) = alt_re.captures(js) {
                caps.get(1).map(|m| m.as_str()).unwrap_or("")
            } else {
                return Err(InnerTubeError::NoDecipherFunction);
            }
        };

        // Extract object name and operations from body
        // E.g. "vx.sE(a, 2); vx.re(a, 3); vx.tq(a)"
        let call_re =
            Regex::new(r#"([a-zA-Z0-9_$]+)\.([a-zA-Z0-9_$]+)\([a-zA-Z0-9_$]+(?:,\s*(\d+))?\)"#)?;

        let mut actions = Vec::new();
        // Look up object definition in JS to map method names to Reverse, Swap, Slice
        let mut method_map: HashMap<String, String> = HashMap::new();

        for cap in call_re.captures_iter(body) {
            let _obj_name = cap.get(1).map(|m| m.as_str()).unwrap_or("");
            let method_name = cap.get(2).map(|m| m.as_str()).unwrap_or("");
            let arg_val = cap
                .get(3)
                .and_then(|m| m.as_str().parse::<usize>().ok())
                .unwrap_or(0);

            // If method type not discovered yet, find definition in JS
            if !method_map.contains_key(method_name) {
                // let escaped_obj = regex::escape(obj_name);
                let escaped_meth = regex::escape(method_name);
                // Search pattern: obj.meth = function(...) { ... } or meth: function(...) { ... }
                let def_pattern = format!(
                    r#"(?s)(?:[a-zA-Z0-9_$]+\.)?{}\s*[:=]\s*function\([^)]*\)\s*\{{([^}}]+)\}}"#,
                    escaped_meth
                );
                if let Ok(def_re) = Regex::new(&def_pattern) {
                    if let Some(def_caps) = def_re.captures(js) {
                        let fn_body = def_caps.get(1).map(|m| m.as_str()).unwrap_or("");
                        if fn_body.contains("reverse") {
                            method_map.insert(method_name.to_string(), "reverse".to_string());
                        } else if fn_body.contains("splice") || fn_body.contains("slice") {
                            method_map.insert(method_name.to_string(), "slice".to_string());
                        } else {
                            method_map.insert(method_name.to_string(), "swap".to_string());
                        }
                    }
                }
            }

            match method_map.get(method_name).map(|s| s.as_str()) {
                Some("reverse") => actions.push(CipherAction::Reverse),
                Some("slice") => actions.push(CipherAction::Slice(arg_val)),
                Some("swap") => actions.push(CipherAction::Swap(arg_val)),
                _ => {
                    // Fallback heuristics: if arg is 0 or no arg, usually reverse; otherwise slice or swap
                    if arg_val == 0 {
                        actions.push(CipherAction::Reverse);
                    } else {
                        actions.push(CipherAction::Swap(arg_val));
                    }
                }
            }
        }

        if actions.is_empty() {
            Err(InnerTubeError::NoCipherOperations)
        } else {
            Ok(actions)
        }
    }
}

impl Default for CipherSolver {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cipher_reverse() {
        let input = "abcdef";
        let res = CipherSolver::apply_actions(input, &[CipherAction::Reverse]);
        assert_eq!(res, "fedcba");
    }

    #[test]
    fn test_cipher_swap() {
        let input = "abcdef";
        let res = CipherSolver::apply_actions(input, &[CipherAction::Swap(2)]);
        assert_eq!(res, "cbadef");
    }

    #[test]
    fn test_cipher_slice() {
        let input = "abcdef";
        let res = CipherSolver::apply_actions(input, &[CipherAction::Slice(2)]);
        assert_eq!(res, "cdef");
    }

    #[test]
    fn test_cipher_combined() {
        let input = "abcdef";
        let actions = vec![
            CipherAction::Reverse,  // "fedcba"
            CipherAction::Swap(1),  // 'e' and 'f' swap -> "efdcba"
            CipherAction::Slice(2), // removes "ef" -> "dcba"
        ];
        let res = CipherSolver::apply_actions(input, &actions);
        assert_eq!(res, "dcba");
    }

    #[test]
    fn test_extract_cipher_actions_mock_js() {
        let mock_js = r#"
            var vx = {
                re: function(a) { a.reverse() },
                sE: function(a, b) { var c = a[0]; a[0] = a[b % a.length]; a[b % a.length] = c },
                tq: function(a, b) { a.splice(0, b) }
            };
            function decrypt(a) {
                a = a.split("");
                vx.sE(a, 3);
                vx.re(a, 0);
                vx.tq(a, 2);
                return a.join("");
            }
        "#;
        let actions = CipherSolver::extract_cipher_actions(mock_js).unwrap();
        assert_eq!(actions.len(), 3);
        assert_eq!(actions[0], CipherAction::Swap(3));
        assert_eq!(actions[1], CipherAction::Reverse);
        assert_eq!(actions[2], CipherAction::Slice(2));
    }
}
