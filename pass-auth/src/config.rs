use std::path::PathBuf;

#[derive(Clone, Debug)]
pub struct ClientConfig {
    pub base_dir: PathBuf,
    pub environment: Option<String>,
    pub proxy_config: ProxyConfig,
    pub debug_config: Option<DebugConfig>,
    pub app_header: Option<String>,
    pub post_login_config: PostLoginConfig,
}

impl ClientConfig {
    pub fn new(base_dir: PathBuf) -> Self {
        Self {
            base_dir,
            environment: None,
            proxy_config: ProxyConfig::default(),
            debug_config: None,
            app_header: None,
            post_login_config: PostLoginConfig::default(),
        }
    }

    pub fn with_environment(mut self, env: String) -> Self {
        self.environment = Some(env);
        self
    }

    pub fn with_proxy_config(mut self, proxy_config: ProxyConfig) -> Self {
        self.proxy_config = proxy_config;
        self
    }

    pub fn with_debug_config(mut self, debug_config: DebugConfig) -> Self {
        self.debug_config = Some(debug_config);
        self
    }

    pub fn with_app_header(mut self, app_header: String) -> Self {
        self.app_header = Some(app_header);
        self
    }

    pub fn with_post_login_config(mut self, post_login_config: PostLoginConfig) -> Self {
        self.post_login_config = post_login_config;
        self
    }
}

#[derive(Clone, Debug, Default)]
pub struct ProxyConfig {
    pub http_proxy: Option<String>,
    pub https_proxy: Option<String>,
}

impl ProxyConfig {
    pub fn from_env() -> Self {
        Self {
            http_proxy: std::env::var("HTTP_PROXY").ok(),
            https_proxy: std::env::var("HTTPS_PROXY").ok(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct DebugConfig {
    pub xdebug_session: Option<String>,
}

impl DebugConfig {
    pub fn from_env() -> Option<Self> {
        std::env::var("XDEBUG_SESSION").ok().map(|session| Self {
            xdebug_session: Some(session),
        })
    }
}
#[derive(Clone, Debug)]
pub struct PostLoginConfig {
    pub create_default_vault: bool,
    pub default_vault_name: String,
}

impl Default for PostLoginConfig {
    fn default() -> Self {
        Self {
            create_default_vault: true,
            default_vault_name: "Personal".to_string(),
        }
    }
}
