use rig::completion::Prompt;
use rig::embeddings::EmbeddingModel as _;
use rig::providers::openai;

use crate::config::ExtractorConfig;
use crate::value::{Record, Value};

const EXTRACT_PREAMBLE: &str = r#"You are a fact extraction engine. Given text, extract structured facts as a JSON array.
Each fact must have these fields:
- "content": the fact as a natural language statement
- "subject": the entity the fact is about
- "predicate": the relationship or attribute
- "object": the value or related entity
- "confidence": a float between 0.0 and 1.0 indicating confidence

Respond with ONLY a valid JSON array. No markdown fences, no explanation."#;

const SUMMARIZE_PREAMBLE: &str =
    "You are a summarization engine. Given text, produce a concise summary \
     that captures the key information. Respond with ONLY the summary text.";

/// LLM client for extraction and summarization operations.
/// Wraps a rig-core OpenAI client and builds purpose-specific agents on demand.
pub struct LlmClient {
    client: openai::Client,
    model: String,
}

impl LlmClient {
    /// Build an LlmClient from the parsed DSL extractor config.
    /// Currently only supports the `openai` provider.
    pub fn from_config(cfg: &ExtractorConfig) -> anyhow::Result<Self> {
        match cfg.provider.as_str() {
            "openai" => {
                let client = openai::Client::from_env();
                Ok(Self {
                    client,
                    model: cfg.model.clone(),
                })
            }
            other => anyhow::bail!("unsupported extractor provider: {other}"),
        }
    }

    /// Extract structured facts from free text via LLM.
    /// Returns a `Vec<Value::Record>` with fields: content, subject, predicate, object, confidence.
    pub async fn extract(&self, text: &str) -> anyhow::Result<Vec<Value>> {
        let agent = self.client.agent(&self.model).preamble(EXTRACT_PREAMBLE).build();

        let response = agent.prompt(text).await?;
        let json_str = strip_code_fences(&response);

        let parsed: serde_json::Value =
            serde_json::from_str(json_str).unwrap_or_else(|_| serde_json::json!([]));

        let now = Value::Timestamp(chrono::Utc::now());
        let facts = match parsed {
            serde_json::Value::Array(arr) => arr
                .into_iter()
                .filter_map(|item| {
                    let obj = item.as_object()?;
                    let mut record = Record::new("Fact");
                    for (k, v) in obj {
                        record.set(k, Value::from(v.clone()));
                    }
                    record.set("valid_from", now.clone());
                    Some(Value::Record(record))
                })
                .collect(),
            _ => vec![],
        };

        Ok(facts)
    }

    /// Summarize text via LLM, returning a plain string.
    pub async fn summarize(&self, text: &str) -> anyhow::Result<String> {
        let agent = self
            .client
            .agent(&self.model)
            .preamble(SUMMARIZE_PREAMBLE)
            .build();

        let response = agent.prompt(text).await?;
        Ok(response)
    }

    /// Call an extern function by simulating it via LLM.
    pub async fn call_extern(
        &self,
        fn_name: &str,
        params: &[(String, String)], // (param_name, param_type)
        return_type: &str,
        arg_values: &[Value],
    ) -> anyhow::Result<Value> {
        let param_desc: Vec<String> = params
            .iter()
            .enumerate()
            .map(|(i, (name, ty))| {
                let val = arg_values
                    .get(i)
                    .map(|v| format!("{v:?}"))
                    .unwrap_or_else(|| "null".into());
                format!("  {name}: {ty} = {val}")
            })
            .collect();

        let prompt = format!(
            "You are simulating a function call.\n\
             Function: {fn_name}({params_str}) -> {return_type}\n\
             Arguments:\n{args}\n\n\
             Return the result as valid JSON. For arrays, return a JSON array. \
             For objects, return a JSON object. For strings, return a JSON string. \
             Respond with ONLY the JSON value.",
            params_str = params
                .iter()
                .map(|(n, t)| format!("{n}: {t}"))
                .collect::<Vec<_>>()
                .join(", "),
            args = param_desc.join("\n"),
        );

        let agent = self.client.agent(&self.model).build();
        let response = agent.prompt(prompt.as_str()).await?;
        let json_str = strip_code_fences(&response);
        let parsed: serde_json::Value =
            serde_json::from_str(json_str).unwrap_or(serde_json::Value::Null);
        Ok(Value::from(parsed))
    }
}

/// Embedding client for generating vector embeddings.
pub struct EmbeddingClient {
    client: openai::Client,
    model_name: String,
}

impl std::fmt::Debug for EmbeddingClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EmbeddingClient")
            .field("model_name", &self.model_name)
            .finish()
    }
}

impl EmbeddingClient {
    pub fn from_config(cfg: &crate::config::EmbeddingConfig) -> anyhow::Result<Self> {
        match cfg.provider.as_str() {
            "openai" => {
                let client = openai::Client::from_env();
                Ok(Self {
                    client,
                    model_name: cfg.model.clone(),
                })
            }
            other => anyhow::bail!("unsupported embedding provider: {other}"),
        }
    }

    /// Generate an embedding vector for a single text.
    pub async fn embed(&self, text: &str) -> anyhow::Result<Vec<f32>> {
        let model = self.client.embedding_model(&self.model_name);
        let embeddings = model.embed_text(text).await?;
        Ok(embeddings.vec.iter().map(|&v| v as f32).collect())
    }

    /// Compute cosine similarity between two texts.
    pub async fn similarity(&self, text_a: &str, text_b: &str) -> anyhow::Result<f64> {
        let model = self.client.embedding_model(&self.model_name);
        let results = model.embed_texts(vec![text_a.to_string(), text_b.to_string()]).await?;
        if results.len() < 2 {
            return Ok(0.0);
        }
        Ok(cosine_similarity(&results[0].vec, &results[1].vec))
    }
}

fn cosine_similarity(a: &[f64], b: &[f64]) -> f64 {
    let dot: f64 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let norm_a: f64 = a.iter().map(|x| x * x).sum::<f64>().sqrt();
    let norm_b: f64 = b.iter().map(|x| x * x).sum::<f64>().sqrt();
    if norm_a == 0.0 || norm_b == 0.0 {
        0.0
    } else {
        dot / (norm_a * norm_b)
    }
}

/// Strip markdown code fences that LLMs sometimes wrap around JSON output.
fn strip_code_fences(s: &str) -> &str {
    let trimmed = s.trim();
    if let Some(rest) = trimmed.strip_prefix("```json") {
        rest.strip_suffix("```").unwrap_or(rest).trim()
    } else if let Some(rest) = trimmed.strip_prefix("```") {
        rest.strip_suffix("```").unwrap_or(rest).trim()
    } else {
        trimmed
    }
}
