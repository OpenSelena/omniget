use serde::{Deserialize, Serialize};
use std::sync::{Mutex, OnceLock};

const AI_CONFIG_FILE: &str = "ai_config.json";

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AiProvider {
    #[default]
    None,
    Openai,
    Anthropic,
    Local,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct AiConfig {
    #[serde(default)]
    pub provider: AiProvider,
    /// Provedor da tabela (`ai_keys::KINDS`) por trás do `provider`: é ele que
    /// diz qual endereço usar e se a chave é obrigatória. Vazio nas configurações
    /// antigas, feitas quando só existia "OpenAI / Anthropic / local".
    #[serde(default)]
    pub kind: String,
    #[serde(default)]
    pub openai_key: String,
    #[serde(default)]
    pub anthropic_key: String,
    #[serde(default)]
    pub local_base_url: String,
    #[serde(default)]
    pub model: String,
}

// Sent to the frontend instead of AiConfig: key material is replaced with
// presence booleans so a raw key can never reach the UI, logs, or telemetry.
#[derive(Clone, Debug, Serialize)]
pub struct AiConfigView {
    pub provider: AiProvider,
    pub kind: String,
    pub model: String,
    pub local_base_url: String,
    pub has_openai_key: bool,
    pub has_anthropic_key: bool,
}

impl AiConfig {
    pub fn view(&self) -> AiConfigView {
        AiConfigView {
            provider: self.provider,
            kind: self.kind.clone(),
            model: self.model.clone(),
            local_base_url: self.local_base_url.clone(),
            has_openai_key: !self.openai_key.is_empty(),
            has_anthropic_key: !self.anthropic_key.is_empty(),
        }
    }

    pub fn is_configured(&self) -> bool {
        match self.provider {
            AiProvider::None => false,
            AiProvider::Openai => !self.openai_key.is_empty(),
            AiProvider::Anthropic => !self.anthropic_key.is_empty(),
            // Config antiga (sem `kind`): basta o endereço, como sempre foi.
            // Com provedor da tabela, quem não dispensa chave precisa dela.
            AiProvider::Local => {
                !self.local_base_url.is_empty()
                    && (!self.kind_needs_key() || !self.openai_key.is_empty())
            }
        }
    }

    fn kind_needs_key(&self) -> bool {
        match crate::core::tools::ai_keys::find_kind(&self.kind) {
            Some(kind) => kind.needs_key(),
            // Provedor fora da tabela é desconhecido: pede chave.
            None => !self.kind.is_empty(),
        }
    }
}

static STORE: OnceLock<Mutex<AiConfig>> = OnceLock::new();

fn store() -> &'static Mutex<AiConfig> {
    STORE.get_or_init(|| Mutex::new(load_from_disk()))
}

fn file_path() -> Option<std::path::PathBuf> {
    // Nos testes o arquivo vai para uma pasta temporária: a configuração real do
    // usuário não pode ser tocada por um `cargo test`.
    #[cfg(test)]
    if let Some(dir) = TEST_DIR.with(|d| d.borrow().clone()) {
        return Some(dir.join(AI_CONFIG_FILE));
    }
    crate::core::paths::app_data_dir().map(|d| d.join(AI_CONFIG_FILE))
}

#[cfg(test)]
thread_local! {
    static TEST_DIR: std::cell::RefCell<Option<std::path::PathBuf>> =
        const { std::cell::RefCell::new(None) };
}

fn load_from_disk() -> AiConfig {
    let Some(path) = file_path() else {
        return AiConfig::default();
    };
    match std::fs::read_to_string(&path) {
        Ok(c) => serde_json::from_str(&c).unwrap_or_default(),
        Err(_) => AiConfig::default(),
    }
}

