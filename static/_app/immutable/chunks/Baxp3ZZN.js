import{invoke as u}from"./DhEqZVGG.js";import{listen as p}from"./DZm6wUkB.js";const f="clitools://updates",y=(e,o)=>u("clitools_config_files",{tool:e,projectDir:o}),h=(e={})=>u("clitools_updates",{auto:e.auto??!1,force:e.force??!1}),$=()=>u("clitools_updates_cached");function k(e){return p(f,o=>e(o.payload))}function w(e){const o=["global","project","local"],s=new Map;for(const a of e){const t=s.get(a.scope)??[];t.push(a),s.set(a.scope,t)}return[...s.entries()].sort((a,t)=>o.indexOf(a[0])-o.indexOf(t[0]))}function v(e){return e<1024?`${e} B`:e<1024*1024?`${(e/1024).toFixed(1)} KB`:`${(e/1024/1024).toFixed(1)} MB`}function x(e,o){const s={};for(const a of o)for(const[t,i]of Object.entries(e[a]??{})){const r=t;s[r]=[...new Set([...s[r]??[],...i??[]])]}return s}function S(e){const o=e.files.filter(t=>t.exists).map(t=>`- [${t.scope}/${t.key}] ${t.path}${t.is_dir?` (pasta, ${t.files} arquivos)`:""} · ${v(t.size)}`).join(`
`),s=e.facts.length?`
Números medidos pelo OmniGet:
${e.facts.map(t=>`- ${t}`).join(`
`)}
`:"";return e.locale.startsWith("pt")?`Revise a configuração do ${e.toolName} nestes arquivos:
${o}
${s}
Objetivo: deixar a configuração mais enxuta e correta, sem perder comportamento.
1. Aponte regras/agentes/comandos pesados em tokens ou duplicados e proponha versões mais curtas.
2. Hooks: comandos cujo programa não existe, variáveis de ambiente inexistentes, eventos errados.
3. MCPs: servidores que não sobem, duplicados, desnecessários ou com segredo escrito no arquivo.
4. Formatos ou caminhos deprecados.
Não abra arquivos de credencial (auth.json, credentials*, oauth*, *token*). Não altere nada ainda: entregue um relatório com o diff proposto por arquivo.`:`Review the ${e.toolName} configuration in these files:
${o}
${s}
Goal: make the configuration leaner and correct without losing behaviour.
1. Point out token-heavy or duplicated rules/agents/commands and propose shorter versions.
2. Hooks: commands whose program does not exist, env vars that do not exist, wrong events.
3. MCP servers that do not start, duplicates, unneeded ones or secrets written in the file.
4. Deprecated formats or paths.
Do not open credential files (auth.json, credentials*, oauth*, *token*). Do not change anything yet: deliver a report with the proposed diff per file.`}const m="omniget.central.tools.updates.last-auto",c=24*3600*1e3;function _(e){let o=null,s=!1;const a=()=>{try{return Number(localStorage.getItem(m)??0)||0}catch{return 0}},t=n=>{try{localStorage.setItem(m,String(n))}catch{}},i=()=>{o&&clearTimeout(o),o=null},r=()=>{if(i(),s||document.visibilityState!=="visible")return;const n=Math.max(0,a()+c-Date.now());o=setTimeout(l,Math.min(n,c)+1e3)},l=async()=>{if(o=null,!(s||document.visibilityState!=="visible")){if(Date.now()-a()>=c)try{const n=await h({auto:!0});t(n.from_cache?n.checked_at*1e3:Date.now()),e==null||e(n)}catch{t(Date.now()-c+3600*1e3)}r()}},d=()=>{document.visibilityState==="visible"?l():i()};return document.addEventListener("visibilitychange",d),l(),()=>{s=!0,i(),document.removeEventListener("visibilitychange",d)}}export{h as a,w as b,y as c,k as d,$ as e,v as f,x as m,S as o,_ as s};
