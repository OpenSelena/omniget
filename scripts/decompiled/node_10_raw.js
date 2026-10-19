import "../chunks/Bzak7iHL.js";
import { o as L } from "../chunks/Ba6J70tO.js";
import {
  aJ as M,
  p as N,
  g as v,
  h as R,
  f as U,
  s as g,
  t as _,
  e as i,
  i as s,
  a as $,
  b as q,
  aH as z,
  j as p,
  d as n,
  c as A,
  r as u,
  aO as O,
} from "../chunks/CMncJC9q.js";
import { i as C } from "../chunks/DzuZ9rGm.js";
import { s as K, a as Q } from "../chunks/B8R8mbJx.js";
import { invoke as B } from "../chunks/DhEqZVGG.js";
import { g as V } from "../chunks/CPghbUUZ.js";
import { t as W } from "../chunks/Byo1Zxp4.js";
import { B as c } from "../chunks/Ce_giywD.js";
import { s as x } from "../chunks/DaX8bPmL.js";
import { i as X } from "../chunks/fNK88VhS.js";
var Y = A(
  '<section class="debug-header svelte-15od1hi"><h3 class="debug-title svelte-15od1hi"> </h3> <p class="debug-desc svelte-15od1hi"> </p></section> <pre class="debug-report svelte-15od1hi"> </pre> <button type="button" class="copy-button svelte-15od1hi"><!></button>',
  1,
);
function ce(D, T) {
  N(T, !0);
  const o = () => K(W, "$t", j),
    [j, I] = Q();
  let a = v(null),
    d = v(R([])),
    l = v(!1);
  L(async () => {
    if (!X()) {
      (x("info", o()("about.debug_disabled_redirect")), await V("/about"));
      return;
    }
    const [e, t] = await Promise.all([
      B("get_debug_info").catch(() => null),
      B("check_dependencies").catch(() => []),
    ]);
    (p(a, e, !0), p(d, t, !0));
  });
  const y = z(() => {
    const e = [];
    if (
      (e.push("```"),
      e.push(`OmniGet v${c.version}`),
      e.push(`Build: ${c.commitShort} (${c.branch}) · ${c.date}`),
      s(a) &&
        (e.push(`OS: ${s(a).os} (${s(a).os_family}, ${s(a).arch})`),
        e.push(`Proxy: ${s(a).proxy_enabled ? "enabled" : "disabled"}`)),
      e.push(""),
      e.push("Dependencies:"),
      s(d).length === 0)
    )
      e.push("  (none detected)");
    else
      for (const t of s(d)) {
        const r = t.installed ? (t.version ?? "unknown") : "not installed";
        e.push(`  - ${t.name}: ${r}`);
      }
    return (
      e.push("```"),
      e.join(`
`)
    );
  });
  async function P() {
    try {
      (await navigator.clipboard.writeText(s(y)),
        p(l, !0),
        x("success", o()("about.debug.copied")),
        setTimeout(() => {
          p(l, !1);
        }, 2e3));
    } catch {
      x("error", o()("about.debug.copy_failed"));
    }
  }
  var k = Y(),
    b = U(k),
    h = n(b),
    S = n(h, !0);
  u(h);
  var w = g(h, 2),
    E = n(w, !0);
  (u(w), u(b));
  var m = g(b, 2),
    F = n(m, !0);
  u(m);
  var f = g(m, 2);
  f.__click = P;
  var G = n(f);
  {
    var H = (e) => {
        var t = O();
        (_((r) => i(t, r), [() => o()("about.debug.copied_short")]), $(e, t));
      },
      J = (e) => {
        var t = O();
        (_((r) => i(t, r), [() => o()("about.debug.copy_button")]), $(e, t));
      };
    C(G, (e) => {
      s(l) ? e(H) : e(J, !1);
    });
  }
  (u(f),
    _(
      (e, t) => {
        (i(S, e), i(E, t), i(F, s(y)));
      },
      [() => o()("about.debug.title"), () => o()("about.debug.description")],
    ),
    $(D, k),
    q(),
    I());
}
M(["click"]);
export { ce as component };