fn write_to_disk(cfg: &AiConfig) {
    let Some(path) = file_path() else { return };
    let Some(parent) = path.parent() else { return };
    if let Err(e) = std::fs::create_dir_all(parent) {
        tracing::warn!("[ai] create_dir_all failed: {}", e);
        return;
    }
    let serialized = match serde_json::to_string_pretty(cfg) {
        Ok(s) => s,
        Err(e) => {
            tracing::warn!("[ai] serialize failed: {}", e);
            return;
        }
    };
    let tmp = path.with_extension("json.tmp");
    let result = (|| -> std::io::Result<()> {
        use std::io::Write;
        let mut f = std::fs::File::create(&tmp)?;
        f.write_all(serialized.as_bytes())?;
        f.sync_all()?;
        Ok(())
    })();
    if let Err(e) = result {
        tracing::warn!("[ai] write tmp failed: {}", e);
        let _ = std::fs::remove_file(&tmp);
        return;
    }
    if let Err(e) = std::fs::rename(&tmp, &path) {
        tracing::warn!("[ai] rename failed: {}", e);
        let _ = std::fs::remove_file(&tmp);
    }
}

pub fn get() -> AiConfig {
    store().lock().unwrap().clone()
}

// Key fields are Option: None keeps the stored key untouched (so the UI never
// has to round-trip secrets), Some("") clears it.
pub fn set(
    provider: AiProvider,
    model: String,
    local_base_url: String,
    openai_key: Option<String>,
    anthropic_key: Option<String>,
) -> AiConfig {
    let mut guard = store().lock().unwrap();
    guard.provider = provider;
    guard.model = model.trim().to_string();
    guard.local_base_url = local_base_url.trim().trim_end_matches('/').to_string();
    if let Some(k) = openai_key {
        guard.openai_key = k.trim().to_string();
    }
    if let Some(k) = anthropic_key {
        guard.anthropic_key = k.trim().to_string();
    }
    write_to_disk(&guard);
    guard.clone()
}

/// O que fazer com a credencial guardada quando a tela salva a configuração.
/// O formulário nunca recebe o segredo de volta, então "não mexi no campo" e
/// "apaguei o campo" precisam chegar aqui como coisas diferentes.
pub enum KeyAction<'a> {
    /// Mantém a chave guardada.
    Keep,
    /// Substitui pela chave digitada.
    Set(&'a str),
    /// Apaga a credencial (troca de provedor, ou campo esvaziado).
    Clear,
}

fn provider_for_kind(kind: &str) -> AiProvider {
    match kind {
        "openai" => AiProvider::Openai,
        "anthropic" => AiProvider::Anthropic,
        // Todo o resto fala o dialeto OpenAI (cada um no seu endereço).
        _ => AiProvider::Local,
    }
}

fn apply_kind(cfg: &mut AiConfig, kind: &str, model: String, base_url: String, key: KeyAction<'_>) {
    let kind = kind.trim();
    cfg.provider = provider_for_kind(kind);
    cfg.kind = kind.to_string();
    cfg.model = model.trim().to_string();
    cfg.local_base_url = if cfg.provider == AiProvider::Local {
        crate::core::tools::ai_keys::app_base_url(kind, &base_url)
    } else {
        String::new()
    };
    match key {
        KeyAction::Keep => {}
        KeyAction::Set(k) => {
            let k = k.trim().to_string();
            if cfg.provider == AiProvider::Anthropic {
                cfg.anthropic_key = k;
            } else {
                cfg.openai_key = k;
            }
        }
        KeyAction::Clear => {
            cfg.openai_key.clear();
            cfg.anthropic_key.clear();
        }
    }
}

/// Salva a escolha da tela de IA: o provedor da tabela define a rota e a
/// necessidade de chave, e `key` decide o destino da credencial guardada.
pub fn set_with_kind(kind: &str, model: String, base_url: String, key: KeyAction<'_>) -> AiConfig {
    let mut guard = store().lock().unwrap();
    apply_kind(&mut guard, kind, model, base_url, key);
    write_to_disk(&guard);
    guard.clone()
}

