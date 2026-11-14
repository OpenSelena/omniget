//! Acervo montado a partir de buscas, não de um canal ou playlist.
//!
//! O `yt_archive` sabe enumerar uma URL; para juntar vídeos de várias buscas
//! (ex.: todos os guias de um jogo, em vários idiomas) a fila precisa ser
//! semeada com resultados já filtrados. `seed` grava esses itens no mesmo
//! estado `.libreget-archive.json`, sem rede, e o `yt_archive::run` segue dali
//! com retomada, falhas registradas e teto de espaço em disco.

use std::path::Path;

use serde::Serialize;

use super::yt_archive::{self, ArchiveItem, ArchiveState, ItemStatus};

#[derive(Debug, Clone, Serialize)]
pub struct SeedResult {
    pub state_path: String,
    /// Itens novos que entraram na fila agora.
    pub added: usize,
    /// Tamanho da fila depois da semeadura.
    pub total: usize,
}

/// Lê NDJSON (uma linha por vídeo, como o `libreget search --json` imprime)
/// e devolve itens pendentes. Linha sem `id` ou que não é JSON é ignorada.
pub fn parse_seed(text: &str) -> Vec<ArchiveItem> {
    let mut seen = std::collections::HashSet::new();
    text.lines()
        .filter_map(|line| serde_json::from_str::<serde_json::Value>(line.trim()).ok())
        .filter_map(|json| {
            let id = json.get("id")?.as_str()?.trim().to_string();
            if id.is_empty() || !seen.insert(id.clone()) {
                return None;
            }
            let text_of = |k: &str| {
                json.get(k)
                    .and_then(|v| v.as_str())
                    .unwrap_or_default()
                    .to_string()
            };
            let url = Some(text_of("url"))
                .filter(|u| !u.is_empty())
                .unwrap_or_else(|| format!("https://www.youtube.com/watch?v={}", id));
            Some(ArchiveItem {
                title: text_of("title"),
                url,
                duration_seconds: json.get("duration").and_then(|v| v.as_f64()).unwrap_or(0.0),
                status: ItemStatus::Pending,
                error: None,
                file: None,
                attempts: 0,
                id,
            })
        })
        .collect()
}

/// Junta os itens à fila da pasta sem perder o que já foi baixado ou falhou.
pub fn seed(dest: &Path, title: &str, items: Vec<ArchiveItem>) -> anyhow::Result<SeedResult> {
    let mut st = yt_archive::load_state(dest).unwrap_or_else(|| ArchiveState::new("", title));
    let added = st.merge(title, items, false);
    let path = yt_archive::save_state(dest, &mut st)?;
    Ok(SeedResult {
        state_path: path.to_string_lossy().to_string(),
        added,
        total: st.items.len(),
    })
}

/// Soma o tamanho dos arquivos sob `dir` (sem seguir links). Usado como teto
/// do acervo: a medida é o disco de fato, não o que o estado acha que baixou.
pub fn dir_size(dir: &Path) -> u64 {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return 0;
    };
    entries
        .flatten()
        .map(|e| match e.file_type() {
            Ok(t) if t.is_dir() => dir_size(&e.path()),
            Ok(t) if t.is_file() => e.metadata().map(|m| m.len()).unwrap_or(0),
            _ => 0,
        })
        .sum()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp(name: &str) -> std::path::PathBuf {
        let dir = super::super::temp_dir().join(format!("{}-{}", name, uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).expect("pasta temporária");
        dir
    }

    const HITS: &str = r#"{"id":"a1","title":"A","url":"https://youtu.be/a1","duration":600.0}
{"id":"b2","title":"B"}
lixo
{"id":"a1","title":"A de novo"}
{"id":"  ","title":"vazio"}"#;

    #[test]
    fn parse_seed_keeps_first_of_each_id() {
        let items = parse_seed(HITS);
        assert_eq!(items.len(), 2);
        assert_eq!(items[0].title, "A");
        assert_eq!(items[0].duration_seconds, 600.0);
        assert_eq!(items[1].url, "https://www.youtube.com/watch?v=b2");
        assert!(items.iter().all(|i| i.status == ItemStatus::Pending));
    }

    #[test]
    fn seeding_twice_adds_only_new_ids_and_keeps_status() {
        let dir = temp("yt-dataset-seed");
        let r = seed(&dir, "Mir4", parse_seed(HITS)).expect("primeira semeadura");
        assert_eq!((r.added, r.total), (2, 2));

        let mut st = yt_archive::load_state(&dir).expect("estado gravado");
        st.mark("a1", ItemStatus::Ok, Some("a1.mp4".into()), None);
        yt_archive::save_state(&dir, &mut st).expect("grava");

        let more = parse_seed("{\"id\":\"a1\"}\n{\"id\":\"c3\",\"title\":\"C\"}");
        let r2 = seed(&dir, "Mir4", more).expect("segunda semeadura");
        assert_eq!((r2.added, r2.total), (1, 3));
        let st = yt_archive::load_state(&dir).expect("estado");
        let a1 = st.items.iter().find(|i| i.id == "a1").expect("a1");
        assert_eq!(
            a1.status,
            ItemStatus::Ok,
            "o que já baixou não volta à fila"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn dir_size_sums_nested_files() {
        let dir = temp("yt-dataset-size");
        std::fs::create_dir_all(dir.join("canal")).expect("subpasta");
        std::fs::write(dir.join("a.bin"), vec![0u8; 100]).expect("a");
        std::fs::write(dir.join("canal").join("b.bin"), vec![0u8; 50]).expect("b");
        assert_eq!(dir_size(&dir), 150);
        assert_eq!(dir_size(&dir.join("não-existe")), 0);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
