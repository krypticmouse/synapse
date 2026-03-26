use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use synapse_channels::ConnectorConfig;
use synapse_dsl::ast::{ChannelDef, ConfigBlock, ConfigValue};

/// Runtime configuration parsed from the DSL config block.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimeConfig {
    pub storage: Option<StorageConfig>,
    pub vectors: HashMap<String, VectorConfig>,
    pub graphs: HashMap<String, GraphConfig>,
    pub embedding: Option<EmbeddingConfig>,
    pub extractor: Option<ExtractorConfig>,
    pub channels: HashMap<String, ChannelRuntimeConfig>,
    pub host: String,
    pub port: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelRuntimeConfig {
    pub source: String,
    pub config: ConnectorConfig,
    pub poll_interval_secs: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageConfig {
    pub backend: String,
    pub url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum VectorConfig {
    External { backend: String, url: String },
    Auto { backend: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum GraphConfig {
    External { backend: String, url: String },
    Auto { backend: String },
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
            vectors: HashMap::new(),
            graphs: HashMap::new(),
            embedding: None,
            extractor: None,
            channels: HashMap::new(),
            host: "localhost".into(),
            port: 8080,
        }
    }
}

fn parse_vector_entry(value: &ConfigValue) -> Option<VectorConfig> {
    match value {
        ConfigValue::FnCall { name, arg } if name == "auto" => Some(VectorConfig::Auto {
            backend: arg.clone(),
        }),
        ConfigValue::FnCall { name, arg } => Some(VectorConfig::External {
            backend: name.clone(),
            url: arg.clone(),
        }),
        ConfigValue::Auto => Some(VectorConfig::Auto {
            backend: "qdrant".into(),
        }),
        _ => None,
    }
}

fn parse_graph_entry(value: &ConfigValue) -> Option<GraphConfig> {
    match value {
        ConfigValue::FnCall { name, arg } if name == "auto" => Some(GraphConfig::Auto {
            backend: arg.clone(),
        }),
        ConfigValue::FnCall { name, arg } => Some(GraphConfig::External {
            backend: name.clone(),
            url: arg.clone(),
        }),
        ConfigValue::Auto => Some(GraphConfig::Auto {
            backend: "neo4j".into(),
        }),
        _ => None,
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
                    ConfigValue::Dict(entries) => {
                        for (name, val) in entries {
                            if let Some(vc) = parse_vector_entry(val) {
                                cfg.vectors.insert(name.clone(), vc);
                            }
                        }
                    }
                    other => {
                        if let Some(vc) = parse_vector_entry(other) {
                            cfg.vectors.insert("default".into(), vc);
                        }
                    }
                },
                "graph" => match &entry.value {
                    ConfigValue::Dict(entries) => {
                        for (name, val) in entries {
                            if let Some(gc) = parse_graph_entry(val) {
                                cfg.graphs.insert(name.clone(), gc);
                            }
                        }
                    }
                    other => {
                        if let Some(gc) = parse_graph_entry(other) {
                            cfg.graphs.insert("default".into(), gc);
                        }
                    }
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
                _ => {}
            }
        }

        cfg
    }

    pub fn add_channel_from_def(&mut self, def: &ChannelDef) {
        let mut connector_config = ConnectorConfig::new();
        for entry in &def.config {
            let value_str = match &entry.value {
                ConfigValue::FnCall { name, arg } => {
                    if name == "env" {
                        std::env::var(arg).unwrap_or_default()
                    } else {
                        format!("{}({})", name, arg)
                    }
                }
                _ => String::new(),
            };
            connector_config.set(&entry.key, value_str);
        }

        let poll_interval_secs = def
            .poll_interval
            .as_ref()
            .map(|d| d.to_secs())
            .unwrap_or(300); // default 5 minutes

        self.channels.insert(
            def.name.clone(),
            ChannelRuntimeConfig {
                source: def.source.clone(),
                config: connector_config,
                poll_interval_secs,
            },
        );
    }
}