/// Desliga a IA (provedor "none"): some o provedor e o tipo; endereço, modelo e
/// chave ficam guardados para quando o usuário religar.
pub fn clear() -> AiConfig {
    let mut guard = store().lock().unwrap();
    guard.provider = AiProvider::None;
    guard.kind = String::new();
    write_to_disk(&guard);
    guard.clone()
}

fn http_client() -> Result<reqwest::Client, String> {
    let builder = reqwest::Client::builder().timeout(std::time::Duration::from_secs(120));
    crate::core::http_client::apply_global_proxy(builder)
        .build()
        .map_err(|e| format!("HTTP client error: {}", e))
}

pub async fn chat(system: &str, user: &str) -> Result<String, String> {
    let cfg = get();
    if cfg.model.is_empty() {
        return Err("No AI model configured".to_string());
    }
    match cfg.provider {
        AiProvider::None => Err("AI is not configured".to_string()),
        AiProvider::Openai => {
            openai_chat(
                "https://api.openai.com/v1/chat/completions",
                &cfg.openai_key,
                &cfg.model,
                system,
                user,
            )
            .await
        }
        AiProvider::Local => {
            if cfg.local_base_url.is_empty() {
                return Err("No local endpoint configured".to_string());
            }
            let endpoint = format!("{}/chat/completions", cfg.local_base_url);
            openai_chat(&endpoint, &cfg.openai_key, &cfg.model, system, user).await
        }
        AiProvider::Anthropic => anthropic_chat(&cfg.anthropic_key, &cfg.model, system, user).await,
    }
}

async fn openai_chat(
    endpoint: &str,
    key: &str,
    model: &str,
    system: &str,
    user: &str,
) -> Result<String, String> {
    let client = http_client()?;
    let body = serde_json::json!({
        "model": model,
        "messages": [
            { "role": "system", "content": system },
            { "role": "user", "content": user },
        ],
    });
    let mut req = client.post(endpoint).json(&body);
    if !key.is_empty() {
        req = req.bearer_auth(key);
    }
    let resp = req
        .send()
        .await
        .map_err(|e| format!("Request failed: {}", e))?;
    let status = resp.status();
    let text = resp
        .text()
        .await
        .map_err(|e| format!("Read body failed: {}", e))?;
    if !status.is_success() {
        return Err(format!("AI error ({})", status.as_u16()));
    }
    let json: serde_json::Value =
        serde_json::from_str(&text).map_err(|e| format!("Bad JSON: {}", e))?;
    // Ledger de custo (Tools → Custos de IA): tokens que o provedor informou.
    let usage = json.get("usage");
    let input = usage
        .and_then(|u| u.get("prompt_tokens"))
        .and_then(|v| v.as_u64())
        .unwrap_or(0);
    let output = usage
        .and_then(|u| u.get("completion_tokens"))
        .and_then(|v| v.as_u64())
        .unwrap_or(0);
    let provider = if endpoint.starts_with("https://api.openai.com") {
        "openai"
    } else {
        "local"
    };
    let cost = if provider == "local" { Some(0.0) } else { None };
    crate::core::tools::usage::record("chat", provider, model, input, output, cost);
    json.get("choices")
        .and_then(|c| c.get(0))
        .and_then(|c| c.get("message"))
        .and_then(|m| m.get("content"))
        .and_then(|c| c.as_str())
        .map(|s| s.trim().to_string())
        .ok_or_else(|| "Empty AI response".to_string())
}

