/**
 * The Nami download engine reports stage names and status messages in
 * English from the Rust side. This module maps the known ones onto the active
 * locale and passes everything else through untouched.
 */

export type NamiTranslate = (key: string, values?: Record<string, unknown>) => string;

const STAGE_KEYS: Record<string, string> = {
  setup: 'nami.stage_setup',
  photos: 'nami.stage_photos',
  videos: 'nami.stage_videos',
  stories: 'nami.stage_stories',
  highlights: 'nami.stage_highlights'
};

const STATUS_KEYS: Record<string, string> = {
  'Download cancelled': 'nami.status_cancelled',
  'Provisioning download engine binary...': 'nami.status_provisioning',
  'Switching to secondary video engine (yt-dlp)...': 'nami.status_switching_engine',
  'Engine not found (auto-download available)': 'nami.engine_missing',
  'All download engines ready': 'nami.engines_ready'
};

export function namiStageLabel(
  stage: string | null | undefined,
  t: NamiTranslate
): string {
  if (!stage) return '';
  const key = STAGE_KEYS[stage.trim().toLowerCase()];
  return key ? t(key) : stage;
}

export function namiStatusMessage(
  message: string | null | undefined,
  t: NamiTranslate
): string {
  if (!message) return '';
  const text = message.trim();

  const exact = STATUS_KEYS[text];
  if (exact) return t(exact);

  if (text.startsWith('Starting ')) {
    return t('nami.status_starting', {
      stage: namiStageLabel(text.slice('Starting '.length), t)
    });
  }
  if (text.startsWith('Downloaded: ')) {
    return t('nami.status_downloaded', { file: text.slice('Downloaded: '.length) });
  }
  const success = /^Successfully downloaded (\d+) files$/.exec(text);
  if (success) {
    return t('nami.status_success', { count: success[1] });
  }

  return text;
}
