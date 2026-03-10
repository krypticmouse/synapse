use serde::{Deserialize, Serialize};

use synapse_core::ast::{ConfigBlock, ConfigValue};

/// Runtime configuration parsed from the DSL config block.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimeConfig {
    pub storage: Option<StorageConfig>,
    pub vector: Option<VectorConfig>,
    pub graph: Option<GraphConfig>,
    pub embedding: Option<EmbeddingConfig>,
    pub extractor: Option<ExtractorConfig>,
    pub host: String,
    pub port: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageConfig {
    pub backend: String,
    pub url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum VectorConfig {
    External { backend: String, url: String },
    Auto,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum GraphConfig {
    External { backend: String, url: String },
    Auto,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddingConfig {
    pub provider: String,
    pub model: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractorConfig {
    pub provider: String,
    pub model: String,
}

impl Default for RuntimeConfig {
    fn default() -> Self {
        Self {
            storage: None,
            vector: None,
            graph: None,
            embedding: None,
            extractor: None,
            host: "localhost".into(),
            port: 8080,
        }
    }
}

impl RuntimeConfig {
    /// Build runtime config from the parsed DSL config block
    pub fn from_config_block(block: &ConfigBlock) -> Self {
        let mut cfg = Self::default();

        for entry in &block.entries {
            match entry.key.as_str() {
                "storage" => {
                    if let ConfigValue::FnCall { name, arg } = &entry.value {
                        cfg.storage = Some(StorageConfig {
                            backend: name.clone(),
                            url: arg.clone(),
                        });
                    }
                }
                "vector" => match &entry.value {
                    ConfigValue::FnCall { name, arg } => {
                        cfg.vector = Some(VectorConfig::External {
                            backend: name.clone(),
                            url: arg.clone(),
                        });
                    }
                    ConfigValue::Auto => {
                        cfg.vector = Some(VectorConfig::Auto);
                    }
                    _ => {}
                },
                "graph" => match &entry.value {
                    ConfigValue::FnCall { name, arg } => {
                        cfg.graph = Some(GraphConfig::External {
                            backend: name.clone(),
                            url: arg.clone(),
                        });
                    }
                    ConfigValue::Auto => {
                        cfg.graph = Some(GraphConfig::Auto);
                    }
                    _ => {}
                },
                "embedding" => {
                    if let ConfigValue::FnCall { name, arg } = &entry.value {
                        cfg.embedding = Some(EmbeddingConfig {
                            provider: name.clone(),
                            model: arg.clone(),
                        });
                    }
                }
                "extractor" => {
                    if let ConfigValue::FnCall { name, arg } = &entry.value {
                        cfg.extractor = Some(ExtractorConfig {
                            provider: name.clone(),
                            model: arg.clone(),
                        });
                    }
                }
                _ => {} // ignore unknown config keys
            }
        }

        cfg
    }
}