async fn anthropic_chat(
    key: &str,
    model: &str,
    system: &str,
    user: &str,
) -> Result<String, String> {
    if key.is_empty() {
        return Err("No Anthropic key configured".to_string());
    }
    let client = http_client()?;
    let body = serde_json::json!({
        "model": model,
        "max_tokens": 1024,
        "system": system,
        "messages": [ { "role": "user", "content": user } ],
    });
    let resp = client
        .post("https://api.anthropic.com/v1/messages")
        .header("x-api-key", key)
        .header("anthropic-version", "2023-06-01")
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("Request failed: {}", e))?;
    let status = resp.status();
    let text = resp
        .text()
        .await
        .map_err(|e| format!("Read body failed: {}", e))?;
    if !status.is_success() {
        return Err(format!("AI error ({})", status.as_u16()));
    }
    let json: serde_json::Value =
        serde_json::from_str(&text).map_err(|e| format!("Bad JSON: {}", e))?;
    let usage = json.get("usage");
    let input = usage
        .and_then(|u| u.get("input_tokens"))
        .and_then(|v| v.as_u64())
        .unwrap_or(0);
    let output = usage
        .and_then(|u| u.get("output_tokens"))
        .and_then(|v| v.as_u64())
        .unwrap_or(0);
    crate::core::tools::usage::record("chat", "anthropic", model, input, output, None);
    json.get("content")
        .and_then(|c| c.get(0))
        .and_then(|c| c.get("text"))
        .and_then(|t| t.as_str())
        .map(|s| s.trim().to_string())
        .ok_or_else(|| "Empty AI response".to_string())
}

// Whisper-style transcription via the OpenAI-compatible audio endpoint. Only
// available for Openai/Local providers (Anthropic has no audio API). Reuses
// the configured key/base so no separate model management is needed.
pub async fn transcribe(audio_path: &std::path::Path) -> Result<String, String> {
    let cfg = get();
    let (endpoint, key) = match cfg.provider {
        AiProvider::Openai => (
            "https://api.openai.com/v1/audio/transcriptions".to_string(),
            cfg.openai_key.clone(),
        ),
        AiProvider::Local => {
            if cfg.local_base_url.is_empty() {
                return Err("No local endpoint configured".to_string());
            }
            (
                format!("{}/audio/transcriptions", cfg.local_base_url),
                cfg.openai_key.clone(),
            )
        }
        _ => return Err("Transcription needs an OpenAI-compatible provider".to_string()),
    };

    let bytes = tokio::fs::read(audio_path)
        .await
        .map_err(|e| format!("Read audio failed: {}", e))?;
    let file_name = audio_path
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "audio.mp3".to_string());

    let part = reqwest::multipart::Part::bytes(bytes)
        .file_name(file_name)
        .mime_str("application/octet-stream")
        .map_err(|e| format!("Multipart error: {}", e))?;
    let form = reqwest::multipart::Form::new()
        .text("model", "whisper-1")
        .text("response_format", "text")
        .part("file", part);

    let client = http_client()?;
    let mut req = client.post(&endpoint).multipart(form);
    if !key.is_empty() {
        req = req.bearer_auth(&key);
    }
    let resp = req
        .send()
        .await
        .map_err(|e| format!("Request failed: {}", e))?;
    let status = resp.status();
    let text = resp
        .text()
        .await
        .map_err(|e| format!("Read body failed: {}", e))?;
    if !status.is_success() {
        return Err(format!("Transcription error ({})", status.as_u16()));
    }
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return Err("Empty transcription".to_string());
    }
    Ok(trimmed.to_string())
}

