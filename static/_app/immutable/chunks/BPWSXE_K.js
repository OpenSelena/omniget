const C=l=>l?l.replace(/[\t\n\f\r ]+/g," ").replace(/^[\t\n\f\r ]+/,"").replace(/[\t\n\f\r ]+$/,""):"",m=l=>C(l==null?void 0:l.textContent),v={XLINK:"http://www.w3.org/1999/xlink",EPUB:"http://www.idpf.org/2007/ops"},q={XML:"application/xml",XHTML:"application/xhtml+xml"},d={strong:["strong","self"],emphasis:["em","self"],style:["span","self"],a:"anchor",strikethrough:["s","self"],sub:["sub","self"],sup:["sup","self"],code:["code","self"],image:"image"},D={tr:["tr",{th:["th",d,["colspan","rowspan","align","valign"]],td:["td",d,["colspan","rowspan","align","valign"]]},["align"]]},E={epigraph:["blockquote"],subtitle:["h2",d],"text-author":["p",d],date:["p",d],stanza:["div","self"],v:["div",d]},M={title:["header",{p:["h1",d],"empty-line":["br"]}],epigraph:["blockquote","self"],image:"image",annotation:["aside"],section:["section","self"],p:["p",d],poem:["blockquote",E],subtitle:["h2",d],cite:["blockquote","self"],"empty-line":["br"],table:["table",D],"text-author":["p",d]};E.epigraph.push(M);const N={image:"image",title:["section",{p:["h1",d],"empty-line":["br"]}],epigraph:["section",M],section:["section",M]};class O{constructor(e){this.fb2=e,this.doc=document.implementation.createDocument(v.XHTML,"html"),this.bins=new Map(Array.from(this.fb2.getElementsByTagName("binary"),o=>[o.id,o]))}getImageSrc(e){const o=e.getAttributeNS(v.XLINK,"href");if(!o)return"data:,";const[,c]=o.split("#");if(!c)return o;const s=this.bins.get(c);return s?`data:${s.getAttribute("content-type")};base64,${s.textContent}`:o}image(e){const o=this.doc.createElement("img");return o.alt=e.getAttribute("alt"),o.title=e.getAttribute("title"),o.setAttribute("src",this.getImageSrc(e)),o}anchor(e){const o=this.convert(e,{a:["a",d]});return o.setAttribute("href",e.getAttributeNS(v.XLINK,"href")),e.getAttribute("type")==="note"&&o.setAttributeNS(v.EPUB,"epub:type","noteref"),o}convert(e,o){if(e.nodeType===3)return this.doc.createTextNode(e.textContent);if(e.nodeType===4)return this.doc.createCDATASection(e.textContent);if(e.nodeType===8)return this.doc.createComment(e.textContent);const c=o==null?void 0:o[e.nodeName];if(!c)return null;if(typeof c=="string")return this[c](e);const[s,u,f]=c,g=this.doc.createElement(s);if(e.id&&(g.id=e.id),g.classList.add(e.nodeName),Array.isArray(f))for(const y of f){const S=e.getAttribute(y);S&&g.setAttribute(y,S)}const L=u==="self"?o:u;let A=e.firstChild;for(;A;){const y=this.convert(A,L);y&&g.append(y),A=A.nextSibling}return g}}const X=async l=>{var f;const e=await l.arrayBuffer(),o=new TextDecoder("utf-8").decode(e),c=new DOMParser,s=c.parseFromString(o,q.XML),u=s.xmlEncoding||((f=o.match(/^<\?xml\s+version\s*=\s*["']1.\d+"\s+encoding\s*=\s*["']([A-Za-z0-9._-]*)["']/))==null?void 0:f[1]);if(u&&u.toLowerCase()!=="utf-8"){const g=new TextDecoder(u).decode(e);return c.parseFromString(g,q.XML)}return s},I=URL.createObjectURL(new Blob([`
@namespace epub "http://www.idpf.org/2007/ops";
body > img, section > img {
    display: block;
    margin: auto;
}
.title h1 {
    text-align: center;
}
body > section > .title, body.notesBodyType > .title {
    margin: 3em 0;
}
body.notesBodyType > section .title h1 {
    text-align: start;
}
body.notesBodyType > section .title {
    margin: 1em 0;
}
p {
    text-indent: 1em;
    margin: 0;
}
:not(p) + p, p:first-child {
    text-indent: 0;
}
.stanza {
    text-indent: 0;
    margin: 1em 0;
}
.text-author, .date {
    text-align: end;
}
.text-author:before {
    content: "—";
}
table {
    border-collapse: collapse;
}
td, th {
    padding: .25em;
}
a[epub|type~="noteref"] {
    font-size: .75em;
    vertical-align: super;
}
body:not(.notesBodyType) > .title, body:not(.notesBodyType) > .epigraph {
    margin: 3em 0;
}
`],{type:"text/css"})),z=l=>`<?xml version="1.0" encoding="utf-8"?>
<html xmlns="http://www.w3.org/1999/xhtml">
    <head><link href="${I}" rel="stylesheet" type="text/css"/></head>
    <body>${l}</body>
</html>`,B="data-foliate-id",H=async l=>{const e={},o=await X(l),c=new O(o),s=t=>o.querySelector(t),u=t=>[...o.querySelectorAll(t)],f=t=>{const n=m(t.querySelector("nickname"));if(n)return n;const r=m(t.querySelector("first-name")),i=m(t.querySelector("middle-name")),a=m(t.querySelector("last-name")),p=[r,i,a].filter(h=>h).join(" "),b=a?[a,[r,i].filter(h=>h).join(" ")].join(", "):null;return{name:p,sortAs:b}},g=t=>(t==null?void 0:t.getAttribute("value"))??m(t),L=s("title-info annotation"),A=u("title-info > sequence").map(t=>({name:C(t.getAttribute("name")),position:t.getAttribute("number")||void 0})).filter(t=>t.name);if(e.metadata={title:m(s("title-info book-title")),identifier:m(s("document-info id")),language:m(s("title-info lang")),author:u("title-info author").map(f),translator:u("title-info translator").map(f),contributor:u("document-info author").map(f).concat(u("document-info program-used").map(m)).map(t=>Object.assign(typeof t=="string"?{name:t}:t,{role:"bkp"})),belongsTo:A.length?{series:A}:void 0,publisher:m(s("publish-info publisher")),published:g(s("title-info date")),modified:g(s("document-info date")),description:L?c.convert(L,{annotation:["div",M]}).innerHTML:null,subject:u("title-info genre").map(m)},s("coverpage image")){const t=c.getImageSrc(s("coverpage image"));e.getCover=()=>fetch(t).then(n=>n.blob())}else e.getCover=()=>null;const y=Array.from(o.querySelectorAll("body"),t=>{const n=c.convert(t,{body:["body",N]});return[Array.from(n.children,r=>{const i=[r,...r.querySelectorAll("[id]")].map(a=>a.id);return{el:r,ids:i}}),n]}),S=[],$=y[0][0].map(({el:t,ids:n},r)=>{const i=Array.from(t.querySelectorAll(":scope > section > .title"),(a,p)=>{a.setAttribute(B,p);const b=a.closest("section"),h=new TextEncoder().encode(b.innerHTML).length-Array.from(b.querySelectorAll("[src]")).reduce((x,T)=>{var w;return x+(((w=T.getAttribute("src"))==null?void 0:w.length)??0)},0);return{title:m(a),index:p,size:h,href:`${r}#${p}`}});return{ids:n,titles:i,el:t}}).concat(y.slice(1).map(([t,n])=>{const r=t.map(i=>i.ids).flat();return n.classList.add("notesBodyType"),{ids:r,el:n,linear:"no"}})).map(({ids:t,titles:n,el:r,linear:i})=>{var x;const a=z(r.outerHTML),p=new Blob([a],{type:q.XHTML}),b=URL.createObjectURL(p);S.push(b);const h=C(((x=r.querySelector(".title, .subtitle, p"))==null?void 0:x.textContent)??(r.classList.contains("title")?r.textContent:""));return{ids:t,title:h,titles:n,load:()=>b,createDocument:()=>new DOMParser().parseFromString(a,q.XHTML),size:p.size-Array.from(r.querySelectorAll("[src]"),T=>{var w;return((w=T.getAttribute("src"))==null?void 0:w.length)??0}).reduce((T,w)=>T+w,0),linear:i}}),k=new Map;return e.sections=$.map((t,n)=>{const{ids:r,load:i,createDocument:a,size:p,linear:b,titles:h}=t;for(const x of r)x&&k.set(x,n);return{id:n,load:i,createDocument:a,size:p,linear:b,subitems:h}}),e.toc=$.map(({title:t,titles:n},r)=>{const i=r.toString();return{label:t,href:i,subitems:n!=null&&n.length?n.map(({title:a,index:p})=>({label:a,href:`${i}#${p}`})):null}}).filter(t=>t),e.resolveHref=t=>{const[n,r]=t.split("#");return n?{index:Number(n),anchor:i=>i.querySelector(`[${B}="${r}"]`)}:{index:k.get(r),anchor:i=>i.getElementById(r)}},e.splitTOCHref=t=>{var n;return((n=t==null?void 0:t.split("#"))==null?void 0:n.map(r=>Number(r)))??[]},e.getTOCFragment=(t,n)=>t.querySelector(`[${B}="${n}"]`),e.destroy=()=>{for(const t of S)URL.revokeObjectURL(t)},e};export{H as makeFB2};
