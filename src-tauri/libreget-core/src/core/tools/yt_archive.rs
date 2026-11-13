//! Arquivar canal, playlist ou Watch Later — e continuar de onde parou.
//!
//! O `--download-archive` do yt-dlp já evita rebaixar o que existe, mas ele é
//! só uma lista de IDs: não sabe dizer o que falhou nem por quê. Aqui mora um
//! estado próprio ao lado do destino (`.omniget-archive.json`), com um status
//! por item, para a UI mostrar a fila e para o app poder ser fechado no meio.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use anyhow::anyhow;
use serde::{Deserialize, Serialize};

pub const STATE_FILE: &str = ".omniget-archive.json";
pub const ARCHIVE_FILE: &str = ".omniget-archive.txt";
const STATE_VERSION: u32 = 1;
const ID: &str = "yt-archive";
/// Prefixo do `--print` para separar o caminho final das linhas de progresso.
const FILE_MARK: &str = "__omniget_file__";

// ── yt-dlp como processo ───────────────────────────────────────────────

/// Cookies da sessão em arquivo Netscape temporário, apagado no fim.
pub struct CookieFile {
    path: Option<PathBuf>,
}

impl CookieFile {
    pub fn new(session: Option<&str>) -> anyhow::Result<Self> {
        let Some(content) = session.filter(|c| !c.trim().is_empty()) else {
            return Ok(Self { path: None });
        };
        let path = super::temp_dir().join(format!("yt-cookies-{}.txt", uuid::Uuid::new_v4()));
        std::fs::write(&path, content.as_bytes())?;
        Ok(Self { path: Some(path) })
    }

    pub fn push_args(&self, args: &mut Vec<String>) {
        if let Some(p) = &self.path {
            args.push("--cookies".to_string());
            args.push(p.to_string_lossy().to_string());
        }
    }
}

impl Drop for CookieFile {
    fn drop(&mut self) {
        if let Some(p) = &self.path {
            let _ = std::fs::remove_file(p);
        }
    }
}

async fn ytdlp_path() -> anyhow::Result<PathBuf> {
    crate::core::dependencies::find_tool("yt-dlp")
        .await
        .ok_or_else(|| anyhow!("o yt-dlp não está instalado"))
}

/// Roda o yt-dlp e devolve a saída padrão. Erro traz o fim do stderr.
pub async fn run_ytdlp(args: &[String]) -> anyhow::Result<String> {
    let bin = ytdlp_path().await?;
    let out = crate::core::ytdlp::ytdlp_command(&bin)
        .args(args)
        .stdin(std::process::Stdio::null())
        .output()
        .await
        .map_err(|e| anyhow!("o yt-dlp não iniciou: {}", e))?;
    if !out.status.success() {
        let err = String::from_utf8_lossy(&out.stderr);
        let tail: Vec<&str> = err.lines().rev().take(3).collect();
        return Err(anyhow!(
            "yt-dlp falhou: {}",
            tail.into_iter().rev().collect::<Vec<_>>().join(" / ")
        ));
    }
    Ok(String::from_utf8_lossy(&out.stdout).to_string())
}

