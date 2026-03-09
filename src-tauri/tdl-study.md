# Estudo: tdl-master — Implementação de Download do Telegram

Estudo completo da implementação de download do [tdl](https://github.com/iyear/tdl) (Go), com foco em traduzir os padrões para Rust/grammers no OmniGet.

---

## 1. Arquitetura Geral

O tdl usa uma arquitetura em 3 camadas:

```
cmd/dl.go          → CLI (Cobra)
app/dl/dl.go       → Orquestração (iterator, progress, resume)
core/downloader/   → Engine de download (chunks, errgroup, WriteAt)
core/dcpool/       → Pool de conexões por DC
core/middlewares/  → Retry, Recovery, FloodWait, Takeout
```

---

## 2. A Chave: DC Proativo (Não Reativo)

### O problema que temos no OmniGet

O OmniGet usa `client.invoke(&request)` para baixar arquivos. Quando o arquivo está em outro DC (ex: DC 4), Telegram retorna `FILE_MIGRATE_4`. O grammers tenta lidar internamente no `iter_download`, mas nosso código custom (`download_parallel`) não consegue porque `copy_auth_to_dc()` é `pub(crate)`.

### Como o tdl resolve

**O tdl NUNCA recebe FILE_MIGRATE.** Ele lê o campo `dc_id` da metadata do documento/foto e conecta ao DC correto ANTES de baixar.

```go
// core/tmedia/document.go
func GetDocumentInfo(doc *tg.MessageMediaDocument) (*Media, bool) {
    d := doc.Document.(*tg.Document)
    return &Media{
        InputFileLoc: &tg.InputDocumentFileLocation{
            ID: d.ID, AccessHash: d.AccessHash, FileReference: d.FileReference,
        },
        DC:   d.DCID,   // ← DC onde o arquivo está armazenado
        Size: d.Size,
    }, true
}

// core/tmedia/photo.go
func GetPhotoInfo(photo *tg.MessageMediaPhoto) (*Media, bool) {
    p := photo.Photo.(*tg.Photo)
    return &Media{
        InputFileLoc: &tg.InputPhotoFileLocation{...},
        DC:   p.DCID,   // ← DC onde a foto está
        Size: int64(size),
    }, true
}
```

Depois, na hora de baixar:

```go
// core/downloader/downloader.go
func (d *Downloader) download(ctx context.Context, elem Elem) error {
    client := d.opts.Pool.Client(ctx, elem.File().DC())  // ← conecta ao DC certo
    // ...
    downloader.NewDownloader().Download(client, elem.File().Location()).Parallel(ctx, writer)
}
```

### Equivalente no Rust/grammers

O `dc_id` está disponível como campo público nos tipos TL gerados do grammers:

```rust
// grammers-tl-types (gerado de api.tl)
pub struct Document {
    pub id: i64,
    pub access_hash: i64,
    pub file_reference: Vec<u8>,
    pub dc_id: i32,        // ← campo público!
    pub size: i64,
    // ...
}

pub struct Photo {
    pub id: i64,
    pub access_hash: i64,
    pub file_reference: Vec<u8>,
    pub dc_id: i32,        // ← campo público!
    pub sizes: Vec<PhotoSize>,
    // ...
}
```

**Acesso:**

```rust
// Extraindo dc_id de MessageMedia
match &raw_media {
    tl::enums::MessageMedia::Document(doc_media) => {
        if let tl::enums::Document::Document(d) = doc_media.document.as_ref()? {
            let dc_id = d.dc_id;  // i32
            let size = d.size;    // i64
        }
    }
    tl::enums::MessageMedia::Photo(photo_media) => {
        if let tl::enums::Photo::Photo(p) = photo_media.photo.as_ref()? {
            let dc_id = p.dc_id;  // i32
        }
    }
}
```

---

## 3. DC Pool — Conexão por DC com Auth Transfer

### Padrão do tdl

```go
// core/dcpool/dcpool.go
type Pool interface {
    Client(ctx context.Context, dc int) *tg.Client
    Default(ctx context.Context) *tg.Client
    Close() error
}

func (p *pool) invoker(ctx context.Context, dc int) tg.Invoker {
    if i, ok := p.invokers[dc]; ok {
        return i  // já conectado, reusar
    }

    if dc == p.current() {
        invoker, _ = p.api.Pool(p.size)       // home DC: pool de conexões
    } else {
        invoker, _ = p.api.DC(ctx, dc, p.size) // outro DC: auth transfer automático
    }

    p.invokers[dc] = chainMiddlewares(invoker, p.middlewares...)
    return p.invokers[dc]
}
```

**Pontos-chave:**
- `api.DC(ctx, dc, size)` faz `auth.exportAuthorization` → `auth.importAuthorization` internamente (gotd library)
- Lazy init: só conecta ao DC quando necessário
- Cache: reutiliza conexões existentes
- Mutex-protected: thread-safe
- Fallback: se falhar, usa o client principal (vai dar FILE_MIGRATE, mas não crasha)

### Equivalente no grammers

O grammers tem `client.invoke_in_dc(dc, &request)` que é público, e internamente chama `copy_auth_to_dc()` (que é `pub(crate)`). O `iter_download` do grammers já usa isso. Mas para nosso uso direto, podemos:

**Opção A — Usar `iter_download` do grammers** (mais simples, o que tentamos):
```rust
let mut iter = client.iter_download(&media);
while let Some(chunk) = iter.next().await? { ... }
```
Isso funciona, mas é sequencial e sem controle de chunk size > 512KB.

**Opção B — Usar `invoke_in_dc` diretamente** (o que o tdl faz, mas precisa do dc_id):
```rust
let request = tl::functions::upload::GetFile { location, offset, limit: 1048576 };
let result = client.invoke_in_dc(dc_id, &request).await;
```
O `invoke_in_dc` é público no grammers e chama `copy_auth_to_dc` internamente na primeira vez.

**IMPORTANTE**: O `invoke_in_dc` do grammers chama `copy_auth_to_dc()` internamente quando necessário. O problema anterior era que chamávamos `invoke_in_dc` sem saber o DC correto (reagindo ao erro), e possivelmente o grammers não fazia o auth copy corretamente. Com o `dc_id` lido da metadata, podemos usar `invoke_in_dc` de forma proativa.

---

## 4. Download Paralelo — errgroup + WriteAt

### Padrão do tdl

```go
// core/downloader/downloader.go — MaxPartSize = 1MB
func (d *Downloader) Download(ctx context.Context, limit int) error {
    wg, wgctx := errgroup.WithContext(ctx)
    wg.SetLimit(limit)  // max N downloads em paralelo

    for d.opts.Iter.Next(wgctx) {
        elem := d.opts.Iter.Value()
        wg.Go(func() error {
            d.opts.Progress.OnAdd(elem)
            defer func() { d.opts.Progress.OnDone(elem, rerr) }()
            if err := d.download(wgctx, elem); err != nil {
                if errors.Is(err, context.Canceled) {
                    return err  // propaga cancelamento
                }
                // log e continua para o próximo arquivo
            }
            return nil
        })
    }
    return wg.Wait()
}
```

O download de cada arquivo usa gotd's `downloader.Parallel`:
- Divide o arquivo em chunks de 1MB
- `BestThreads(fileSize, maxThreads)` determina paralelismo por arquivo
- `io.WriterAt` para escrita em offsets arbitrários (paralelo)
- Cada chunk vai para o DC correto via pool

### BestThreads

```go
// core/util/tutil/tutil.go
func BestThreads(size int64, max int) int {
    // < 1 MB → 1 thread
    // < 5 MB → 2 threads
    // < 20 MB → 4 threads
    // < 50 MB → 8 threads
    // >= 50 MB → max threads
}
```

### Equivalente no Rust

```rust
// Usando JoinSet + Semaphore (como tínhamos antes, mas agora com dc_id)
let semaphore = Arc::new(Semaphore::new(threads));
let mut join_set = JoinSet::new();

for part_idx in 0..num_parts {
    let permit = semaphore.clone().acquire_owned().await?;
    join_set.spawn(async move {
        let _permit = permit;
        let request = tl::functions::upload::GetFile {
            precise: true, cdn_supported: false,
            location: location.clone(),
            offset: (part_idx * PART_SIZE) as i64,
            limit: PART_SIZE,
        };
        // USA invoke_in_dc com o dc_id correto!
        let result = client.invoke_in_dc(dc_id, &request).await;
        // ... write chunk at offset ...
    });
}
```

---

## 5. Middleware Chain

### Padrão do tdl (3 camadas)

```
recovery → retry → floodwait → RPC
```

1. **FloodWait** (innermost): `FLOOD_WAIT_X` → sleep X seconds, retry automaticamente
2. **Retry** (middle): Erros internos do Telegram (Timedout, RPC_CALL_FAIL, WORKER_BUSY) → até 5 retries
3. **Recovery** (outermost): Erros de rede (connection reset, timeout TCP) → backoff exponencial

**O que NÃO é retriado:**
- `FILE_MIGRATE` — nunca acontece porque usa DC correto
- Erros de negócio (CHAT_ADMIN_REQUIRED, etc.) — propagados imediatamente

### Equivalente no Rust

```rust
// Retry wrapper para nosso download
async fn invoke_with_retry<R>(
    client: &Client,
    dc_id: i32,
    request: &R,
    max_retries: u32,
) -> Result<R::Return, anyhow::Error> {
    let mut last_err = None;
    for attempt in 0..=max_retries {
        if attempt > 0 {
            let delay = 1000 * (1u64 << (attempt - 1).min(4));
            tokio::time::sleep(Duration::from_millis(delay)).await;
        }
        match client.invoke_in_dc(dc_id, request).await {
            Ok(response) => return Ok(response),
            Err(InvocationError::Rpc(ref err)) if err.code == 420 => {
                // FLOOD_WAIT: sleep pelo tempo indicado
                let wait = err.value.unwrap_or(5) as u64;
                tracing::warn!("[tg] FLOOD_WAIT_{}, sleeping {}s", wait, wait);
                tokio::time::sleep(Duration::from_secs(wait)).await;
                continue; // não conta como retry
            }
            Err(e) => {
                last_err = Some(e);
                continue;
            }
        }
    }
    Err(anyhow::anyhow!("Failed after {} retries: {:?}", max_retries, last_err))
}
```

---

## 6. Resolução de Peers (Canais Privados)

### Padrão do tdl

```go
// pkg/tmessage/urls.go — Parse de URLs
// t.me/c/1234567890/123 → channel_id = 1234567890, msg_id = 123
// t.me/username/123 → username = "username", msg_id = 123

// core/util/tutil/tutil.go
func GetInputPeer(ctx, manager, from string) (peers.Peer, error) {
    id, err := strconv.ParseInt(from, 10, 64)
    if err != nil {
        return manager.Resolve(ctx, from)  // por username
    }
    // Tenta channel, depois user, depois chat
    if p, err = manager.ResolveChannelID(ctx, id); err == nil { return p, nil }
    if p, err = manager.ResolveUserID(ctx, id); err == nil { return p, nil }
    if p, err = manager.ResolveChatID(ctx, id); err == nil { return p, nil }
}
```

**Pontos-chave:**
- Usa `peers.Manager` com storage persistente (BoltDB) para cache de access_hash
- Para canais privados, o access_hash precisa ter sido visto antes (via dialogs)
- Resolve cascata: channel → user → chat

---

## 7. Progress Tracking com WriteAt

### Padrão do tdl

```go
// core/downloader/progress.go
type writeAt struct {
    elem       Elem
    progress   Progress
    partSize   int
    downloaded *atomic.Int64
}

func (w *writeAt) WriteAt(p []byte, off int64) (int, error) {
    at, err := w.elem.To().WriteAt(p, off)
    w.progress.OnDownload(w.elem, ProgressState{
        Downloaded: w.downloaded.Add(int64(at)),
        Total:      w.elem.File().Size(),
    })
    return at, nil
}
```

- `atomic.Int64` para thread-safety
- Cada chunk atualiza o progresso imediatamente
- `OnAdd` no início, `OnDone` no final (com erro ou sucesso)

### Equivalente no Rust

```rust
let downloaded = Arc::new(AtomicU64::new(0));

// Em cada task de chunk:
let prev = downloaded.fetch_add(chunk_len, Ordering::Relaxed);
let percent = ((prev + chunk_len) as f64 / total_size as f64) * 100.0;
let _ = progress_tx.send(percent.min(100.0)).await;
```

---

## 8. Tratamento de Erros

### Padrão do tdl

| Erro | Ação |
|------|------|
| `FLOOD_WAIT_X` | Sleep X seconds, retry |
| `FILE_REFERENCE_EXPIRED` | Re-fetch message, retry |
| `Timedout`, `RPC_CALL_FAIL` | Retry até 5x |
| Connection reset, TCP timeout | Backoff exponencial |
| `context.Canceled` (Ctrl+C) | Propaga, para tudo |
| Outros erros | Log + skip arquivo, continua batch |

### Equivalente no Rust

```rust
match result {
    Err(InvocationError::Rpc(err)) if err.code == 420 => { /* FLOOD_WAIT */ }
    Err(InvocationError::Rpc(err)) if err.code == 400
        && err.name.contains("FILE_REFERENCE") => { /* re-fetch */ }
    Err(InvocationError::Rpc(err)) if err.code == 500 => { /* retry interno */ }
    Err(e) if is_network_error(&e) => { /* backoff exponencial */ }
    Err(e) => { /* log e skip */ }
}
```

---

## 9. Takeout Sessions (Opcional)

O tdl suporta "Takeout" — modo de export bulk do Telegram com rate limits elevados:

```go
// Inicia takeout session
sid, _ := c.API().AccountInitTakeoutSession(ctx, &tg.AccountInitTakeoutSessionRequest{
    Files:       true,
    FileMaxSize: 4000 * 1024 * 1024,  // 4 GB
})

// Middleware wraps todas as requests
tg.InvokeWithTakeoutRequest{TakeoutID: sid, Query: request}

// Finaliza
c.API().AccountFinishTakeoutSession(ctx, &tg.AccountFinishTakeoutSessionRequest{Success: true})
```

Útil para downloads massivos. Pode ser implementado como feature futura no OmniGet.

---

## 10. Plano de Implementação para o OmniGet

### Passo 1: Extrair dc_id da media

No `parallel_download.rs`, modificar `media_to_input_location` para retornar `dc_id`:

```rust
pub struct MediaLocation {
    pub location: tl::enums::InputFileLocation,
    pub size: u64,
    pub dc_id: i32,
}

pub fn media_to_location(media: &tl::enums::MessageMedia) -> Option<MediaLocation> {
    match media {
        tl::enums::MessageMedia::Document(doc_media) => {
            let doc = match doc_media.document.as_ref()? {
                tl::enums::Document::Document(d) => d,
                _ => return None,
            };
            Some(MediaLocation {
                location: tl::enums::InputFileLocation::InputDocumentFileLocation(...),
                size: doc.size as u64,
                dc_id: doc.dc_id,
            })
        }
        tl::enums::MessageMedia::Photo(photo_media) => {
            let photo = match photo_media.photo.as_ref()? {
                tl::enums::Photo::Photo(p) => p,
                _ => return None,
            };
            Some(MediaLocation {
                location: tl::enums::InputFileLocation::InputPhotoFileLocation(...),
                size: largest.size as u64,
                dc_id: photo.dc_id,
            })
        }
        _ => None,
    }
}
```

### Passo 2: Download paralelo com invoke_in_dc usando dc_id

```rust
pub async fn download_parallel(
    client: &Client,
    media: &MediaLocation,
    output_path: &Path,
    progress_tx: mpsc::Sender<f64>,
    cancel_token: &CancellationToken,
    max_threads: usize,
) -> anyhow::Result<u64> {
    let threads = best_threads(media.size, max_threads);
    let num_parts = media.size.div_ceil(PART_SIZE as u64);
    let dc_id = media.dc_id;

    // Pre-criar arquivo com tamanho total
    let file = Arc::new(std::fs::File::create(output_path)?);
    file.set_len(media.size)?;

    let downloaded = Arc::new(AtomicU64::new(0));
    let semaphore = Arc::new(Semaphore::new(threads));
    let mut join_set = JoinSet::new();

    for part_idx in 0..num_parts {
        let offset = part_idx * PART_SIZE as u64;
        // ... clone vars ...
        join_set.spawn(async move {
            let _permit = semaphore.acquire().await?;

            let request = tl::functions::upload::GetFile {
                precise: true, cdn_supported: false,
                location: location.clone(),
                offset: offset as i64,
                limit: PART_SIZE,
            };

            // ★ CHAVE: usa invoke_in_dc com o dc_id da metadata
            let result = client.invoke_in_dc(dc_id, &request).await;

            match result {
                Ok(tl::enums::upload::File::File(f)) => {
                    write_at_offset(&file, &f.bytes, offset)?;
                    // update progress...
                }
                Err(e) => { /* retry logic */ }
            }
        });
    }
    // ... join_set.join_next() ...
}
```

### Passo 3: Retry com FLOOD_WAIT

Implementar retry wrapper que trata FLOOD_WAIT automaticamente, como o tdl faz com middleware.

### Passo 4: Fallback para sequencial

Se `invoke_in_dc` falhar por qualquer motivo, fall back para `client.iter_download()` do grammers que tem seu próprio FILE_MIGRATE handling.

### Passo 5: (Futuro) Takeout sessions

Para downloads massivos de canais inteiros.

---

## 11. Diferenças Chave: tdl (gotd) vs OmniGet (grammers)

| Aspecto | tdl (gotd/Go) | OmniGet (grammers/Rust) |
|---------|---------------|------------------------|
| DC Pool | `api.DC(ctx, dc, size)` — pool de N conexões por DC | `client.invoke_in_dc(dc, &req)` — uma conexão |
| Auth Transfer | `api.DC()` faz internamente | `invoke_in_dc` chama `copy_auth_to_dc()` internamente |
| FILE_MIGRATE | Nunca acontece (DC proativo) | Pode acontecer se dc_id incorreto |
| Parallel chunks | gotd `downloader.Parallel` com `io.WriterAt` | Custom `JoinSet` + `Semaphore` |
| Chunk size | 1 MB (max API) | 1 MB (nosso) ou 512 KB (grammers default) |
| FloodWait | Middleware automático | Manual (implementar) |
| Takeout | Suportado | Não implementado |
| Resume | Fingerprint + KV storage | Não implementado |
| Connection pooling | N conexões por DC | 1 conexão por DC |

---

## 12. Resumo das Lições

1. **Ler `dc_id` da metadata** — O campo `dc_id` está nos tipos `Document` e `Photo` do grammers-tl-types. Usar ele para conectar ao DC correto ANTES de baixar.

2. **`invoke_in_dc(dc_id, &request)`** — O grammers faz `copy_auth_to_dc()` internamente. Usar diretamente com o `dc_id` da metadata.

3. **Parallel com WriteAt** — Dividir em chunks de 1MB, usar `Semaphore` para limitar threads, `seek_write`/`write_all_at` para escrita paralela.

4. **FLOOD_WAIT handler** — Detectar código 420, sleep pelo tempo indicado, retry.

5. **Batch error handling** — Erro em 1 arquivo não para o batch. Só `context.Canceled` propaga.

6. **Temp file pattern** — Download para `.tmp`, rename no final.

7. **BestThreads** — Escalar paralelismo por tamanho do arquivo.
