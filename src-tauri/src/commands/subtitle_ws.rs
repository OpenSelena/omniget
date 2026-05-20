use omniget_core::core::subtitle_merge::{self, Cue};

#[cfg(not(target_os = "android"))]
#[tauri::command]
pub async fn subtitle_load(path: String) -> Result<Vec<Cue>, String> {
    let content = tokio::fs::read_to_string(&path)
        .await
        .map_err(|e| format!("Read failed: {}", e))?;
    let cues = subtitle_merge::parse_cues(&content);
    if cues.is_empty() {
        return Err("No cues found".to_string());
    }
    Ok(cues)
}

#[cfg(not(target_os = "android"))]
#[tauri::command]
pub async fn subtitle_save(path: String, cues: Vec<Cue>, format: String) -> Result<(), String> {
    let body = match format.as_str() {
        "vtt" => subtitle_merge::cues_to_vtt(&cues),
        "ass" => subtitle_merge::cues_to_ass(&cues),
        _ => subtitle_merge::cues_to_srt(&cues),
    };
    tokio::fs::write(&path, body)
        .await
        .map_err(|e| format!("Write failed: {}", e))
}

#[cfg(not(target_os = "android"))]
#[tauri::command]
pub async fn subtitle_translate(cues: Vec<Cue>, target_lang: String) -> Result<Vec<Cue>, String> {
    if !omniget_core::core::ai::get().is_configured() {
        return Err("ai_not_configured".to_string());
    }
    if cues.is_empty() {
        return Ok(cues);
    }
    // Numbered lines keep the model aligned; multi-line cues are flattened with
    // a sentinel so we can restore line breaks after translation.
    let joined: String = cues
        .iter()
        .enumerate()
        .map(|(i, c)| format!("{}\u{241F}{}", i + 1, c.text.replace('\n', "\u{2424}")))
        .collect::<Vec<_>>()
        .join("\n");
    let system = format!(
        "Translate each numbered line into {}. Keep the exact same numbering and \
line count. Preserve the \u{2424} markers as line breaks. Output ONLY the \
translated numbered lines, nothing else.",
        target_lang
    );
    let out = omniget_core::core::ai::chat(&system, &joined).await?;

    let mut map: std::collections::HashMap<usize, String> = std::collections::HashMap::new();
    for line in out.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        if let Some((num, rest)) = line.split_once('\u{241F}') {
            if let Ok(idx) = num.trim().parse::<usize>() {
                map.insert(idx, rest.replace('\u{2424}', "\n"));
            }
        }
    }
    if map.len() != cues.len() {
        return Err("translation_mismatch".to_string());
    }
    let mut result = cues;
    for (i, c) in result.iter_mut().enumerate() {
        if let Some(t) = map.get(&(i + 1)) {
            c.text = t.trim().to_string();
        }
    }
    Ok(result)
}

#[cfg(not(target_os = "android"))]
#[tauri::command]
pub async fn subtitle_grammar_fix(cues: Vec<Cue>, style: String) -> Result<Vec<Cue>, String> {
    if !omniget_core::core::ai::get().is_configured() {
        return Err("ai_not_configured".to_string());
    }
    if cues.is_empty() {
        return Ok(cues);
    }
    let tone = match style.as_str() {
        "formal" => "Use a formal register.",
        "casual" => "Use a casual, conversational register.",
        _ => "Keep the original register and wording as close as possible.",
    };
    let joined: String = cues
        .iter()
        .enumerate()
        .map(|(i, c)| format!("{}\u{241F}{}", i + 1, c.text.replace('\n', "\u{2424}")))
        .collect::<Vec<_>>()
        .join("\n");
    let system = format!(
        "Fix grammar, spelling and punctuation in each numbered line without \
changing its meaning. {} Keep the exact same numbering and line count. \
Preserve the \u{2424} markers as line breaks. Output ONLY the corrected \
numbered lines, nothing else.",
        tone
    );
    let out = omniget_core::core::ai::chat(&system, &joined).await?;

    let mut map: std::collections::HashMap<usize, String> = std::collections::HashMap::new();
    for line in out.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        if let Some((num, rest)) = line.split_once('\u{241F}') {
            if let Ok(idx) = num.trim().parse::<usize>() {
                map.insert(idx, rest.replace('\u{2424}', "\n"));
            }
        }
    }
    if map.len() != cues.len() {
        return Err("grammar_mismatch".to_string());
    }
    let mut result = cues;
    for (i, c) in result.iter_mut().enumerate() {
        if let Some(t) = map.get(&(i + 1)) {
            c.text = t.trim().to_string();
        }
    }
    Ok(result)
}