const AI_HISTORY_FILE: &str = "ai_history.json";
const MAX_HISTORY: usize = 100;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AiHistoryEntry {
    pub id: u64,
    pub kind: String,
    pub url: String,
    pub title: String,
    pub content: String,
    pub created_at_ms: u64,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
struct AiHistoryFile {
    #[serde(default)]
    entries: Vec<AiHistoryEntry>,
}

fn history_path() -> Option<std::path::PathBuf> {
    crate::core::paths::app_data_dir().map(|d| d.join(AI_HISTORY_FILE))
}

pub fn history_list() -> Vec<AiHistoryEntry> {
    let Some(path) = history_path() else {
        return Vec::new();
    };
    match std::fs::read_to_string(&path) {
        Ok(c) => serde_json::from_str::<AiHistoryFile>(&c)
            .map(|f| f.entries)
            .unwrap_or_default(),
        Err(_) => Vec::new(),
    }
}

pub fn history_add(kind: &str, url: &str, title: &str, content: &str) {
    let Some(path) = history_path() else { return };
    let Some(parent) = path.parent() else { return };
    let _ = std::fs::create_dir_all(parent);
    let mut entries = history_list();
    let id = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0);
    entries.push(AiHistoryEntry {
        id,
        kind: kind.to_string(),
        url: url.to_string(),
        title: title.to_string(),
        content: content.to_string(),
        created_at_ms: id,
    });
    if entries.len() > MAX_HISTORY {
        let overflow = entries.len() - MAX_HISTORY;
        entries.drain(0..overflow);
    }
    if let Ok(s) = serde_json::to_string_pretty(&AiHistoryFile { entries }) {
        let tmp = path.with_extension("json.tmp");
        let ok = (|| -> std::io::Result<()> {
            use std::io::Write;
            let mut f = std::fs::File::create(&tmp)?;
            f.write_all(s.as_bytes())?;
            f.sync_all()?;
            Ok(())
        })()
        .is_ok();
        if ok {
            let _ = std::fs::rename(&tmp, &path);
        } else {
            let _ = std::fs::remove_file(&tmp);
        }
    }
}

