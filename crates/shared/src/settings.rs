use std::collections::BTreeMap;

pub const APP_NAME: &str = "YT-HOME";
pub const APP_VERSION: &str = env!("CARGO_PKG_VERSION");
pub const SESSION_COOKIE: &str = "YT-HOME";

pub const DEFAULT_CONFIG_JSON: &str = r#"{
  "log": {
    "level": "info",
    "timestamp": true
  },
  "dns": {
    "servers": [
      {
        "type": "local",
        "tag": "local-dns"
      }
    ],
    "rules": [],
    "final": "local-dns"
  },
  "route": {
    "rules": [
      {
        "action": "sniff"
      },
      {
        "protocol": [
          "dns"
        ],
        "action": "hijack-dns"
      }
    ],
    "rule_set": [],
    "final": "direct",
    "auto_detect_interface": true
  },
  "experimental": {
    "cache_file": {
      "enabled": true
    }
  }
}"#;

pub fn default_settings() -> BTreeMap<&'static str, &'static str> {
    BTreeMap::from([
        ("changeRetention", "1000"),
        ("config", DEFAULT_CONFIG_JSON),
        ("sessionMaxAge", "0"),
        ("subCertFile", ""),
        ("subClashExt", ""),
        ("subDomain", ""),
        ("subEncode", "true"),
        ("subJsonExt", ""),
        ("subKeyFile", ""),
        ("subListen", ""),
        ("subPath", "/sub/"),
        ("subPort", "2096"),
        ("subShowInfo", "false"),
        ("subURI", ""),
        ("subUpdates", "12"),
        ("timeLocation", "UTC"),
        ("trafficAge", "30"),
        ("version", APP_VERSION),
        ("webCertFile", ""),
        ("webDomain", ""),
        ("webKeyFile", ""),
        ("webListen", ""),
        ("webPath", "/"),
        ("webPort", "80"),
        ("webURI", ""),
    ])
}
