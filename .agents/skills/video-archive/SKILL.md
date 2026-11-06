---
name: video-archive
description: >-
  Build or grow a local video dataset from YouTube with the OmniGet CLI
  (`omniget-cli search` + `omniget-cli archive`): many search terms in many
  languages, duration and title filters, low-resolution H.264 files, subtitles
  and `.info.json` metadata, a GB cap, and a queue that resumes after a stop.
  Use when asked to "download every video about X", "build a gameplay/tutorial
  dataset", "archive a channel", "continue the archive", or "how far is the
  download".
---

# Video archive with the OmniGet CLI

The CLI binary is `omniget-cli` (installed with
`cargo install --path src-tauri/omniget-cli --root ~/.local --locked`).
Everything below runs without the desktop app open. `yt-dlp` is found on PATH
or in the app's data folder; `ffmpeg` must be on PATH.

## Layout of a dataset folder (outside the repo)

```
~/datasets/<topic>/
  queries.txt        one search term per line, # for comments
  hits.ndjson        output of search (one video per line)
  videos/            the archive: <uploader>/<title> [<id>].mp4 + .srt + .info.json
  videos/.omniget-archive.json   queue state (pending/ok/failed/skipped)
  run.log            NDJSON log of the last archive run
```

## Workflow

1. **Search.** One term per language and theme; filters cut shorts, live
   streams and off-topic results. Results are deduplicated across terms.
   ```bash
   omniget-cli --json search --queries-file queries.txt -n 100 \
     --min-duration 120 --max-duration 5400 --title-has "mir4,mir 4" \
     > hits.ndjson 2> search.log
   ```
   `-n` goes up to 500 per term. Progress per term is on stderr.
2. **Seed and download.** `--seed` adds only new ids; what is already done
   stays done.
   ```bash
   omniget-cli --json archive --dest videos --seed hits.ndjson \
     --quality 360 --subs ".*-orig" --info-json --max-gb 30 --sleep 5 \
     >> run.log 2>&1
   ```
   Run it in the background for big queues. A channel or playlist URL can be
   passed instead of (or with) `--seed`; it is re-listed on every run so new
   uploads join the queue.
3. **Check progress** without downloading:
   `omniget-cli --json archive --dest videos --scan-only | tail -1`
   (the last line is the summary: `done`, `failed`, `pending`, `size_bytes`,
   `cap_reached`).
4. **Resume** by running the same archive command again. Ctrl+C, or
   `pkill -INT -f "omniget-cli --json archive"`, stops yt-dlp within a second
   and puts the current item back in the queue (its `.part` is reused). Wait
   for the process to exit before starting another run on the same folder.
5. **Retry failures** later with `--retry-failed`. If the network drops, the
   run does not burn the queue: it logs `sem conexão: aguardando a rede voltar`
   and waits for DNS to come back. If YouTube flags the IP ("Sign in to
   confirm you're not a bot"), it logs `YouTube barrou o IP como robô` and
   retries the same item after 15, 30, 60, 120, 240 and 240 min (~12 h). If the
   block outlasts that, it stops with `blocked: true` and keeps the queue
   pending; run again hours later.

## Choices and why

| Flag | Value | Why |
| :--- | :--- | :--- |
| `--quality` | `360` | Caps height and prefers H.264/AAC: ~200 MB per hour, decodes everywhere. Use `480`/`720` when the model needs small UI text. |
| `--subs` | `".*-orig"` | Only YouTube's original-language transcript. Translated tracks multiply requests and trigger HTTP 429. Subtitle errors never fail the video. |
| `--sleep` | `5` | Spacing between videos (randomised up to 2×) keeps YouTube from throttling. |
| `--max-gb` | disk budget | Checked before each video; the run stops with `cap_reached: true`. |
| `--cookies` | only if needed | Age-gated or members-only videos. Off by default so a mass download never runs on the user's account. |

## Rules

- Keep datasets outside the repository. Do not commit media.
- The archive is for private use (research, training, testing); do not
  re-upload the videos.
- When `failed` grows with `HTTP Error 429`, stop, raise `--sleep`, and resume
  later; do not loop retries.
- Report to the user with the summary line, not with the raw log.