// ── Estado ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ItemStatus {
    /// Ainda na fila.
    Pending,
    /// Baixado (ou já presente no `--download-archive`).
    Ok,
    /// Tentou e deu erro; o motivo fica no item.
    Failed,
    /// Privado, removido ou fora do recorte pedido — não adianta tentar.
    Skipped,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ArchiveItem {
    pub id: String,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub url: String,
    #[serde(default)]
    pub duration_seconds: f64,
    pub status: ItemStatus,
    #[serde(default)]
    pub error: Option<String>,
    #[serde(default)]
    pub file: Option<String>,
    #[serde(default)]
    pub attempts: u32,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct Counts {
    pub total: usize,
    pub done: usize,
    pub failed: usize,
    pub skipped: usize,
    pub pending: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArchiveState {
    #[serde(default)]
    pub version: u32,
    #[serde(default)]
    pub source_url: String,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub updated: String,
    #[serde(default)]
    pub items: Vec<ArchiveItem>,
}

impl ArchiveState {
    pub fn new(source_url: &str, title: &str) -> Self {
        Self {
            version: STATE_VERSION,
            source_url: source_url.to_string(),
            title: title.to_string(),
            updated: String::new(),
            items: Vec::new(),
        }
    }

    pub fn counts(&self) -> Counts {
        let mut c = Counts {
            total: self.items.len(),
            ..Default::default()
        };
        for it in &self.items {
            match it.status {
                ItemStatus::Ok => c.done += 1,
                ItemStatus::Failed => c.failed += 1,
                ItemStatus::Skipped => c.skipped += 1,
                ItemStatus::Pending => c.pending += 1,
            }
        }
        c
    }

    /// Junta o que a enumeração trouxe sem perder o que já foi feito:
    /// item conhecido mantém o status, item novo entra pendente. Devolve
    /// quantos entraram agora.
    pub fn merge(&mut self, title: &str, fresh: Vec<ArchiveItem>, retry_failed: bool) -> usize {
        if !title.trim().is_empty() {
            self.title = title.trim().to_string();
        }
        let mut added = 0;
        for f in fresh {
            match self.items.iter_mut().find(|i| i.id == f.id) {
                Some(old) => {
                    if !f.title.is_empty() {
                        old.title.clone_from(&f.title);
                    }
                    if !f.url.is_empty() {
                        old.url.clone_from(&f.url);
                    }
                    if f.duration_seconds > 0.0 {
                        old.duration_seconds = f.duration_seconds;
                    }
                    // Vídeo que virou privado desde a última varredura sai da
                    // fila; o que voltou a existir volta para ela.
                    if f.status == ItemStatus::Skipped && old.status == ItemStatus::Pending {
                        old.status = ItemStatus::Skipped;
                        old.error.clone_from(&f.error);
                    } else if old.status == ItemStatus::Skipped && f.status == ItemStatus::Pending {
                        old.status = ItemStatus::Pending;
                        old.error = None;
                    }
                }
                None => {
                    self.items.push(f);
                    added += 1;
                }
            }
        }
        if retry_failed {
            for it in self.items.iter_mut() {
                if it.status == ItemStatus::Failed {
                    it.status = ItemStatus::Pending;
                    it.error = None;
                }
            }
        }
        added
    }

    pub fn next_pending(&self) -> Option<usize> {
        self.items
            .iter()
            .position(|i| i.status == ItemStatus::Pending)
    }

    pub fn mark(
        &mut self,
        id: &str,
        status: ItemStatus,
        file: Option<String>,
        error: Option<String>,
    ) {
        if let Some(it) = self.items.iter_mut().find(|i| i.id == id) {
            it.status = status;
            it.attempts = it.attempts.saturating_add(1);
            it.file = file;
            it.error = error;
        }
    }
}

pub fn state_path(dest: &Path) -> PathBuf {
    dest.join(STATE_FILE)
}

pub fn load_state(dest: &Path) -> Option<ArchiveState> {
    let text = std::fs::read_to_string(state_path(dest)).ok()?;
    serde_json::from_str::<ArchiveState>(&text).ok()
}

/// Grava no `.tmp` e renomeia: fechar o app no meio da escrita não deixa um
/// estado pela metade.
pub fn save_state(dest: &Path, state: &mut ArchiveState) -> anyhow::Result<PathBuf> {
    std::fs::create_dir_all(dest)?;
    state.version = STATE_VERSION;
    state.updated = chrono::Local::now().to_rfc3339();
    let final_path = state_path(dest);
    let tmp = final_path.with_extension("json.tmp");
    std::fs::write(&tmp, serde_json::to_vec_pretty(state)?)?;
    std::fs::rename(&tmp, &final_path)?;
    Ok(final_path)
}

// ── Enumeração ─────────────────────────────────────────────────────────

fn unavailable_reason(title: &str) -> Option<&'static str> {
    let t = title.trim().to_lowercase();
    if t.contains("[private video]") || t == "private video" {
        return Some("vídeo privado");
    }
    if t.contains("[deleted video]") || t == "deleted video" {
        return Some("vídeo apagado");
    }
    if t.contains("[unavailable video]") {
        return Some("vídeo indisponível");
    }
    None
}

fn collect_entries(v: &serde_json::Value, out: &mut Vec<ArchiveItem>) {
    if let Some(entries) = v.get("entries").and_then(|e| e.as_array()) {
        for e in entries {
            collect_entries(e, out);
        }
        return;
    }
    let kind = v.get("_type").and_then(|t| t.as_str()).unwrap_or("video");
    if kind == "playlist" {
        return;
    }
    let id = v.get("id").and_then(|i| i.as_str()).unwrap_or("").trim();
    if id.is_empty() {
        return;
    }
    let title = v
        .get("title")
        .and_then(|t| t.as_str())
        .unwrap_or("")
        .to_string();
    let url = v
        .get("url")
        .and_then(|u| u.as_str())
        .or_else(|| v.get("webpage_url").and_then(|u| u.as_str()))
        .map(|s| s.to_string())
        .unwrap_or_else(|| format!("https://www.youtube.com/watch?v={}", id));
    let reason = unavailable_reason(&title);
    out.push(ArchiveItem {
        id: id.to_string(),
        title,
        url,
        duration_seconds: v.get("duration").and_then(|d| d.as_f64()).unwrap_or(0.0),
        status: if reason.is_some() {
            ItemStatus::Skipped
        } else {
            ItemStatus::Pending
        },
        error: reason.map(|r| r.to_string()),
        file: None,
        attempts: 0,
    });
}

/// Lê o `-J --flat-playlist`: devolve o título da coleção e os itens, sem
/// repetição, na ordem em que o YouTube devolveu.
pub fn parse_flat_playlist(json: &str) -> anyhow::Result<(String, Vec<ArchiveItem>)> {
    let v: serde_json::Value = serde_json::from_str(json)?;
    let title = v
        .get("title")
        .and_then(|t| t.as_str())
        .or_else(|| v.get("channel").and_then(|t| t.as_str()))
        .unwrap_or("")
        .to_string();
    let mut items = Vec::new();
    collect_entries(&v, &mut items);
    // Uma URL de vídeo único também vale como arquivo de um item só.
    if items.is_empty() {
        collect_entries(
            &serde_json::json!({ "_type": "video", "id": v.get("id"), "title": v.get("title"),
                                 "webpage_url": v.get("webpage_url"), "duration": v.get("duration") }),
            &mut items,
        );
    }
    let mut seen = std::collections::HashSet::new();
    items.retain(|i| seen.insert(i.id.clone()));
    Ok((title, items))
}

// ── Progresso do download ──────────────────────────────────────────────

/// `[download]  12.3% of ...`
pub fn parse_percent(line: &str) -> Option<f64> {
    let rest = line.trim().strip_prefix("[download]")?;
    let token = rest.split_whitespace().next()?;
    token.strip_suffix('%')?.parse::<f64>().ok()
}

/// Linha do `--print` com o arquivo final.
pub fn parse_printed_file(line: &str) -> Option<String> {
    let idx = line.find(FILE_MARK)?;
    let path = line[idx + FILE_MARK.len()..].trim();
    if path.is_empty() {
        None
    } else {
        Some(path.to_string())
    }
}

// ── Opções e execução ──────────────────────────────────────────────────

#[derive(Debug, Clone, Deserialize)]
pub struct Options {
    /// Canal, playlist, `WL` (Watch Later) ou vídeo solto.
    pub url: String,
    /// Pasta do arquivo; o estado mora nela.
    pub dest: String,
    /// "best" | "1080" | "720" | "480" | "audio"
    #[serde(default = "default_quality")]
    pub quality: String,
    #[serde(default)]
    pub write_subs: bool,
    #[serde(default)]
    pub write_thumbnail: bool,
    #[serde(default)]
    pub write_info_json: bool,
    /// Teto de itens por rodada (0 = tudo o que estiver pendente).
    #[serde(default)]
    pub max_items: u32,
    /// Reenumerar a coleção mesmo já tendo estado gravado.
    #[serde(default)]
    pub refresh: bool,
    /// Devolver à fila o que falhou antes.
    #[serde(default)]
    pub retry_failed: bool,
    #[serde(default)]
    pub account_slug: Option<String>,
    #[serde(default)]
    pub session_netscape: Option<String>,
    /// Idiomas de legenda no formato do `--sub-langs` (padrão "pt.*,en.*").
    #[serde(default)]
    pub sub_langs: Option<String>,
    /// Para antes do próximo item quando a pasta passar disso (0 = sem teto).
    #[serde(default)]
    pub max_bytes: u64,
    /// Pausa antes de cada download, contra o limite de taxa do YouTube.
    #[serde(default)]
    pub sleep_seconds: u32,
}

fn default_quality() -> String {
    "best".to_string()
}

/// Tradução da qualidade escolhida para o seletor do yt-dlp. Qualquer altura
/// numérica vale (360, 240...); entre formatos da mesma altura o H.264/AAC
/// ganha, porque o MP4 final toca em qualquer lugar e decodifica rápido.
pub fn format_args(quality: &str) -> Vec<String> {
    let height = quality.parse::<u32>().ok().filter(|h| *h > 0);
    match (quality, height) {
        ("audio", _) => vec![
            "-f".into(),
            "bestaudio/best".into(),
            "-x".into(),
            "--audio-format".into(),
            "mp3".into(),
        ],
        // `bv` (só vídeo) e não `bv*`: com `bv*` o sort elege o progressivo
        // 18 (360p h264+aac num arquivo só), que o YouTube estrangula por IP
        // após um volume grande (500 KB em 75 s contra 2,5 MiB/s do DASH 134).
        (_, Some(h)) => vec![
            "-f".into(),
            format!("bv[height<={h}]+ba/b[height<={h}]/bv*+ba/b"),
            "-S".into(),
            format!("res:{h},vcodec:h264,acodec:aac"),
            "--merge-output-format".into(),
            "mp4".into(),
        ],
        _ => vec![
            "-f".into(),
            "bv*+ba/b".into(),
            "--merge-output-format".into(),
            "mp4".into(),
        ],
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct ArchiveResult {
    pub dest: String,
    pub state_path: String,
    pub title: String,
    pub source_url: String,
    pub total: usize,
    pub done: usize,
    pub failed: usize,
    pub skipped: usize,
    pub pending: usize,
    pub items: Vec<ArchiveItem>,
    pub used_session: bool,
    pub cancelled: bool,
    /// A rodada parou porque a pasta atingiu `max_bytes`.
    pub cap_reached: bool,
    /// A rodada parou porque o YouTube seguiu barrando o IP como robô depois
    /// de todas as pausas; os itens continuam pendentes.
    pub blocked: bool,
}

fn result_of(
    dest: &Path,
    state: &ArchiveState,
    used_session: bool,
    cancelled: bool,
) -> ArchiveResult {
    let c = state.counts();
    ArchiveResult {
        dest: dest.to_string_lossy().to_string(),
        state_path: state_path(dest).to_string_lossy().to_string(),
        title: state.title.clone(),
        source_url: state.source_url.clone(),
        total: c.total,
        done: c.done,
        failed: c.failed,
        skipped: c.skipped,
        pending: c.pending,
        items: state.items.clone(),
        used_session,
        cancelled,
        cap_reached: false,
        blocked: false,
    }
}

static CANCEL: AtomicBool = AtomicBool::new(false);

/// Pede parada depois do item que estiver rodando.
pub fn cancel() {
    CANCEL.store(true, Ordering::SeqCst);
}

fn has_session(opts: &Options) -> bool {
    opts.session_netscape
        .as_ref()
        .map(|s| !s.trim().is_empty())
        .unwrap_or(false)
}

/// Lê o estado gravado sem tocar na rede — é o que a tela mostra ao abrir.
pub fn state(dest: &str) -> anyhow::Result<ArchiveResult> {
    let dir = PathBuf::from(dest.trim());
    let st = load_state(&dir).ok_or_else(|| anyhow!("não há arquivo em andamento nessa pasta"))?;
    Ok(result_of(&dir, &st, false, false))
}

/// Enumera a coleção e funde com o estado, sem baixar nada.
pub async fn scan(opts: &Options, progress: &super::ProgressFn) -> anyhow::Result<ArchiveResult> {
    let dest = PathBuf::from(opts.dest.trim());
    if opts.dest.trim().is_empty() {
        return Err(anyhow!("escolha a pasta do arquivo"));
    }
    std::fs::create_dir_all(&dest)?;
    let mut st = enumerate(opts, &dest, progress).await?;
    save_state(&dest, &mut st)?;
    Ok(result_of(&dest, &st, has_session(opts), false))
}

async fn enumerate(
    opts: &Options,
    dest: &Path,
    progress: &super::ProgressFn,
) -> anyhow::Result<ArchiveState> {
    super::report(
        progress,
        ID,
        "progress",
        0,
        None,
        Some("lendo a coleção".into()),
    );
    let mut args = vec![
        "-J".to_string(),
        "--flat-playlist".to_string(),
        "--no-warnings".to_string(),
        "--ignore-errors".to_string(),
    ];
    let cookie = CookieFile::new(opts.session_netscape.as_deref())?;
    cookie.push_args(&mut args);
    args.push(opts.url.trim().to_string());
    let json = run_ytdlp(&args).await?;
    let (title, items) = parse_flat_playlist(&json)?;
    if items.is_empty() {
        return Err(anyhow!(
            "não achei nenhum vídeo nessa URL (Watch Later precisa da sessão do YouTube)"
        ));
    }
    let mut st = load_state(dest).unwrap_or_else(|| ArchiveState::new(opts.url.trim(), &title));
    st.source_url = opts.url.trim().to_string();
    st.merge(&title, items, opts.retry_failed);
    Ok(st)
}

pub async fn run(opts: Options, progress: super::ProgressFn) -> anyhow::Result<ArchiveResult> {
    if opts.dest.trim().is_empty() {
        return Err(anyhow!("escolha a pasta do arquivo"));
    }
    let dest = PathBuf::from(opts.dest.trim());
    std::fs::create_dir_all(&dest)?;
    CANCEL.store(false, Ordering::SeqCst);

    let mut st = open_state(&opts, &dest, &progress).await?;
    let bin = ytdlp_path().await?;
    let cookie = CookieFile::new(opts.session_netscape.as_deref())?;
    let archive_txt = dest.join(ARCHIVE_FILE);
    let total = st.items.len() as u64;
    let limit = match opts.max_items {
        0 => usize::MAX,
        n => n as usize,
    };

    let mut rodados = 0usize;
    let mut waits = 0u32;
    let mut cancelled = false;
    let mut blocked = false;
    let mut cap_reached = false;
    while rodados < limit {
        if CANCEL.load(Ordering::SeqCst) {
            cancelled = true;
            break;
        }
        if over_cap(&dest, opts.max_bytes) {
            cap_reached = true;
            break;
        }
        let Some(idx) = st.next_pending() else { break };
        let item = st.items[idx].clone();
        let feitos = st.counts().done as u64;
        super::report(
            &progress,
            ID,
            "progress",
            feitos,
            Some(total),
            Some(item.title.clone()),
        );
        match download_one(&bin, &opts, &cookie, &archive_txt, &dest, &item, &progress).await {
            Ok(file) => st.mark(&item.id, ItemStatus::Ok, file, None),
            // Cancelado no meio (o yt-dlp é encerrado em `wait_or_cancel`): o
            // item foi interrompido, não falhou, e continua na fila.
            Err(_) if CANCEL.load(Ordering::SeqCst) => {
                cancelled = true;
                break;
            }
            Err(e) => match on_error(&e.to_string(), &mut waits, &progress, feitos, total).await {
                OnError::Retry => continue,
                OnError::Cancelled => {
                    cancelled = true;
                    break;
                }
                OnError::Blocked => {
                    blocked = true;
                    break;
                }
                OnError::Fail => {
                    tracing::warn!("[yt-archive] {}: {}", item.id, e);
                    st.mark(&item.id, ItemStatus::Failed, None, Some(e.to_string()));
                }
            },
        }
        save_state(&dest, &mut st)?;
        waits = 0;
        rodados += 1;
    }

    let c = st.counts();
    super::report(
        &progress,
        ID,
        if cancelled || blocked {
            "progress"
        } else {
            "done"
        },
        c.done as u64,
        Some(total),
        None,
    );
    let mut result = result_of(&dest, &st, has_session(&opts), cancelled);
    result.cap_reached = cap_reached;
    result.blocked = blocked;
    Ok(result)
}

/// O que a rodada faz com o item depois de um erro do yt-dlp.
#[derive(Debug, PartialEq)]
enum OnError {
    Retry,
    Fail,
    Cancelled,
    Blocked,
}

/// Erros da máquina ou do IP (sem rede; YouTube barrando o IP como robô) não
/// são do vídeo: todo item falharia em segundos e a fila inteira seria
/// queimada. O item fica pendente e a rodada espera antes de repeti-lo.
/// `waits` conta as pausas seguidas no mesmo item.
async fn on_error(
    msg: &str,
    waits: &mut u32,
    progress: &super::ProgressFn,
    done: u64,
    total: u64,
) -> OnError {
    let n = *waits as usize;
    let resumed = if is_offline_error(msg) && *waits < MAX_OFFLINE_WAITS {
        wait_online(progress, done, total).await
    } else if is_blocked_error(msg) {
        let Some(mins) = BLOCK_WAIT_MINS.get(n) else {
            return OnError::Blocked;
        };
        wait_unblocked(*mins, progress, done, total).await
    } else {
        return OnError::Fail;
    };
    *waits += 1;
    if resumed {
        OnError::Retry
    } else {
        OnError::Cancelled
    }
}

/// Fila salva (com as falhas devolvidas, se pedido) ou enumerada do zero.
async fn open_state(
    opts: &Options,
    dest: &Path,
    progress: &super::ProgressFn,
) -> anyhow::Result<ArchiveState> {
    let mut st = match load_state(dest) {
        Some(mut st) if !opts.refresh => {
            if opts.retry_failed {
                st.merge("", Vec::new(), true);
            }
            st
        }
        _ => enumerate(opts, dest, progress).await?,
    };
    save_state(dest, &mut st)?;
    Ok(st)
}

/// Esperas seguidas pela rede no mesmo item antes de tratá-lo como falha
/// comum; evita laço infinito se um item der erro de DNS com a rede de pé.
const MAX_OFFLINE_WAITS: u32 = 5;
const ONLINE_PROBE: &str = "www.youtube.com:443";
const OFFLINE_POLL_SECS: u64 = 30;

/// Erros da rede da máquina (DNS, rota), não do vídeo, em macOS, Linux e
/// Windows.
fn is_offline_error(msg: &str) -> bool {
    const SINAIS: [&str; 6] = [
        "nodename nor servname provided",
        "Temporary failure in name resolution",
        "Name or service not known",
        "getaddrinfo failed",
        "Network is unreachable",
        "No route to host",
    ];
    SINAIS.iter().any(|s| msg.contains(s))
}

/// Pausas, em minutos, antes de repetir um item barrado como robô. O bloqueio
/// é do IP e costuma durar horas; somadas dão ~12 h, depois a rodada para.
const BLOCK_WAIT_MINS: [u64; 6] = [15, 30, 60, 120, 240, 240];

/// O YouTube pediu login "para confirmar que você não é um robô": vale para
/// o IP inteiro, não para o vídeo (o pedido de idade é outro erro, do vídeo).
fn is_blocked_error(msg: &str) -> bool {
    msg.contains("confirm you") && msg.contains("not a bot")
}

/// Espera o DNS do YouTube voltar a resolver. Devolve `false` se a rodada foi
/// cancelada durante a espera.
async fn wait_online(progress: &super::ProgressFn, done: u64, total: u64) -> bool {
    let msg = "sem conexão: aguardando a rede voltar".to_string();
    super::report(progress, ID, "progress", done, Some(total), Some(msg));
    loop {
        if !sleep_or_cancel(OFFLINE_POLL_SECS).await {
            return false;
        }
        if tokio::net::lookup_host(ONLINE_PROBE).await.is_ok() {
            return true;
        }
    }
}

/// Pausa antes de repetir um item barrado como robô. Devolve `false` se a
/// rodada foi cancelada durante a pausa.
async fn wait_unblocked(mins: u64, progress: &super::ProgressFn, done: u64, total: u64) -> bool {
    let msg = format!("YouTube barrou o IP como robô: nova tentativa em {mins} min");
    super::report(progress, ID, "progress", done, Some(total), Some(msg));
    sleep_or_cancel(mins * 60).await
}

/// Dorme `secs` segundos olhando o cancelamento a cada segundo.
async fn sleep_or_cancel(secs: u64) -> bool {
    for _ in 0..secs {
        if CANCEL.load(Ordering::SeqCst) {
            return false;
        }
        tokio::time::sleep(Duration::from_secs(1)).await;
    }
    !CANCEL.load(Ordering::SeqCst)
}

fn over_cap(dest: &Path, max_bytes: u64) -> bool {
    max_bytes > 0 && super::yt_dataset::dir_size(dest) >= max_bytes
}

/// Legendas e pausa entre downloads, conforme as opções.
fn extra_args(opts: &Options) -> Vec<String> {
    let mut args = Vec::new();
    if opts.write_subs {
        let langs = opts
            .sub_langs
            .as_deref()
            .map(str::trim)
            .filter(|l| !l.is_empty())
            .unwrap_or("pt.*,en.*");
        // `--ignore-errors` rebaixa erro de legenda (o 429 do YouTube é
        // comum) a aviso: o vídeo é o que importa. Erro no vídeo continua
        // saindo com código 1 e o item fica como falha.
        args.extend(
            [
                "--write-subs",
                "--write-auto-subs",
                "--sub-langs",
                langs,
                "--convert-subs",
                "srt",
                "--ignore-errors",
            ]
            .map(String::from),
        );
    }
    if opts.sleep_seconds > 0 {
        args.extend([
            "--sleep-interval".to_string(),
            opts.sleep_seconds.to_string(),
            "--max-sleep-interval".to_string(),
            (opts.sleep_seconds * 2).to_string(),
        ]);
    }
    args
}

#[allow(clippy::too_many_arguments)]
async fn download_one(
    bin: &Path,
    opts: &Options,
    cookie: &CookieFile,
    archive_txt: &Path,
    dest: &Path,
    item: &ArchiveItem,
    progress: &super::ProgressFn,
) -> anyhow::Result<Option<String>> {
    use tokio::io::{AsyncBufReadExt, BufReader};

    let template = dest.join("%(uploader)s/%(title).120B [%(id)s].%(ext)s");
    let mut args = format_args(&opts.quality);
    args.extend([
        "--no-playlist".to_string(),
        "--newline".to_string(),
        "--no-warnings".to_string(),
        "--no-simulate".to_string(),
        "--download-archive".to_string(),
        archive_txt.to_string_lossy().to_string(),
        "--print".to_string(),
        format!("after_move:{}%(filepath)s", FILE_MARK),
        "-o".to_string(),
        template.to_string_lossy().to_string(),
    ]);
    args.extend(extra_args(opts));
    if opts.write_thumbnail {
        args.push("--write-thumbnail".to_string());
    }
    if opts.write_info_json {
        args.push("--write-info-json".to_string());
    }
    cookie.push_args(&mut args);
    args.push(item.url.clone());

    let mut child = crate::core::ytdlp::ytdlp_command(bin)
        .args(&args)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| anyhow!("o yt-dlp não iniciou: {}", e))?;

    let stdout = child.stdout.take();
    let stderr = child.stderr.take();
    let p = progress.clone();
    let titulo = item.title.clone();
    let out_task = tokio::spawn(async move {
        let mut file: Option<String> = None;
        if let Some(o) = stdout {
            let mut lines = BufReader::new(o).lines();
            while let Ok(Some(line)) = lines.next_line().await {
                if let Some(f) = parse_printed_file(&line) {
                    file = Some(f);
                } else if let Some(pc) = parse_percent(&line) {
                    super::report(
                        &p,
                        ID,
                        "item",
                        pc.round() as u64,
                        Some(100),
                        Some(titulo.clone()),
                    );
                }
            }
        }
        file
    });
    let err_task = tokio::spawn(async move {
        let mut tail: Vec<String> = Vec::new();
        if let Some(e) = stderr {
            let mut lines = BufReader::new(e).lines();
            while let Ok(Some(line)) = lines.next_line().await {
                if tail.len() == 3 {
                    tail.remove(0);
                }
                tail.push(line);
            }
        }
        tail.join(" / ")
    });

    let status = wait_or_cancel(&mut child).await?;
    let file = out_task.await.unwrap_or(None);
    let tail = err_task.await.unwrap_or_default();
    if !status.success() {
        return Err(anyhow!(
            "{}",
            if tail.trim().is_empty() {
                "o yt-dlp saiu com erro".to_string()
            } else {
                tail
            }
        ));
    }
    Ok(file)
}

/// Espera o yt-dlp, mas o encerra se a rodada for cancelada. Sem isso um
/// `cancel()` (botão do app ou SIGINT só no processo da CLI) esperava o vídeo
/// inteiro terminar. O `.part` fica e o yt-dlp retoma dele na próxima rodada.
async fn wait_or_cancel(
    child: &mut tokio::process::Child,
) -> anyhow::Result<std::process::ExitStatus> {
    loop {
        if let Some(status) = child.try_wait()? {
            return Ok(status);
        }
        if CANCEL.load(Ordering::SeqCst) {
            let _ = child.kill().await;
            return Err(anyhow!("cancelado"));
        }
        tokio::time::sleep(Duration::from_millis(300)).await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const FLAT: &str = r#"{
      "_type": "playlist",
      "id": "PL123",
      "title": "Minha playlist",
      "entries": [
        {"_type": "url", "id": "aaaaaaaaaaa", "title": "Primeiro", "url": "https://www.youtube.com/watch?v=aaaaaaaaaaa", "duration": 61.0},
        {"_type": "url", "id": "bbbbbbbbbbb", "title": "[Private video]", "url": "https://www.youtube.com/watch?v=bbbbbbbbbbb"},
        {"_type": "url", "id": "ccccccccccc", "title": "Terceiro"}
      ]
    }"#;

    const NESTED: &str = r#"{
      "_type": "playlist",
      "title": "Canal do Fulano",
      "entries": [
        {"_type": "playlist", "title": "Vídeos", "entries": [
          {"_type": "url", "id": "aaaaaaaaaaa", "title": "Um"}
        ]},
        {"_type": "playlist", "title": "Shorts", "entries": [
          {"_type": "url", "id": "ddddddddddd", "title": "Dois"},
          {"_type": "url", "id": "aaaaaaaaaaa", "title": "Um de novo"}
        ]}
      ]
    }"#;

    #[test]
    fn reads_the_flat_playlist() {
        let (title, items) = parse_flat_playlist(FLAT).expect("json válido");
        assert_eq!(title, "Minha playlist");
        assert_eq!(items.len(), 3);
        assert_eq!(items[0].id, "aaaaaaaaaaa");
        assert!((items[0].duration_seconds - 61.0).abs() < 1e-9);
        assert_eq!(
            items[1].status,
            ItemStatus::Skipped,
            "privado não entra na fila"
        );
        assert_eq!(items[1].error.as_deref(), Some("vídeo privado"));
        assert_eq!(
            items[2].url, "https://www.youtube.com/watch?v=ccccccccccc",
            "item sem url ganha uma montada pelo id"
        );
    }

    #[test]
    fn flattens_channel_tabs_and_drops_repeats() {
        let (title, items) = parse_flat_playlist(NESTED).expect("json válido");
        assert_eq!(title, "Canal do Fulano");
        let ids: Vec<&str> = items.iter().map(|i| i.id.as_str()).collect();
        assert_eq!(ids, vec!["aaaaaaaaaaa", "ddddddddddd"]);
    }

    #[test]
    fn a_single_video_is_an_archive_of_one() {
        let json = r#"{"id":"aaaaaaaaaaa","title":"Solo","duration":10.0,
                       "webpage_url":"https://youtu.be/aaaaaaaaaaa"}"#;
        let (_, items) = parse_flat_playlist(json).expect("json válido");
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].url, "https://youtu.be/aaaaaaaaaaa");
    }

    fn estado_com_progresso() -> ArchiveState {
        let (title, items) = parse_flat_playlist(FLAT).expect("json válido");
        let mut st = ArchiveState::new("https://exemplo", &title);
        st.merge(&title, items, false);
        st.mark(
            "aaaaaaaaaaa",
            ItemStatus::Ok,
            Some("/x/um.mp4".into()),
            None,
        );
        st.mark(
            "ccccccccccc",
            ItemStatus::Failed,
            None,
            Some("rede caiu".into()),
        );
        st
    }

    #[test]
    fn counts_add_up() {
        let st = estado_com_progresso();
        assert_eq!(
            st.counts(),
            Counts {
                total: 3,
                done: 1,
                failed: 1,
                skipped: 1,
                pending: 0
            }
        );
        assert_eq!(st.next_pending(), None);
    }

    #[test]
    fn merging_keeps_what_was_already_done_and_queues_the_new() {
        let mut st = estado_com_progresso();
        let novos = r#"{"_type":"playlist","title":"Minha playlist","entries":[
            {"_type":"url","id":"aaaaaaaaaaa","title":"Primeiro"},
            {"_type":"url","id":"bbbbbbbbbbb","title":"[Private video]"},
            {"_type":"url","id":"ccccccccccc","title":"Terceiro"},
            {"_type":"url","id":"eeeeeeeeeee","title":"Recém-postado"}
        ]}"#;
        let (title, items) = parse_flat_playlist(novos).expect("json válido");
        let added = st.merge(&title, items, false);
        assert_eq!(added, 1, "só o vídeo novo entra");
        assert_eq!(st.items.len(), 4);
        assert_eq!(
            st.items[0].status,
            ItemStatus::Ok,
            "o baixado continua baixado"
        );
        assert_eq!(st.items[0].file.as_deref(), Some("/x/um.mp4"));
        assert_eq!(
            st.items[2].status,
            ItemStatus::Failed,
            "falha não vira pendente sozinha"
        );
        assert_eq!(st.next_pending(), Some(3));
    }

    #[test]
    fn retry_puts_the_failures_back_in_the_queue() {
        let mut st = estado_com_progresso();
        st.merge("", Vec::new(), true);
        assert_eq!(st.items[2].status, ItemStatus::Pending);
        assert!(st.items[2].error.is_none());
        assert_eq!(st.items[0].status, ItemStatus::Ok, "o que deu certo fica");
        assert_eq!(
            st.items[1].status,
            ItemStatus::Skipped,
            "privado segue fora"
        );
    }

    #[test]
    fn a_video_that_became_private_leaves_the_queue() {
        let mut st = estado_com_progresso();
        let novos = r#"{"_type":"playlist","entries":[
            {"_type":"url","id":"eeeeeeeeeee","title":"Novo"}]}"#;
        let (_, items) = parse_flat_playlist(novos).expect("json válido");
        st.merge("", items, false);
        assert_eq!(st.next_pending(), Some(3));
        let sumiu = r#"{"_type":"playlist","entries":[
            {"_type":"url","id":"eeeeeeeeeee","title":"[Deleted video]"}]}"#;
        let (_, items) = parse_flat_playlist(sumiu).expect("json válido");
        st.merge("", items, false);
        assert_eq!(st.next_pending(), None);
        assert_eq!(st.items[3].error.as_deref(), Some("vídeo apagado"));
    }

    #[test]
    fn state_survives_a_round_trip_on_disk() {
        let dir =
            super::super::temp_dir().join(format!("yt-archive-teste-{}", uuid::Uuid::new_v4()));
        let mut st = estado_com_progresso();
        let path = save_state(&dir, &mut st).expect("gravou");
        assert!(path.ends_with(STATE_FILE));
        let lido = load_state(&dir).expect("leu de volta");
        assert_eq!(lido.items, st.items);
        assert_eq!(lido.version, STATE_VERSION);
        assert!(!lido.updated.is_empty());
        assert!(
            !dir.join(format!("{}.tmp", STATE_FILE)).exists(),
            "o temporário não pode sobrar"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn no_state_means_nothing_to_resume() {
        let dir =
            super::super::temp_dir().join(format!("yt-archive-vazio-{}", uuid::Uuid::new_v4()));
        assert!(load_state(&dir).is_none());
        assert!(state(&dir.to_string_lossy()).is_err());
    }

    #[test]
    fn download_lines_are_understood() {
        assert_eq!(parse_percent("[download]   3.4% of ~ 12.00MiB"), Some(3.4));
        assert_eq!(parse_percent("[download] 100% of 12.00MiB"), Some(100.0));
        assert_eq!(parse_percent("[youtube] extraindo"), None);
        assert_eq!(
            parse_printed_file(&format!("{}/casa/um.mp4", FILE_MARK)).as_deref(),
            Some("/casa/um.mp4")
        );
        assert_eq!(parse_printed_file("[download] 10%"), None);
    }

    #[test]
    fn quality_maps_to_a_selector() {
        assert!(format_args("audio").contains(&"-x".to_string()));
        assert!(format_args("1080")
            .iter()
            .any(|a| a.contains("height<=1080")));
        assert!(format_args("best").iter().any(|a| a == "bv*+ba/b"));
        let low = format_args("360");
        assert!(low.iter().any(|a| a.starts_with("bv[height<=360]+ba/")));
        assert!(low.iter().any(|a| a == "res:360,vcodec:h264,acodec:aac"));
        assert!(!format_args("0").iter().any(|a| a.contains("height")));
    }

    fn opts_for(dest: &str) -> Options {
        serde_json::from_value(serde_json::json!({ "url": "", "dest": dest }))
            .expect("opções mínimas")
    }

    #[test]
    fn extra_args_follow_subs_and_sleep_options() {
        let mut o = opts_for("x");
        assert!(extra_args(&o).is_empty(), "sem legenda nem pausa");
        o.write_subs = true;
        assert!(extra_args(&o).contains(&"pt.*,en.*".to_string()));
        o.sub_langs = Some(".*-orig,en".into());
        o.sleep_seconds = 5;
        let a = extra_args(&o);
        assert!(a.contains(&".*-orig,en".to_string()));
        assert!(a.contains(&"--ignore-errors".to_string()));
        let i = a
            .iter()
            .position(|x| x == "--max-sleep-interval")
            .expect("pausa");
        assert_eq!(a[i + 1], "10");
    }

    /// Mensagens reais do yt-dlp: as de rede caída pausam a rodada, as do
    /// servidor ou do vídeo continuam virando falha do item.
    #[test]
    fn offline_errors_are_told_apart_from_item_errors() {
        let mac = "ERROR: [youtube] Ye9xvMX9qs8: Unable to download API page: \
                   [Errno 8] nodename nor servname provided, or not known (caused by ...)";
        let linux = "ERROR: [youtube] x: Unable to download webpage: \
                     [Errno -3] Temporary failure in name resolution";
        let win = "ERROR: [youtube] x: Unable to download webpage: \
                   <urlopen error [Errno 11001] getaddrinfo failed>";
        for m in [mac, linux, win] {
            assert!(is_offline_error(m), "{m}");
        }
        for m in [
            "ERROR: \r[download] Got error: Remote end closed connection without response. Giving up after 10 retries",
            "ERROR: \r[download] Got error: HTTP Error 503: Service Unavailable. Giving up after 10 retries",
            "ERROR: [youtube] abc: Video unavailable. This video is private",
            "ERROR: unable to download video data: HTTP Error 429: Too Many Requests",
        ] {
            assert!(!is_offline_error(m), "{m}");
        }
    }

    const BOT: &str = "ERROR: [youtube] azI9TwV0EWk: Sign in to confirm you’re not a bot. \
                       This helps protect our community. Use --cookies-from-browser";

    #[test]
    fn bot_check_is_told_apart_from_video_errors() {
        assert!(is_blocked_error(BOT));
        for m in [
            "ERROR: [youtube] mxfBFUzbDPs: Sign in to confirm your age. This video may be inappropriate",
            "ERROR: unable to download video data: HTTP Error 403: Forbidden",
            "ERROR: [youtube] x: [Errno 8] nodename nor servname provided, or not known",
        ] {
            assert!(!is_blocked_error(m), "{m}");
        }
    }

    /// Erro do vídeo falha o item na hora; bot check depois de todas as pausas
    /// para a rodada sem marcar falha (os dois caminhos não dormem).
    #[tokio::test]
    async fn on_error_fails_the_item_or_stops_the_run() {
        let p = super::super::noop_progress();
        let mut waits = 0;
        let r = on_error("ERROR: HTTP Error 403: Forbidden", &mut waits, &p, 0, 1).await;
        assert_eq!(r, OnError::Fail);
        let mut waits = BLOCK_WAIT_MINS.len() as u32;
        assert_eq!(on_error(BOT, &mut waits, &p, 0, 1).await, OnError::Blocked);
    }

    /// Com o teto já estourado a rodada nem chama o yt-dlp: sai antes do
    /// primeiro item, com a fila intacta.
    #[tokio::test]
    async fn run_stops_before_downloading_when_folder_is_over_cap() {
        let dir = super::super::temp_dir().join(format!("yt-archive-cap-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).expect("pasta");
        std::fs::write(dir.join("big.bin"), vec![0u8; 2048]).expect("arquivo");
        let items = super::super::yt_dataset::parse_seed("{\"id\":\"a1\"}\n{\"id\":\"b2\"}");
        super::super::yt_dataset::seed(&dir, "cap", items).expect("semeia");

        let mut o = opts_for(&dir.to_string_lossy());
        o.max_bytes = 1024;
        let r = run(o, super::super::noop_progress()).await;
        let _ = std::fs::remove_dir_all(&dir);
        let r = match r {
            Ok(r) => r,
            // Máquina sem yt-dlp: o teto não chegou a ser testado aqui.
            Err(e) if e.to_string().contains("yt-dlp") => return,
            Err(e) => panic!("{e}"),
        };
        assert!(r.cap_reached);
        assert_eq!((r.pending, r.done, r.failed), (2, 0, 0));
    }

    #[test]
    fn cookies_only_show_up_when_there_is_a_session() {
        let mut args: Vec<String> = Vec::new();
        let vazio = CookieFile::new(None).expect("sem sessão");
        vazio.push_args(&mut args);
        assert!(args.is_empty());
        let branco = CookieFile::new(Some("   ")).expect("sessão em branco");
        branco.push_args(&mut args);
        assert!(args.is_empty(), "espaço em branco não é sessão");

        let c = CookieFile::new(Some("# Netscape HTTP Cookie File\n")).expect("com sessão");
        c.push_args(&mut args);
        assert_eq!(args.len(), 2);
        assert_eq!(args[0], "--cookies");
        let path = PathBuf::from(&args[1]);
        assert!(
            path.exists(),
            "o arquivo de cookies devia existir enquanto vive"
        );
        drop(c);
        assert!(!path.exists(), "e sumir quando sai de cena");
    }

    /// Baixa um vídeo curto de verdade, confere o estado gravado e roda de
    /// novo para provar que a retomada não rebaixa nada.
    /// `cargo test -p omniget-core --lib -- --ignored live_yt_archive_resumes`
    #[tokio::test]
    #[ignore]
    async fn live_yt_archive_resumes_without_downloading_twice() {
        let dir =
            super::super::temp_dir().join(format!("yt-archive-resume-{}", uuid::Uuid::new_v4()));
        let opts = Options {
            url: "https://www.youtube.com/watch?v=jNQXAC9IVRw".into(),
            dest: dir.to_string_lossy().to_string(),
            quality: "480".into(),
            write_subs: false,
            write_thumbnail: false,
            write_info_json: false,
            max_items: 0,
            refresh: false,
            retry_failed: false,
            account_slug: None,
            session_netscape: None,
            sub_langs: None,
            max_bytes: 0,
            sleep_seconds: 0,
        };
        let r = run(opts.clone(), super::super::noop_progress())
            .await
            .expect("primeira rodada");
        assert_eq!(r.total, 1);
        assert_eq!(r.done, 1, "falhou: {:?}", r.items[0].error);
        let arquivo = r.items[0].file.clone().expect("o caminho final do vídeo");
        assert!(std::path::Path::new(&arquivo).exists(), "{}", arquivo);
        assert!(load_state(&dir).is_some(), "o estado tem de ficar na pasta");

        let mtime = std::fs::metadata(&arquivo).and_then(|m| m.modified()).ok();
        let r2 = run(opts, super::super::noop_progress())
            .await
            .expect("segunda rodada");
        assert_eq!(r2.pending, 0, "não sobrou nada para baixar");
        assert_eq!(r2.done, 1);
        assert_eq!(r2.items[0].attempts, 1, "não podia ter tentado de novo");
        assert_eq!(
            std::fs::metadata(&arquivo).and_then(|m| m.modified()).ok(),
            mtime,
            "o arquivo foi reescrito"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// Precisa de rede e do yt-dlp.
    /// `cargo test -p omniget-core --lib -- --ignored live_yt_archive`
    #[tokio::test]
    #[ignore]
    async fn live_yt_archive_enumerates_a_public_playlist() {
        let dir = super::super::temp_dir().join("yt-archive-live");
        let opts = Options {
            url: "https://www.youtube.com/playlist?list=PLbpi6ZahtOH6Blw3RGYpWkSByi_T7Rygb".into(),
            dest: dir.to_string_lossy().to_string(),
            quality: default_quality(),
            write_subs: false,
            write_thumbnail: false,
            write_info_json: false,
            max_items: 0,
            refresh: true,
            retry_failed: false,
            account_slug: None,
            session_netscape: None,
            sub_langs: None,
            max_bytes: 0,
            sleep_seconds: 0,
        };
        let r = scan(&opts, &super::super::noop_progress())
            .await
            .expect("a playlist devia listar");
        assert!(r.total > 0);
        eprintln!("{}: {} itens", r.title, r.total);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