pub fn history_clear() {
    if let Some(path) = history_path() {
        let _ = std::fs::remove_file(path);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn view_hides_keys() {
        let cfg = AiConfig {
            provider: AiProvider::Openai,
            kind: "openai".to_string(),
            openai_key: "secret".to_string(),
            anthropic_key: String::new(),
            local_base_url: String::new(),
            model: "m".to_string(),
        };
        let v = cfg.view();
        assert!(v.has_openai_key);
        assert!(!v.has_anthropic_key);
        assert_eq!(v.kind, "openai");
        let json = serde_json::to_string(&v).unwrap();
        assert!(!json.contains("secret"));
    }

    #[test]
    fn is_configured_logic() {
        let mut cfg = AiConfig::default();
        assert!(!cfg.is_configured());
        cfg.provider = AiProvider::Anthropic;
        assert!(!cfg.is_configured());
        cfg.anthropic_key = "k".to_string();
        assert!(cfg.is_configured());
    }

    #[test]
    fn a_kindless_local_config_still_counts_as_configured() {
        // Configuração antiga (antes do `kind`): só o endereço, sem chave.
        let cfg = AiConfig {
            provider: AiProvider::Local,
            kind: String::new(),
            local_base_url: "http://localhost:11434/v1".to_string(),
            model: "llama3".to_string(),
            ..AiConfig::default()
        };
        assert!(cfg.is_configured());
    }

    #[test]
    fn a_table_provider_that_needs_a_key_asks_for_one() {
        let mut cfg = AiConfig {
            provider: AiProvider::Local,
            kind: "deepseek".to_string(),
            local_base_url: "https://api.deepseek.com".to_string(),
            ..AiConfig::default()
        };
        assert!(!cfg.is_configured());
        cfg.openai_key = "sk-x".to_string();
        assert!(cfg.is_configured());

        // Ollama roda na máquina do usuário: chave não é obrigatória.
        let ollama = AiConfig {
            provider: AiProvider::Local,
            kind: "ollama".to_string(),
            local_base_url: "http://localhost:11434/v1".to_string(),
            ..AiConfig::default()
        };
        assert!(ollama.is_configured());

        // Provedor fora da tabela é desconhecido: pede chave.
        let unknown = AiConfig {
            provider: AiProvider::Local,
            kind: "provedor-novo".to_string(),
            local_base_url: "https://exemplo.com/v1".to_string(),
            ..AiConfig::default()
        };
        assert!(!unknown.is_configured());
    }

    #[test]
    fn the_kind_decides_the_route_and_the_provider() {
        let mut cfg = AiConfig::default();
        apply_kind(
            &mut cfg,
            "deepseek",
            "deepseek-chat".to_string(),
            String::new(),
            KeyAction::Set("sk-d"),
        );
        assert_eq!(cfg.provider, AiProvider::Local);
        assert_eq!(cfg.kind, "deepseek");
        assert_eq!(cfg.local_base_url, "https://api.deepseek.com");
        assert_eq!(cfg.openai_key, "sk-d");

        apply_kind(
            &mut cfg,
            "anthropic",
            "claude-3-5-sonnet".to_string(),
            String::new(),
            KeyAction::Set("sk-a"),
        );
        assert_eq!(cfg.provider, AiProvider::Anthropic);
        assert_eq!(cfg.anthropic_key, "sk-a");
        // Provedor da OpenAI não guarda endereço.
        assert!(cfg.local_base_url.is_empty());

        apply_kind(
            &mut cfg,
            "gemini",
            "gemini-2.0-flash".to_string(),
            String::new(),
            KeyAction::Keep,
        );
        assert_eq!(
            cfg.local_base_url,
            "https://generativelanguage.googleapis.com/v1beta/openai"
        );
        // O endereço digitado pelo usuário vence o da tabela.
        apply_kind(
            &mut cfg,
            "custom",
            "meu-modelo".to_string(),
            "http://localhost:1234/v1".to_string(),
            KeyAction::Keep,
        );
        assert_eq!(cfg.local_base_url, "http://localhost:1234/v1");
    }

    #[test]
    fn switching_provider_can_drop_the_previous_credential() {
        let mut cfg = AiConfig {
            provider: AiProvider::Anthropic,
            kind: "anthropic".to_string(),
            anthropic_key: "sk-ant".to_string(),
            ..AiConfig::default()
        };
        apply_kind(
            &mut cfg,
            "openai",
            "gpt-4o-mini".to_string(),
            String::new(),
            KeyAction::Clear,
        );
        assert!(cfg.openai_key.is_empty());
        assert!(cfg.anthropic_key.is_empty());
        assert_eq!(cfg.provider, AiProvider::Openai);
    }

    #[test]
    fn the_store_keeps_the_credential_where_it_belongs() {
        // Passa pelo mesmo caminho da tela de IA (store + arquivo), só que numa
        // pasta temporária: é o teste que pega troca de provedor vazando a chave.
        let dir = std::env::temp_dir().join(format!("omniget-ai-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        TEST_DIR.with(|d| *d.borrow_mut() = Some(dir.clone()));

        let saved = set_with_kind(
            "deepseek",
            "deepseek-chat".to_string(),
            String::new(),
            KeyAction::Set("sk-deepseek"),
        );
        assert_eq!(saved.provider, AiProvider::Local);
        assert_eq!(saved.kind, "deepseek");
        assert_eq!(saved.local_base_url, "https://api.deepseek.com");

        // O que ficou gravado é o que uma próxima execução vai ler.
        let on_disk: AiConfig = serde_json::from_str(
            &std::fs::read_to_string(dir.join(AI_CONFIG_FILE)).expect("config escrita"),
        )
        .unwrap();
        assert_eq!(on_disk.kind, "deepseek");
        assert_eq!(on_disk.model, "deepseek-chat");
        assert!(on_disk.is_configured());

        // Trocar de provedor com o campo de chave vazio apaga a credencial antiga.
        let switched = set_with_kind(
            "openai",
            "gpt-4o-mini".to_string(),
            String::new(),
            KeyAction::Clear,
        );
        assert_eq!(switched.provider, AiProvider::Openai);
        assert!(switched.openai_key.is_empty());
        assert!(switched.anthropic_key.is_empty());

        // "Desligar" tira o provedor e o tipo.
        let off = clear();
        assert_eq!(off.provider, AiProvider::None);
        assert!(off.kind.is_empty());
        assert!(!get().is_configured());

        TEST_DIR.with(|d| *d.borrow_mut() = None);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
